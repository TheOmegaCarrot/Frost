//! Streaming bodies to and from files: `body_file` sends a file as the
//! request's body, and `save_to` saves the response's body to a file, never
//! leaving part of one there.
//!
//! A request's files are checked before anything is sent, a batch's
//! included, but each is opened, or its partial file made, only as its
//! request is sent.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use frostlang::{AbortReason, CancelToken, Value};
use frostlang_http::HttpConfig;
use tempfile::TempDir;

use crate::common::tls::pem_literal;
use crate::common::{
    DEADLINE, Ended, Server, TestCa, assert_raises, at, failure_kind, failure_message, importer,
    raises, run_with, runtime, value,
};

/// A scratch directory, deleted with it.
fn scratch() -> TempDir {
    tempfile::tempdir().expect("a scratch directory can be made")
}

/// The names of the entries in `dir`, sorted.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("the directory can be read")
        .map(|entry| {
            entry
                .expect("an entry can be read")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// `path` as text, for a Frost String literal.
fn text(path: &Path) -> String {
    path.to_str().expect("scratch paths are UTF-8").to_string()
}

#[test]
fn save_to_saves_the_body_and_gives_the_files_path() {
    let server = Server::start(|_, connection| connection.respond(200, &[], b"\x00\xffsaved"));
    let dir = scratch();
    let path = dir.path().join("out.bin");
    let result = value(&format!(
        r"
        def result = http.client().get('{}', {{save_to: '{}'}})
        [result.response.file, keys(result.response)]
        ",
        server.url("/"),
        text(&path)
    ));
    assert_eq!(
        result,
        value(&format!(
            "['{}', ['code', 'file', 'headers', 'url']]",
            text(&path)
        )),
        "the response gives the file, not the body"
    );
    assert_eq!(fs::read(&path).expect("the file exists"), b"\x00\xffsaved");
    assert_eq!(
        entries(dir.path()),
        ["out.bin"],
        "nothing else is left behind"
    );
}

#[test]
fn save_to_replaces_a_file_and_gives_it_ordinary_permissions() {
    let server = Server::ok("new");
    let dir = scratch();
    let path = dir.path().join("out.txt");
    fs::write(&path, "old").expect("the old file is written");
    let result = value(&format!(
        "http.client().get('{}', {{save_to: '{}'}})",
        server.url("/"),
        text(&path)
    ));
    assert_eq!(at(&result, &["ok"]), Value::Bool(true));
    assert_eq!(fs::read_to_string(&path).expect("the file exists"), "new");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Under the same umask, as any file the process creates.
        let ordinary = dir.path().join("ordinary");
        fs::File::create(&ordinary).expect("a file can be created");
        let mode = |path: &Path| fs::metadata(path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode(&path), mode(&ordinary), "the saved file's permissions");
    }
}

#[test]
fn a_saved_body_has_no_maximum_size() {
    let server = Server::start(|_, connection| connection.respond(200, &[], &[b'x'; 100]));
    let dir = scratch();
    let path = dir.path().join("big");
    let script = format!(
        "http.client().get('{}', {{save_to: '{}'}})",
        server.url("/"),
        text(&path)
    );
    let config = HttpConfig::new(runtime()).with_max_body_size(10);
    let Ended::Finished(result) = run_with(&script, importer(config), None) else {
        panic!("the script should run");
    };
    assert_eq!(at(&result, &["ok"]), Value::Bool(true), "{result:?}");
    assert_eq!(fs::read(&path).expect("the file exists").len(), 100);
}

#[test]
fn a_head_request_saves_an_empty_file() {
    let server = Server::start(|_, connection| {
        connection.write(b"HTTP/1.1 200 OK\r\ncontent-length: 1000\r\n\r\n");
    });
    let dir = scratch();
    let path = dir.path().join("empty");
    value(&format!(
        "http.client().head('{}', {{save_to: '{}'}})",
        server.url("/"),
        text(&path)
    ));
    assert_eq!(fs::read(&path).expect("the file exists"), b"");
}

#[test]
fn a_failed_download_leaves_the_destination_as_it_was() {
    let server = Server::start(|_, connection| {
        connection.write(b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\nonly some");
        connection.close();
    });
    let dir = scratch();
    let path = dir.path().join("out.txt");
    fs::write(&path, "old").expect("the old file is written");
    let result = value(&format!(
        "http.client().get('{}', {{save_to: '{}'}})",
        server.url("/"),
        text(&path)
    ));
    assert_eq!(failure_kind(&result), "body");
    assert_eq!(fs::read_to_string(&path).expect("the file exists"), "old");
    assert_eq!(entries(dir.path()), ["out.txt"], "no partial file is left");
}

#[test]
fn a_destination_that_cannot_take_the_finished_body_fails_as_save() {
    // The body is held back until the test has put a directory where the
    // file should go, so that the file cannot take its place.
    let (arrived_sender, arrived) = mpsc::channel();
    let (proceed, proceeding) = mpsc::channel::<()>();
    let gate = Mutex::new((arrived_sender, proceeding));
    let server = Server::start(move |_, connection| {
        let gate = gate.lock().expect("not poisoned");
        connection.write(b"HTTP/1.1 200 OK\r\ncontent-length: 4\r\n\r\n");
        gate.0.send(()).expect("the test is listening");
        gate.1
            .recv_timeout(DEADLINE)
            .expect("the test lets the body go");
        connection.write(b"body");
    });
    let dir = scratch();
    let path = dir.path().join("out");
    let blocker = {
        let path = path.clone();
        thread::spawn(move || {
            arrived.recv_timeout(DEADLINE).expect("the request arrives");
            fs::create_dir(&path).expect("the directory is made");
            proceed.send(()).expect("the server is waiting");
        })
    };
    let result = value(&format!(
        "http.client().get('{}', {{save_to: '{}'}})",
        server.url("/"),
        text(&path)
    ));
    blocker.join().expect("the blocker finishes");
    assert_eq!(failure_kind(&result), "save", "{result:?}");
    let message = failure_message(&result);
    assert!(
        message.starts_with(&format!("could not save to \"{}\": ", text(&path))),
        "the message names the file: {message:?}"
    );
    assert_eq!(entries(dir.path()), ["out"], "no partial file is left");
}

#[test]
fn cancelling_a_download_leaves_no_file() {
    let (arrived_sender, arrived) = mpsc::channel();
    let arrived_sender = Mutex::new(arrived_sender);
    let server = Server::start(move |_, connection| {
        connection.write(b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\npart");
        let _ = arrived_sender.lock().expect("not poisoned").send(());
        connection.wait_for_close();
    });
    let dir = scratch();
    let path = dir.path().join("out");
    let token = CancelToken::new();
    let canceller = {
        let token = token.clone();
        thread::spawn(move || {
            arrived.recv_timeout(DEADLINE).expect("the request arrives");
            token.cancel();
        })
    };
    let script = format!(
        "http.client({{read_timeout: 3600000000}}).get('{}', {{save_to: '{}'}})",
        server.url("/"),
        text(&path)
    );
    let ended = run_with(&script, importer(HttpConfig::new(runtime())), Some(&token));
    canceller.join().expect("the canceller finishes");
    assert_eq!(ended, Ended::Aborted(AbortReason::Cancelled));
    // The abandoned download is dropped on the runtime's threads, after the
    // run has ended.
    let start = Instant::now();
    while !entries(dir.path()).is_empty() {
        assert!(
            start.elapsed() < DEADLINE,
            "the partial file should be deleted, but {:?} remain",
            entries(dir.path())
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_destination_that_cannot_be_made_raises_before_sending() {
    let server = Server::ok("");
    let dir = scratch();
    let file = dir.path().join("file");
    fs::write(&file, "").expect("the file is written");
    let dir_text = text(dir.path());
    let separator = std::path::MAIN_SEPARATOR;
    // Each destination, and why it cannot be saved to.
    let cases = [
        (
            text(&dir.path().join("missing").join("out")),
            "its directory does not exist",
        ),
        // A file is not a directory to save into.
        (text(&file.join("out")), "its directory does not exist"),
        (dir_text.clone(), "it is a directory"),
        (format!("{dir_text}{separator}"), "it names no file"),
        // Each would otherwise save to a file named for its directory.
        (
            format!("{dir_text}{separator}new{separator}"),
            "it names no file",
        ),
        (
            format!("{dir_text}{separator}new{separator}."),
            "it names no file",
        ),
        (
            format!("{dir_text}{separator}new{separator}.."),
            "it names no file",
        ),
        (String::new(), "it names no file"),
    ];
    for (path, reason) in cases {
        assert_eq!(
            raises(&format!(
                "http.client().get('{}', {{save_to: '{path}'}})",
                server.url("/")
            )),
            format!("Function client.get cannot save to \"{path}\": {reason}"),
            "saving to {path:?}"
        );
    }
    assert_raises(&[
        (
            &format!(
                r"http.client().get('{}', {{save_to: '{dir_text}{separator}a\u{{0}}b'}})",
                server.url("/")
            ),
            &format!(
                r#"Function client.get cannot save to "{dir_text}{separator}a\u{{0}}b": it contains a NUL character"#
            ),
        ),
        (
            "http.client().get('http://a.invalid', {save_to: 1})",
            "Function client.get requires argument 2 (options) to be valid: \
             expected String, got Int (at `save_to`)",
        ),
    ]);
    assert!(server.requests().is_empty(), "nothing may be sent");
    assert_eq!(entries(dir.path()), ["file"], "nothing may be made");
}

#[test]
fn body_file_sends_the_file_with_its_length() {
    let server = Server::ok("");
    let dir = scratch();
    let cases: [(&str, &[u8]); 2] = [("data.bin", b"\x00\xffcontent"), ("empty", b"")];
    for (name, content) in cases {
        let path = dir.path().join(name);
        fs::write(&path, content).expect("the file is written");
        let result = value(&format!(
            "http.client().post('{}', {{body_file: '{}'}})",
            server.url("/upload"),
            text(&path)
        ));
        assert_eq!(at(&result, &["ok"]), Value::Bool(true), "{name}");
        let request = server.requests().pop().expect("the request was read");
        assert_eq!(request.body, content, "{name}");
        assert_eq!(
            request.header("content-length"),
            [content.len().to_string()],
            "{name}"
        );
        assert!(
            request.header("transfer-encoding").is_empty(),
            "{name} is sent with a length, not chunked"
        );
    }
}

#[test]
fn a_request_may_send_a_file_and_save_the_response() {
    let server = Server::start(|request, connection| {
        let mut echo = b"echo: ".to_vec();
        echo.extend_from_slice(&request.body);
        connection.respond(200, &[], &echo);
    });
    let dir = scratch();
    let (upload, download) = (dir.path().join("up"), dir.path().join("down"));
    fs::write(&upload, "payload").expect("the file is written");
    let result = value(&format!(
        "http.client().put('{}', {{body_file: '{}', save_to: '{}'}})",
        server.url("/"),
        text(&upload),
        text(&download)
    ));
    assert_eq!(at(&result, &["ok"]), Value::Bool(true));
    assert_eq!(
        fs::read_to_string(&download).expect("saved"),
        "echo: payload"
    );
}

#[test]
fn a_batch_saves_each_response_to_its_own_file() {
    let server = Server::start(|request, connection| {
        connection.respond(200, &[], request.target.as_bytes());
    });
    let dir = scratch();
    let (first, second) = (dir.path().join("first"), dir.path().join("second"));
    value(&format!(
        "http.client().request_all([{{url: '{}', save_to: '{}'}}, {{url: '{}', save_to: '{}'}}])",
        server.url("/1"),
        text(&first),
        server.url("/2"),
        text(&second)
    ));
    assert_eq!(fs::read_to_string(&first).expect("saved"), "/1");
    assert_eq!(fs::read_to_string(&second).expect("saved"), "/2");
}

#[test]
fn a_body_file_that_cannot_be_read_raises_before_sending() {
    let server = Server::ok("");
    let dir = scratch();
    let missing = dir.path().join("missing");
    let message = raises(&format!(
        "http.client().post('{}', {{body_file: '{}'}})",
        server.url("/"),
        text(&missing)
    ));
    assert!(
        message.starts_with(&format!(
            "Function client.post cannot read \"{}\": ",
            text(&missing)
        )),
        "a missing file: {message}"
    );
    let file = dir.path().join("file");
    fs::write(&file, "x").expect("the file is written");
    assert_raises(&[
        (
            &format!(
                "http.client().post('{}', {{body_file: '{}'}})",
                server.url("/"),
                text(dir.path())
            ),
            &format!(
                "Function client.post cannot read \"{}\": it is a directory",
                text(dir.path())
            ),
        ),
        (
            &format!(
                "http.client().post('{}', {{body: 'x', body_file: '{}'}})",
                server.url("/"),
                text(&file)
            ),
            "Function client.post takes option `body` or `body_file`, not both",
        ),
        #[cfg(unix)]
        (
            &format!(
                "http.client().post('{}', {{body_file: '/dev/null'}})",
                server.url("/")
            ),
            "Function client.post cannot read \"/dev/null\": it is not a regular file",
        ),
        (
            "http.client().post('http://a.invalid', {body_file: ['a']})",
            "Function client.post requires argument 2 (options) to be valid: \
             expected String, got Array (at `body_file`)",
        ),
    ]);
    assert!(server.requests().is_empty(), "nothing may be sent");
}

#[test]
fn a_body_file_cut_short_while_sent_fails_as_body() {
    let dir = scratch();
    let path = big_file(&dir);
    let (result, received) = send_changing_over_http1(&path, cut_short);
    assert_cut_short(&result, received, &path);
}

#[test]
fn a_body_file_cut_short_while_sent_over_http2_fails_as_body() {
    // HTTP/2 sends a body apart from the request, and its failure never
    // reaches the request's own error.
    let dir = scratch();
    let path = big_file(&dir);
    let (result, received) = send_changing_over_http2(&path, cut_short);
    assert_cut_short(&result, received, &path);
}

#[test]
fn a_body_file_growing_while_sent_sends_its_length_when_opened() {
    // HTTP/2 does not hold a body to its `content-length`, so the client
    // must.
    type SendChanging = fn(&Path, Change) -> (Value, u64);
    let cases: [(&str, SendChanging); 2] = [
        ("HTTP/1", send_changing_over_http1),
        ("HTTP/2", send_changing_over_http2),
    ];
    for (protocol, send) in cases {
        let dir = scratch();
        let path = big_file(&dir);
        let (result, received) = send(&path, grow);
        assert_eq!(received, BIG, "the bytes sent over {protocol}");
        assert_eq!(
            at(&result, &["ok"]),
            Value::Bool(true),
            "over {protocol}: {result:?}"
        );
    }
}

/// The size of the file sent by the tests that change it partway through:
/// far more than the client buffers, so that most of it is still unread when
/// it changes.
const BIG: u64 = 64 * 1024 * 1024;

/// How much of the file arrives before it changes.
const READ_FIRST: u64 = 1024 * 1024;

/// A change made to the file at a path while it is sent.
type Change = fn(&Path);

/// The result of sending the [`BIG`] file at `path` over HTTP/1 to a server
/// that makes `change` to it once [`READ_FIRST`] bytes arrive, and how much
/// of it arrived.
fn send_changing_over_http1(path: &Path, change: Change) -> (Value, u64) {
    // A server of its own, since it must act partway through the body.
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port is free");
    let url = format!(
        "http://{}/",
        listener
            .local_addr()
            .expect("a bound socket has an address")
    );
    let server = {
        let path = path.to_owned();
        thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the request arrives");
            stream
                .set_read_timeout(Some(DEADLINE))
                .expect("a timeout can be set");
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            while line != "\r\n" {
                line.clear();
                reader.read_line(&mut line).expect("the head is read");
            }
            let mut first = vec![0; usize::try_from(READ_FIRST).expect("small")];
            reader.read_exact(&mut first).expect("the body starts");
            change(&path);
            // The rest of the body, or as much as the client sends before it
            // gives up.
            let mut received = READ_FIRST;
            let mut buffer = vec![0; 64 * 1024];
            while received < BIG {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => received += u64::try_from(read).expect("small"),
                }
            }
            if received == BIG {
                let _ = reader
                    .get_mut()
                    .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
            }
            received
        })
    };
    let result = value(&format!(
        "http.client().post('{url}', {{body_file: '{}'}})",
        text(path)
    ));
    (result, server.join().expect("the server finishes"))
}

/// The result of sending the [`BIG`] file at `path` over HTTP/2, as
/// [`send_changing_over_http1`] does.
fn send_changing_over_http2(path: &Path, change: Change) -> (Value, u64) {
    let ca = TestCa::new("http2");
    let mut tls = (*ca.server_config(&["127.0.0.1"])).clone();
    tls.alpn_protocols = vec![b"h2".to_vec()];
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port is free");
    let url = format!(
        "https://{}/",
        listener
            .local_addr()
            .expect("a bound socket has an address")
    );
    let server = {
        let path = path.to_owned();
        thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime can be made")
                .block_on(receive_over_http2(listener, Arc::new(tls), &path, change))
        })
    };
    let result = value(&format!(
        "http.client({{ca: [{}]}}).post('{url}', {{body_file: '{}'}})",
        pem_literal(&ca.pem()),
        text(path)
    ));
    (result, server.join().expect("the server finishes"))
}

/// Makes the file at `path` [`READ_FIRST`] bytes longer.
fn grow(path: &Path) {
    fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_len(BIG + READ_FIRST))
        .expect("the file grows");
}

/// A file of [`BIG`] bytes in `dir`.
fn big_file(dir: &TempDir) -> PathBuf {
    let path = dir.path().join("big");
    fs::File::create(&path)
        .and_then(|file| file.set_len(BIG))
        .expect("the file is made");
    path
}

/// Cuts the file at `path` short, to nothing.
fn cut_short(path: &Path) {
    fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_len(0))
        .expect("the file is cut short");
}

/// Assert that `result`, of sending the file at `path` that was cut short
/// while it was sent, failed for it, with `received` of its bytes arrived:
/// fewer than all, since the file is read as it is sent.
#[track_caller]
fn assert_cut_short(result: &Value, received: u64, path: &Path) {
    assert!(
        received < BIG,
        "the file is read as it is sent, but all {received} bytes arrived"
    );
    assert_eq!(failure_kind(result), "body", "{result:?}");
    let message = failure_message(result);
    assert!(
        message.starts_with(&format!("could not read \"{}\": it ended ", text(path)))
            && message.ends_with(" bytes short of its length"),
        "the message names the file and why: {message:?}"
    );
}

/// Serves one request over HTTP/2 and TLS `tls` on `listener`, making
/// `change` to the file at `path` once [`READ_FIRST`] bytes of the body
/// arrive, and gives how much of it arrived. A body that arrives whole, by
/// its length, is answered `200`.
async fn receive_over_http2(
    listener: TcpListener,
    tls: Arc<rustls::ServerConfig>,
    path: &Path,
    change: Change,
) -> u64 {
    listener
        .set_nonblocking(true)
        .expect("the listener can be made async");
    let listener = tokio::net::TcpListener::from_std(listener).expect("the listener is async");
    let (stream, _) = listener.accept().await.expect("the request arrives");
    let stream = tokio_rustls::TlsAcceptor::from(tls)
        .accept(stream)
        .await
        .expect("TLS is negotiated");
    // Windows larger than the defaults, which would make a whole file slow to
    // send, yet far smaller than the file.
    let mut connection = h2::server::Builder::new()
        .initial_window_size(4 * 1024 * 1024)
        .initial_connection_window_size(4 * 1024 * 1024)
        .handshake::<_, &[u8]>(stream)
        .await
        .expect("HTTP/2 is negotiated");
    let (request, mut respond) = connection
        .accept()
        .await
        .expect("a request arrives")
        .expect("the request is valid");
    // The connection carries the body and response only while it is driven,
    // until the client closes it.
    let driving = tokio::spawn(async move { while let Some(Ok(_)) = connection.accept().await {} });
    let mut body = request.into_body();
    let mut received = 0;
    loop {
        match body.data().await {
            Some(Ok(chunk)) => {
                let _ = body.flow_control().release_capacity(chunk.len());
                let before = received;
                received += u64::try_from(chunk.len()).expect("small");
                if before < READ_FIRST && received >= READ_FIRST {
                    change(path);
                }
            }
            // The client gave up on the body, or broke its length, which the
            // server checks.
            Some(Err(_)) => break,
            None => {
                let _ = respond.send_response(http::Response::new(()), true);
                break;
            }
        }
    }
    let _ = driving.await;
    received
}

#[test]
fn a_batch_opens_each_body_file_only_as_its_request_is_sent() {
    let (message, second) = send_second_after(|second| {
        fs::remove_file(second).expect("the second file is deleted");
    });
    assert!(
        message.starts_with(&format!("could not read \"{second}\": ")),
        "the message names the file: {message:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_body_file_without_read_permission_raises_before_a_batch_sends_any() {
    use std::os::unix::fs::PermissionsExt;
    let server = Server::ok("");
    let dir = scratch();
    let (readable, unreadable) = (dir.path().join("readable"), dir.path().join("unreadable"));
    fs::write(&readable, "x").expect("the file is written");
    fs::write(&unreadable, "x").expect("the file is written");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o200))
        .expect("the permissions can be set");
    if fs::File::open(&unreadable).is_ok() {
        // Permissions do not bind a privileged user, so there is nothing to
        // test.
        return;
    }
    let message = raises(&format!(
        "http.client().request_all([
            {{url: '{}', method: 'POST', body_file: '{}'}},
            {{url: '{}', method: 'POST', body_file: '{}'}},
        ])",
        server.url("/1"),
        text(&readable),
        server.url("/2"),
        text(&unreadable)
    ));
    assert!(
        message.starts_with(&format!(
            "Function client.request_all cannot read \"{}\": ",
            text(&unreadable)
        )),
        "the unreadable file is named: {message}"
    );
    assert!(server.requests().is_empty(), "nothing may be sent");
}

/// Makes a FIFO at `path`, which std cannot.
#[cfg(unix)]
fn make_fifo(path: &Path) {
    let status = std::process::Command::new("mkfifo")
        .arg(path)
        .status()
        .expect("`mkfifo` runs");
    assert!(status.success(), "`mkfifo` makes {path:?}");
}

#[cfg(unix)]
#[test]
fn a_fifo_as_body_file_raises_without_waiting_for_a_writer() {
    // Opening a FIFO to read waits for a writer, which never comes.
    let server = Server::ok("");
    let dir = scratch();
    let fifo = dir.path().join("fifo");
    make_fifo(&fifo);
    assert_raises(&[(
        &format!(
            "http.client().post('{}', {{body_file: '{}'}})",
            server.url("/"),
            text(&fifo)
        ),
        &format!(
            "Function client.post cannot read \"{}\": it is not a regular file",
            text(&fifo)
        ),
    )]);
    assert!(server.requests().is_empty(), "nothing may be sent");
}

#[cfg(unix)]
#[test]
fn a_body_file_replaced_by_a_fifo_before_it_is_sent_fails_without_waiting() {
    let (message, second) = send_second_after(|second| {
        fs::remove_file(second).expect("the second file is deleted");
        make_fifo(second);
    });
    assert_eq!(
        message,
        format!("could not read \"{second}\": it is not a regular file")
    );
}

#[test]
fn a_body_file_replaced_before_it_is_sent_is_checked_again() {
    let (message, second) = send_second_after(|second| {
        fs::remove_file(second).expect("the second file is deleted");
        fs::create_dir(second).expect("a directory takes its place");
    });
    assert_eq!(
        message,
        format!("could not read \"{second}\": it is a directory")
    );
}

/// The failure message of the second of a batch's two `body_file` requests,
/// sent one at a time, when answering the first does `change` to the second's
/// file, and that file's path; checking that the second failed as `'body'`
/// and was not sent.
fn send_second_after(change: impl Fn(&Path) + Send + Sync + 'static) -> (String, String) {
    let dir = scratch();
    let (first, second) = (dir.path().join("first"), dir.path().join("second"));
    fs::write(&first, "one").expect("the file is written");
    fs::write(&second, "two").expect("the file is written");
    let server = {
        let second = second.clone();
        Server::start(move |request, connection| {
            if request.target == "/1" {
                change(&second);
            }
            connection.respond(200, &[], b"");
        })
    };
    let result = value(&format!(
        "http.client().request_all([
            {{url: '{}', method: 'POST', body_file: '{}'}},
            {{url: '{}', method: 'POST', body_file: '{}'}},
        ], {{concurrency: 1}})",
        server.url("/1"),
        text(&first),
        server.url("/2"),
        text(&second)
    ));
    let Value::Array(results) = result else {
        panic!("a batch gives an Array, but gave {result:?}");
    };
    assert_eq!(at(&results[0], &["ok"]), Value::Bool(true), "{results:?}");
    assert_eq!(failure_kind(&results[1]), "body");
    let targets: Vec<String> = server
        .requests()
        .into_iter()
        .map(|request| request.target)
        .collect();
    assert_eq!(targets, ["/1"], "a request without its body is not sent");
    (failure_message(&results[1]), text(&second))
}

#[test]
fn a_batch_makes_each_partial_file_only_as_its_request_is_sent() {
    let dir = scratch();
    let (first, later) = (dir.path().join("first"), dir.path().join("later"));
    fs::create_dir(&later).expect("the directory is made");
    // Answering the first request deletes the directory the second saves
    // into.
    let server = {
        let later = later.clone();
        Server::start(move |request, connection| {
            if request.target == "/1" {
                fs::remove_dir(&later).expect("the directory is deleted");
            }
            connection.respond(200, &[], b"body");
        })
    };
    let destination = later.join("second");
    let result = value(&format!(
        "http.client().request_all([
            {{url: '{}', save_to: '{}'}},
            {{url: '{}', save_to: '{}'}},
        ], {{concurrency: 1}})",
        server.url("/1"),
        text(&first),
        server.url("/2"),
        text(&destination)
    ));
    let Value::Array(results) = result else {
        panic!("a batch gives an Array, but gave {result:?}");
    };
    assert_eq!(at(&results[0], &["ok"]), Value::Bool(true), "{results:?}");
    assert_eq!(failure_kind(&results[1]), "save");
    let message = failure_message(&results[1]);
    assert!(
        message.starts_with(&format!("could not save to \"{}\": ", text(&destination))),
        "the message names the file: {message:?}"
    );
    let targets: Vec<String> = server
        .requests()
        .into_iter()
        .map(|request| request.target)
        .collect();
    assert_eq!(
        targets,
        ["/1"],
        "a request with nowhere to save is not sent"
    );
}

#[test]
fn a_redirect_that_resends_the_body_is_not_followed_from_a_body_file() {
    let server = Server::start(|request, connection| match request.target.as_str() {
        "/moved" => connection.respond(200, &[], b"arrived"),
        code => {
            let code = code[1..].parse().expect("a status code");
            connection.respond(code, &[("location", "/moved")], b"");
        }
    });
    let dir = scratch();
    let path = dir.path().join("upload");
    fs::write(&path, "payload").expect("the file is written");
    // A redirect keeping the method keeps the body, which a file cannot send
    // twice: the redirect is the response.
    for (method, code) in [
        ("POST", 307),
        ("POST", 308),
        ("PUT", 301),
        ("PUT", 302),
        ("PATCH", 307),
    ] {
        let result = value(&format!(
            "http.client().request({{url: '{}', method: '{method}', body_file: '{}'}})",
            server.url(&format!("/{code}")),
            text(&path)
        ));
        assert_eq!(
            at(&result, &["response", "code"]),
            Value::Int(code),
            "{method} answered {code}: {result:?}"
        );
    }
    // The same redirects are followed with a `body`, which can be sent again.
    let result = value(&format!(
        "http.client().request({{url: '{}', method: 'POST', body: 'payload'}})",
        server.url("/307")
    ));
    assert_eq!(at(&result, &["response", "body"]), Value::from("arrived"));
    // A redirect that turns the request into a GET, dropping the body, is
    // followed.
    for (method, code) in [("POST", 301), ("POST", 302), ("PUT", 303)] {
        let result = value(&format!(
            "http.client().request({{url: '{}', method: '{method}', body_file: '{}'}})",
            server.url(&format!("/{code}")),
            text(&path)
        ));
        assert_eq!(
            at(&result, &["response", "body"]),
            Value::from("arrived"),
            "{method} answered {code}: {result:?}"
        );
    }
}
