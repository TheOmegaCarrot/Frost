//! Requests that fail once they reach for the network: each returns
//! `{ok: false, error: {kind, message}}` rather than raising, and each `kind`
//! has a test here.
//!
//! reqwest words the messages; the tests pin only the extension's own.

use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

use frostlang::Value;
use frostlang_http::HttpConfig;

use crate::common::{
    ClosedPort, DEADLINE, Ended, Server, at, failure_kind, gzip, importer, run_with, runtime, value,
};

/// The message of the failed result `result`, which must not be empty.
#[track_caller]
fn failure_message(result: &Value) -> String {
    match at(result, &["error", "message"]) {
        Value::String(message) if !message.is_empty() => message.as_str().to_string(),
        other => panic!("a failure's message should be a non-empty String, but is {other:?}"),
    }
}

#[test]
fn a_refused_connection_is_connect() {
    let closed = ClosedPort::new();
    let result = value(&format!("http.client().get('{}')", closed.url()));
    assert_eq!(failure_kind(&result), "connect");
    assert!(
        failure_message(&result).contains("refused"),
        "the message gives the cause: {result:?}"
    );
}

/// A server that answers each request only after `delay`.
fn slow_server(delay: Duration) -> Server {
    Server::start(move |_, connection| {
        std::thread::sleep(delay);
        connection.respond(200, &[], b"late");
    })
}

#[test]
fn a_request_over_its_timeout_is_timeout() {
    let server = slow_server(Duration::from_millis(500));
    let url = server.url("/");
    let cases = [
        // The client's limit on a whole request.
        ("{timeout: 50000}", ""),
        // A request's own limit replaces the client's.
        ("{timeout: 60000000}", ", {timeout: 50000}"),
        ("", ", {timeout: 50000}"),
    ];
    for (config, options) in cases {
        let result = value(&format!("http.client({config}).get('{url}'{options})"));
        assert_eq!(failure_kind(&result), "timeout", "{config} {options}");
        failure_message(&result);
    }
    let result = value(&format!(
        "http.client({{timeout: 50000}}).get('{url}', {{timeout: 60000000}})"
    ));
    assert_eq!(
        at(&result, &["response", "body"]),
        Value::from("late"),
        "a request's longer limit replaces its client's shorter one"
    );
}

#[test]
fn a_body_trickling_past_the_timeout_is_timeout() {
    // Data keeps arriving, well within the read timeout, but the whole
    // response takes far longer than the request's limit.
    let server = Server::start(|_, connection| {
        connection.write(b"HTTP/1.1 200 OK\r\ncontent-length: 1000\r\n\r\n");
        for _ in 0..1000 {
            std::thread::sleep(Duration::from_millis(20));
            if !connection.write(b"x") {
                break;
            }
        }
    });
    let url = server.url("/");
    for (config, options) in [("{timeout: 200000}", ""), ("", ", {timeout: 200000}")] {
        let result = value(&format!("http.client({config}).get('{url}'{options})"));
        assert_eq!(failure_kind(&result), "timeout", "{config} {options}");
    }
}

#[test]
fn a_response_stalling_past_the_read_timeout_is_timeout() {
    // The headers and some of the body arrive, then nothing.
    let server = Server::start(|_, connection| {
        connection.write(b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\nsome");
        connection.wait_for_close();
    });
    let result = value(&format!(
        "http.client({{read_timeout: 50000}}).get('{}')",
        server.url("/")
    ));
    assert_eq!(failure_kind(&result), "timeout");
}

#[test]
fn a_tls_handshake_stalling_past_the_connect_timeout_is_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port is free");
    let address = listener
        .local_addr()
        .expect("a bound socket has an address");
    let (accepted_sender, accepted) = mpsc::channel();
    // Accepts the connection, but never answers the TLS hello: the
    // connection is held open, silent, until the client gives up.
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = accepted_sender.send(());
            let _ = std::io::copy(&mut stream, &mut std::io::sink());
        }
    });
    let result = value(&format!(
        "http.client({{connect_timeout: 50000}}).get('https://{address}/')"
    ));
    assert_eq!(failure_kind(&result), "timeout");
    accepted
        .recv_timeout(DEADLINE)
        .expect("the client connected before timing out");
}

#[test]
fn a_redirect_loop_is_redirect() {
    let server = Server::start(|_, connection| {
        connection.respond(301, &[("location", "/loop")], b"");
    });
    let result = value(&format!("http.client().get('{}')", server.url("/loop")));
    assert_eq!(failure_kind(&result), "redirect");
    failure_message(&result);
}

#[test]
fn a_body_cut_short_is_body_even_while_decompressing() {
    let server = Server::start(|request, connection| {
        let head: &[u8] = match request.target.as_str() {
            "/plain" => b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\n",
            _ => b"HTTP/1.1 200 OK\r\ncontent-encoding: gzip\r\ncontent-length: 100\r\n\r\n",
        };
        connection.write(head);
        connection.write(&gzip(&"a".repeat(1000))[..10]);
        connection.close();
    });
    for path in ["/plain", "/gzipped"] {
        let result = value(&format!("http.client().get('{}')", server.url(path)));
        assert_eq!(failure_kind(&result), "body", "{path}");
        failure_message(&result);
    }
}

#[test]
fn a_body_that_does_not_decompress_is_decode() {
    let server = Server::start(|_, connection| {
        connection.respond(200, &[("content-encoding", "gzip")], b"not gzip at all");
    });
    let result = value(&format!("http.client().get('{}')", server.url("/")));
    assert_eq!(failure_kind(&result), "decode");
    failure_message(&result);
}

#[test]
fn a_connection_closed_without_a_response_is_request() {
    let server = Server::start(|_, connection| connection.close());
    let result = value(&format!("http.client().get('{}')", server.url("/")));
    assert_eq!(failure_kind(&result), "request");
    failure_message(&result);
}

#[test]
fn a_response_that_is_not_http_is_request() {
    let server = Server::start(|_, connection| {
        connection.write(b"SSH-2.0-OpenSSH_9.6\r\n");
        connection.close();
    });
    let result = value(&format!("http.client().get('{}')", server.url("/")));
    assert_eq!(failure_kind(&result), "request");
}

/// The result of `call`, with `url` bound to `path` on `server`, run where
/// the host allows bodies of at most `max_body_size` bytes.
fn with_max_body_size(server: &Server, path: &str, max_body_size: u64, call: &str) -> Value {
    let config = HttpConfig::new(runtime()).with_max_body_size(max_body_size);
    let script = format!("def url = '{}'\n{call}", server.url(path));
    match run_with(&script, importer(config), None) {
        Ended::Finished(result) => result,
        other => panic!("the script should run, but ended with {other:?}"),
    }
}

#[test]
fn a_body_larger_than_the_host_allows_is_too_large() {
    let server = Server::start(|request, connection| match request.target.as_str() {
        "/declared" => connection.respond(200, &[], &[b'x'; 11]),
        "/exact" => connection.respond(200, &[], &[b'x'; 10]),
        // No declared length: found too large while reading.
        "/chunked" => {
            connection.write(b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n");
            connection.write(b"6\r\nxxxxxx\r\n6\r\nxxxxxx\r\n0\r\n\r\n");
        }
        "/gzipped" => {
            let body = gzip(&"a".repeat(1000));
            assert!(body.len() <= 100, "small on the wire: {} bytes", body.len());
            connection.respond(200, &[("content-encoding", "gzip")], &body);
        }
        other => panic!("unexpected request for {other}"),
    });
    // The gzipped body is within the maximum on the wire, and over it once
    // decompressed.
    for (path, max_body_size) in [("/declared", 10), ("/chunked", 10), ("/gzipped", 100)] {
        let result = with_max_body_size(&server, path, max_body_size, "http.client().get(url)");
        assert_eq!(failure_kind(&result), "too_large", "{path}");
        assert_eq!(
            failure_message(&result),
            format!("the response body is larger than the maximum of {max_body_size} bytes"),
            "{path}"
        );
    }
    let result = with_max_body_size(&server, "/exact", 10, "http.client().get(url)");
    assert_eq!(
        at(&result, &["response", "body"]),
        Value::from("x".repeat(10)),
        "a body of exactly the maximum is allowed"
    );
}

#[test]
fn a_decompressed_body_is_held_whole_without_a_maximum() {
    let server = Server::start(|_, connection| {
        connection.respond(
            200,
            &[("content-encoding", "gzip")],
            &gzip(&"a".repeat(1000)),
        );
    });
    let result = value(&format!("http.client().get('{}')", server.url("/")));
    assert_eq!(
        at(&result, &["response", "body"]),
        Value::from("a".repeat(1000))
    );
}

#[test]
fn a_head_response_is_never_too_large() {
    let server = Server::start(|_, connection| {
        connection.write(b"HTTP/1.1 200 OK\r\ncontent-length: 1000\r\n\r\n");
    });
    let result = with_max_body_size(&server, "/", 10, "http.client().head(url)");
    assert_eq!(at(&result, &["response", "body"]), Value::from(""));
}
