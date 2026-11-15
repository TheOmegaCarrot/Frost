//! An HTTP/1.1 server for the tests, on the loopback interface, written on
//! plain sockets so that a test controls every byte it sends, and when.
//!
//! Each test gives a handler, called for each request with the request and
//! the connection to answer on. The server records every request it reads.
//! It serves plain HTTP, or HTTPS with a certificate from [`super::tls`].

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// A request as the server read it.
#[derive(Clone, Debug)]
pub(crate) struct Recorded {
    pub(crate) method: String,
    /// The request target, as sent: the path and query.
    pub(crate) target: String,
    /// Each header line's name, lowercase, and value, in the order sent.
    pub(crate) headers: Vec<(String, Vec<u8>)>,
    pub(crate) body: Vec<u8>,
}

impl Recorded {
    /// The values of every header named `name`, in any case, in the order
    /// sent, as text.
    pub(crate) fn header(&self, name: &str) -> Vec<String> {
        self.headers
            .iter()
            .filter(|(named, _)| named.eq_ignore_ascii_case(name))
            .map(|(_, value)| String::from_utf8_lossy(value).into_owned())
            .collect()
    }
}

/// A connection's byte stream: plain, or over TLS.
trait Stream: Read + Write + Send {}

impl<T: Read + Write + Send> Stream for T {}

/// A connection's stream, buffered for reading requests.
type Reader = BufReader<Box<dyn Stream>>;

/// One request's connection, for a handler to answer on.
pub(crate) struct Connection<'a> {
    reader: &'a mut Reader,
    close: bool,
}

impl Connection<'_> {
    /// Sends a response with `code`, `headers`, and `body`, with a
    /// `content-length` header added.
    pub(crate) fn respond(&mut self, code: u16, headers: &[(&str, &str)], body: &[u8]) {
        let mut head = format!("HTTP/1.1 {code} Test\r\ncontent-length: {}\r\n", body.len());
        for (name, value) in headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        self.write(head.as_bytes());
        self.write(body);
    }

    /// Sends `bytes` as they are, returning whether they could be: the
    /// client may have gone, which some tests intend.
    pub(crate) fn write(&mut self, bytes: &[u8]) -> bool {
        let stream = self.reader.get_mut();
        stream
            .write_all(bytes)
            .and_then(|()| stream.flush())
            .is_ok()
    }

    /// Closes the connection once the handler returns.
    pub(crate) fn close(&mut self) {
        self.close = true;
    }

    /// Waits until the client closes the connection, then closes it too.
    pub(crate) fn wait_for_close(&mut self) {
        let mut rest = Vec::new();
        let _ = self.reader.read_to_end(&mut rest);
        self.close = true;
    }
}

/// What a test's server does with each request.
type Handler = dyn Fn(&Recorded, &mut Connection<'_>) + Send + Sync;

/// A running test server. Its threads outlive it, idle, until the tests end.
pub(crate) struct Server {
    /// `http` or `https`.
    scheme: &'static str,
    address: SocketAddr,
    log: Arc<Mutex<Vec<Recorded>>>,
    connections: Arc<AtomicUsize>,
}

impl Server {
    /// A server answering each request with `handler`.
    pub(crate) fn start(
        handler: impl Fn(&Recorded, &mut Connection<'_>) + Send + Sync + 'static,
    ) -> Self {
        Self::serving(None, handler)
    }

    /// A server answering each request with `handler`, over TLS with the
    /// certificate `tls` gives.
    pub(crate) fn start_tls(
        tls: Arc<rustls::ServerConfig>,
        handler: impl Fn(&Recorded, &mut Connection<'_>) + Send + Sync + 'static,
    ) -> Self {
        Self::serving(Some(tls), handler)
    }

    fn serving(
        tls: Option<Arc<rustls::ServerConfig>>,
        handler: impl Fn(&Recorded, &mut Connection<'_>) + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the test server binds a port");
        let address = listener
            .local_addr()
            .expect("a bound socket has an address");
        let scheme = if tls.is_some() { "https" } else { "http" };
        let log = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let handler: Arc<Handler> = Arc::new(handler);
        {
            let log = Arc::clone(&log);
            let connections = Arc::clone(&connections);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    connections.fetch_add(1, Ordering::SeqCst);
                    let stream: Box<dyn Stream> = match &tls {
                        Some(tls) => {
                            let session = rustls::ServerConnection::new(Arc::clone(tls))
                                .expect("a TLS session starts");
                            Box::new(rustls::StreamOwned::new(session, stream))
                        }
                        None => Box::new(stream),
                    };
                    let log = Arc::clone(&log);
                    let handler = Arc::clone(&handler);
                    thread::spawn(move || serve(stream, &log, &*handler));
                }
            });
        }
        Self {
            scheme,
            address,
            log,
            connections,
        }
    }

    /// How many connections the server has accepted.
    pub(crate) fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// A server answering every request `200` with `body`.
    pub(crate) fn ok(body: &'static str) -> Self {
        Self::start(move |_, connection| connection.respond(200, &[], body.as_bytes()))
    }

    /// The URL of `path` on this server.
    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}://{}{path}", self.scheme, self.address)
    }

    /// The server's address, as `host:port`.
    pub(crate) fn authority(&self) -> String {
        self.address.to_string()
    }

    /// Every request read so far, in the order read.
    pub(crate) fn requests(&self) -> Vec<Recorded> {
        self.log.lock().expect("not poisoned").clone()
    }

    /// The only request read so far.
    #[track_caller]
    pub(crate) fn request(&self) -> Recorded {
        let requests = self.requests();
        match <[Recorded; 1]>::try_from(requests) {
            Ok([request]) => request,
            Err(requests) => panic!("expected one request, but read {requests:#?}"),
        }
    }
}

/// Answers the requests on `stream` with `handler`, until either side closes
/// it.
fn serve(stream: Box<dyn Stream>, log: &Mutex<Vec<Recorded>>, handler: &Handler) {
    let mut reader = BufReader::new(stream);
    while let Some(request) = read_request(&mut reader) {
        log.lock().expect("not poisoned").push(request.clone());
        let mut connection = Connection {
            reader: &mut reader,
            close: false,
        };
        handler(&request, &mut connection);
        if connection.close {
            break;
        }
    }
}

/// The next request on `reader`, or `None` once the connection closes.
fn read_request(reader: &mut Reader) -> Option<Recorded> {
    let request_line = read_line(reader)?;
    let mut parts = request_line.splitn(3, ' ');
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    let mut headers = Vec::new();
    loop {
        let line = read_line(reader)?;
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        headers.push((name.to_ascii_lowercase(), value.trim().as_bytes().to_vec()));
    }
    let header = |name: &str| {
        headers
            .iter()
            .find(|(named, _)| named == name)
            .map(|(_, value)| String::from_utf8_lossy(value).into_owned())
    };
    let body = if let Some(length) = header("content-length") {
        let mut body = vec![0; length.parse().ok()?];
        reader.read_exact(&mut body).ok()?;
        body
    } else if header("transfer-encoding").is_some_and(|coding| coding == "chunked") {
        read_chunked(reader)?
    } else {
        Vec::new()
    };
    Some(Recorded {
        method,
        target,
        headers,
        body,
    })
}

/// A chunked body from `reader`.
fn read_chunked(reader: &mut Reader) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let size = usize::from_str_radix(read_line(reader)?.split(';').next()?.trim(), 16).ok()?;
        if size == 0 {
            // Trailers, then the blank line.
            while !read_line(reader)?.is_empty() {}
            return Some(body);
        }
        let start = body.len();
        body.resize(start + size, 0);
        reader.read_exact(&mut body[start..]).ok()?;
        read_line(reader)?;
    }
}

/// A line from `reader` without its line ending, or `None` at the end of the
/// stream.
fn read_line(reader: &mut Reader) -> Option<String> {
    let mut line = Vec::new();
    match reader.read_until(b'\n', &mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => {
            while line
                .last()
                .is_some_and(|byte| matches!(byte, b'\r' | b'\n'))
            {
                line.pop();
            }
            // Header values may hold any byte; the tests read them lossily.
            Some(String::from_utf8_lossy(&line).into_owned())
        }
    }
}
