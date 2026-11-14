//! Requests that get a response: methods, bodies, the result's shape,
//! redirects, connection reuse, and `request_all`.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use frostlang::Value;

use crate::common::{ClosedPort, Server, assert_raises, at, failure_kind, value};

#[test]
fn each_method_function_sends_its_method() {
    let server = Server::ok("hi");
    for (function, method) in [
        ("get", "GET"),
        ("head", "HEAD"),
        ("delete", "DELETE"),
        ("post", "POST"),
        ("put", "PUT"),
        ("patch", "PATCH"),
    ] {
        let result = value(&format!("http.client().{function}('{}')", server.url("/m")));
        assert_eq!(
            at(&result, &["response", "code"]),
            Value::Int(200),
            "{function}"
        );
        let request = server.requests().pop().expect("a request was read");
        assert_eq!(request.method, method, "client.{function}");
        assert_eq!(request.target, "/m", "client.{function}");
    }
}

#[test]
fn request_sends_get_unless_given_a_method() {
    let server = Server::ok("");
    let cases = [
        ("", "GET"),
        // A standard method, in any case, is sent uppercase.
        (", method: 'post'", "POST"),
        (", method: 'Patch'", "PATCH"),
        (", method: 'options'", "OPTIONS"),
        (", method: 'trace'", "TRACE"),
        // Any other is sent exactly as written.
        (", method: 'brew'", "brew"),
        (", method: 'PROPFIND'", "PROPFIND"),
        (", method: 'MkCol'", "MkCol"),
    ];
    for (method, sent) in cases {
        let result = value(&format!(
            "http.client().request({{url: '{}'{method}}})",
            server.url("/")
        ));
        assert_eq!(at(&result, &["ok"]), Value::Bool(true), "{method}");
        assert_eq!(
            server.requests().pop().expect("read").method,
            sent,
            "{method}"
        );
    }
}

#[test]
fn a_method_that_is_not_a_token_raises() {
    for method in ["", "bad method", "a(b)", "caf\u{e9}"] {
        assert_raises(&[(
            &format!("http.client().request({{url: 'http://a.invalid', method: '{method}'}})"),
            &format!("Function client.request got an invalid method \"{method}\""),
        )]);
    }
}

#[test]
fn a_body_of_text_or_bytes_is_sent_with_any_method() {
    let server = Server::ok("");
    let cases = [
        (
            "client.post(url, {body: 'caf\u{e9}'})",
            "POST",
            "caf\u{e9}".as_bytes(),
        ),
        (
            "client.put(url, {body: x\"00ff10\"})",
            "PUT",
            &[0x00, 0xff, 0x10][..],
        ),
        (
            "client.get(url, {body: 'a GET with a body'})",
            "GET",
            b"a GET with a body",
        ),
        (
            "client.request({url, method: 'brew', body: 'tea'})",
            "brew",
            b"tea",
        ),
        ("client.post(url, {body: ''})", "POST", b""),
    ];
    for (call, method, body) in cases {
        let script = format!(
            "def url = '{}'\ndef client = http.client()\n{call}",
            server.url("/b")
        );
        assert_eq!(at(&value(&script), &["ok"]), Value::Bool(true), "{call}");
        let request = server.requests().pop().expect("read");
        assert_eq!(request.method, method, "{call}");
        assert_eq!(request.body, body, "{call}");
        // An empty body may go without a length.
        if !body.is_empty() {
            assert_eq!(
                request.header("content-length"),
                [body.len().to_string()],
                "{call}"
            );
        }
    }
    assert_raises(&[(
        "http.client().post('http://a.invalid', {body: 1})",
        "Function client.post requires argument 2 (options) to be valid: \
         expected String or Bytes, got Int (at `body`)",
    )]);
}

#[test]
fn a_response_gives_its_code_headers_body_and_url() {
    let server = Server::start(|_, connection| {
        connection.respond(201, &[("X-Id", "7")], b"created");
    });
    let url = server.url("/things?a=1");
    assert_eq!(
        value(&format!("http.client().post('{url}')")),
        value(&format!(
            "{{ok: true, response: {{code: 201, headers: {{['content-length']: '7', ['x-id']: '7'}}, body: 'created', url: '{url}'}}}}"
        ))
    );
}

#[test]
fn any_status_code_is_a_response() {
    for code in [204, 299, 304, 404, 418, 500, 503, 599] {
        let server = Server::start(move |_, connection| {
            let body: &[u8] = if code == 204 || code == 304 {
                b""
            } else {
                b"status"
            };
            connection.respond(code, &[], body);
        });
        let result = value(&format!("http.client().get('{}')", server.url("/")));
        assert_eq!(at(&result, &["ok"]), Value::Bool(true), "status {code}");
        assert_eq!(
            at(&result, &["response", "code"]),
            Value::Int(code.into()),
            "status {code}"
        );
    }
}

#[test]
fn a_body_is_text_when_utf8_and_bytes_otherwise_whatever_its_type() {
    let cases: [(&str, &[u8], Value); 5] = [
        (
            "text/plain",
            "caf\u{e9}".as_bytes(),
            Value::from("caf\u{e9}"),
        ),
        ("application/octet-stream", b"plain", Value::from("plain")),
        (
            "text/plain; charset=utf-8",
            b"\xff\xfe",
            Value::from(&[0xff_u8, 0xfe][..]),
        ),
        (
            "text/plain; charset=latin1",
            b"caf\xe9",
            Value::from(&b"caf\xe9"[..]),
        ),
        ("text/plain", b"", Value::from("")),
    ];
    for (content_type, body, expected) in cases {
        let server = Server::start(move |_, connection| {
            connection.respond(200, &[("content-type", content_type)], body);
        });
        let result = value(&format!("http.client().get('{}')", server.url("/")));
        assert_eq!(
            at(&result, &["response", "body"]),
            expected,
            "{content_type}: {body:?}"
        );
    }
}

#[test]
fn a_head_response_has_an_empty_body() {
    let server = Server::start(|_, connection| {
        connection.write(b"HTTP/1.1 200 OK\r\ncontent-length: 1000\r\n\r\n");
    });
    let result = value(&format!("http.client().head('{}')", server.url("/")));
    assert_eq!(at(&result, &["response", "body"]), Value::from(""));
    assert_eq!(
        at(&result, &["response", "headers", "content-length"]),
        Value::from("1000")
    );
}

/// A server redirecting `/hop/<n>` to `/hop/<n - 1>`, and answering `/hop/0`
/// with `landed`.
fn hopping_server() -> Server {
    Server::start(|request, connection| {
        let hops: u32 = request
            .target
            .strip_prefix("/hop/")
            .and_then(|hops| hops.parse().ok())
            .expect("a hop request");
        if hops == 0 {
            connection.respond(200, &[], b"landed");
        } else {
            let location = format!("/hop/{}", hops - 1);
            connection.respond(302, &[("location", &location)], b"");
        }
    })
}

#[test]
fn redirects_are_followed_up_to_the_clients_limit() {
    let server = hopping_server();
    let cases = [
        // Ten by default.
        ("", 10, true),
        ("", 11, false),
        ("{redirects: 2}", 2, true),
        ("{redirects: 2}", 3, false),
        ("{redirects: 1}", 1, true),
    ];
    for (config, hops, follows) in cases {
        let url = server.url(&format!("/hop/{hops}"));
        let result = value(&format!("http.client({config}).get('{url}')"));
        if follows {
            assert_eq!(
                at(&result, &["response", "body"]),
                Value::from("landed"),
                "{config} {hops}"
            );
            assert_eq!(
                at(&result, &["response", "url"]),
                Value::from(server.url("/hop/0")),
                "the final URL ({config} {hops})"
            );
        } else {
            assert_eq!(failure_kind(&result), "redirect", "{config} {hops}");
        }
    }
}

#[test]
fn with_no_redirects_a_redirect_is_the_response() {
    let server = hopping_server();
    let url = server.url("/hop/1");
    let result = value(&format!("http.client({{redirects: 0}}).get('{url}')"));
    assert_eq!(at(&result, &["response", "code"]), Value::Int(302));
    assert_eq!(
        at(&result, &["response", "headers", "location"]),
        Value::from("/hop/0")
    );
    assert_eq!(at(&result, &["response", "url"]), Value::from(url));
    assert_eq!(server.requests().len(), 1, "nothing followed");
}

#[test]
fn a_client_reuses_its_connections() {
    let server = Server::ok("again");
    let url = server.url("/");
    value(&format!(
        "def client = http.client()\n[client.get('{url}'), client.get('{url}'), client.get('{url}')]"
    ));
    assert_eq!(server.requests().len(), 3);
    assert_eq!(
        server.connections(),
        1,
        "one client's requests share a connection"
    );

    let server = Server::ok("fresh");
    let url = server.url("/");
    value(&format!(
        "[http.client().get('{url}'), http.client().get('{url}')]"
    ));
    assert_eq!(
        server.connections(),
        2,
        "each client has its own connections"
    );
}

#[test]
fn request_options_must_be_known() {
    let get_options = "expected one of `query`, `headers`, `body`, `timeout` (at a key)";
    assert_raises(&[
        (
            "http.client().get('http://a.invalid', {url: 'http://b.invalid'})",
            &format!(
                "Function client.get requires argument 2 (options) to be valid: \
                 unknown field `url`, {get_options}"
            ),
        ),
        (
            "http.client().post('http://a.invalid', {method: 'PUT'})",
            &format!(
                "Function client.post requires argument 2 (options) to be valid: \
                 unknown field `method`, {get_options}"
            ),
        ),
        (
            "http.client().request({url: 'http://a.invalid', query: {}, verbose: true})",
            "Function client.request requires argument 1 (config) to be valid: \
             unknown field `verbose`, expected one of \
             `url`, `method`, `query`, `headers`, `body`, `timeout` (at a key)",
        ),
        (
            "http.client().request({method: 'GET'})",
            "Function client.request requires argument 1 (config) to be valid: \
             missing field `url`",
        ),
        (
            "http.client().get('http://a.invalid', {timeout: -1})",
            "Function client.get requires argument 2 (options) to be valid: \
             expected Int at least 0, got -1 (at `timeout`)",
        ),
        (
            "http.client({redirects: -1})",
            "Function ext.http.client requires argument 1 (config) to be valid: \
             expected Int at least 0, got -1 (at `redirects`)",
        ),
        (
            "http.client({proxy: false})",
            "Function ext.http.client requires argument 1 (config) to be valid: \
             unknown field `proxy`, expected one of `base_url`, `headers`, `timeout`, \
             `connect_timeout`, `read_timeout`, `redirects` (at a key)",
        ),
    ]);
}

#[test]
fn every_config_value_checks_its_type() {
    let client = "Function ext.http.client requires argument 1 (config) to be valid";
    let request = "Function client.request requires argument 1 (config) to be valid";
    let get = "Function client.get requires argument 2 (options) to be valid";
    let batch =
        "Function client.request_all requires element 0 of argument 1 (configs) to be valid";
    let url = "'http://a.invalid'";
    assert_raises(&[
        (
            "http.client({timeout: 1.5})",
            &format!("{client}: expected Int, got Float (at `timeout`)"),
        ),
        (
            "http.client({connect_timeout: '10'})",
            &format!("{client}: expected Int, got String (at `connect_timeout`)"),
        ),
        (
            "http.client({read_timeout: true})",
            &format!("{client}: expected Int, got Bool (at `read_timeout`)"),
        ),
        (
            "http.client({redirects: 2.0})",
            &format!("{client}: expected Int, got Float (at `redirects`)"),
        ),
        (
            "http.client({headers: 'accept: */*'})",
            &format!("{client}: expected Map, got String (at `headers`)"),
        ),
        (
            &format!("http.client().request({{url: {url}, method: 1}})"),
            &format!("{request}: expected String, got Int (at `method`)"),
        ),
        (
            &format!("http.client().request_all([{{url: {url}, method: ['GET']}}])"),
            &format!("{batch}: expected String, got Array (at `method`)"),
        ),
        (
            &format!("http.client().get({url}, {{timeout: 1.5}})"),
            &format!("{get}: expected Int, got Float (at `timeout`)"),
        ),
        (
            &format!("http.client().get({url}, {{headers: ['accept']}})"),
            &format!("{get}: expected Map, got Array (at `headers`)"),
        ),
    ]);
}

#[test]
fn request_all_gives_each_result_in_order() {
    // Each later request is answered sooner.
    let server = Server::start(|request, connection| {
        let n: u64 = request
            .target
            .trim_start_matches('/')
            .parse()
            .expect("a number");
        std::thread::sleep(Duration::from_millis(40 - n * 10));
        connection.respond(200, &[], request.target.as_bytes());
    });
    let base = server.url("");
    let closed = ClosedPort::new();
    let closed = closed.url();
    let result = value(&format!(
        "def client = http.client({{base_url: '{base}'}})
        client.request_all([{{url: '0'}}, {{url: '1', method: 'POST'}}, {{url: '{closed}'}}, {{url: '3'}}])"
    ));
    let Value::Array(results) = result else {
        panic!("request_all should give an Array, but gave {result:?}");
    };
    let results: Vec<Value> = results.iter().cloned().collect();
    assert_eq!(results.len(), 4);
    assert_eq!(at(&results[0], &["response", "body"]), Value::from("/0"));
    assert_eq!(at(&results[1], &["response", "body"]), Value::from("/1"));
    assert_eq!(
        failure_kind(&results[2]),
        "connect",
        "a failure is a result like any other"
    );
    assert_eq!(at(&results[3], &["response", "body"]), Value::from("/3"));
    let post = server
        .requests()
        .into_iter()
        .find(|request| request.target == "/1")
        .expect("the second request was read");
    assert_eq!(post.method, "POST");
}

#[test]
fn request_all_of_nothing_gives_nothing() {
    assert_eq!(value("http.client().request_all([])"), value("[]"));
}

/// How many requests a [`gathering_server`] holds at once.
#[derive(Default)]
struct InFlight {
    now: usize,
    most: usize,
}

/// The requests a [`gathering_server`] holds, and word of each change.
#[derive(Default)]
struct Gathering {
    in_flight: Mutex<InFlight>,
    changed: Condvar,
}

/// A server holding each request until `expected` are in flight at once, or
/// two seconds have passed, and then a little longer, recording the most it
/// ever had. A client sending more than `expected` at once is caught, as its
/// extra request arrives while the others are held.
fn gathering_server(expected: usize) -> (Server, Arc<Gathering>) {
    let gathering = Arc::new(Gathering::default());
    let server = {
        let gathering = Arc::clone(&gathering);
        Server::start(move |_, connection| {
            let mut in_flight = gathering.in_flight.lock().expect("not poisoned");
            in_flight.now += 1;
            in_flight.most = in_flight.most.max(in_flight.now);
            gathering.changed.notify_all();
            let (in_flight, _) = gathering
                .changed
                .wait_timeout_while(in_flight, Duration::from_secs(2), |in_flight| {
                    in_flight.now < expected
                })
                .expect("not poisoned");
            // Held, unlocked, so that any request beyond the limit arrives
            // while these are still in flight.
            let (mut in_flight, _) = gathering
                .changed
                .wait_timeout_while(in_flight, Duration::from_millis(100), |_| true)
                .expect("not poisoned");
            in_flight.now -= 1;
            drop(in_flight);
            connection.respond(200, &[], b"");
        })
    };
    (server, gathering)
}

#[test]
fn request_all_keeps_exactly_concurrency_requests_in_flight() {
    for (options, concurrency) in [
        (", {concurrency: 1}", 1),
        (", {concurrency: 3}", 3),
        ("", 8),
    ] {
        // Two full waves: a short last wave would wait out the server's
        // gathering time.
        let (server, gathering) = gathering_server(concurrency);
        let url = server.url("/");
        let configs = vec![format!("{{url: '{url}'}}"); concurrency * 2].join(", ");
        let results = value(&format!("http.client().request_all([{configs}]{options})"));
        assert_eq!(
            value_len(&results),
            concurrency * 2,
            "every request has a result ({options})"
        );
        let most = gathering.in_flight.lock().expect("not poisoned").most;
        assert_eq!(most, concurrency, "the most in flight at once ({options})");
    }
}

/// The length of the Array `value`.
fn value_len(value: &Value) -> usize {
    match value {
        Value::Array(array) => array.len(),
        other => panic!("expected an Array, but got {other:?}"),
    }
}

#[test]
fn request_all_checks_every_request_before_sending_any() {
    let server = Server::ok("");
    let url = server.url("/");
    assert_raises(&[(
        &format!(
            "http.client().request_all([{{url: '{url}'}}, {{url: '{url}', method: 'bad method'}}])"
        ),
        "Function client.request_all got an invalid method \"bad method\"",
    )]);
    // Long enough for a request sent before the raise to be read.
    std::thread::sleep(Duration::from_millis(200));
    assert!(server.requests().is_empty(), "no request may be sent");
}

#[test]
fn request_all_options_must_be_valid() {
    let invalid = "Function client.request_all requires argument 2 (options) to be valid";
    assert_raises(&[
        (
            "http.client().request_all([], {concurrency: 0})",
            "Function client.request_all requires option `concurrency` to be at least 1",
        ),
        (
            "http.client().request_all([], {concurrency: -2})",
            &format!("{invalid}: expected Int at least 0, got -2 (at `concurrency`)"),
        ),
        (
            "http.client().request_all([], {concurrency: '4'})",
            &format!("{invalid}: expected Int, got String (at `concurrency`)"),
        ),
        (
            "http.client().request_all([], {limit: 4})",
            &format!("{invalid}: unknown field `limit`, expected `concurrency` (at a key)"),
        ),
        (
            "http.client().request_all([{url: 'http://a.invalid', retries: 1}])",
            "Function client.request_all requires element 0 of argument 1 (configs) to be valid: \
             unknown field `retries`, expected one of \
             `url`, `method`, `query`, `headers`, `body`, `timeout` (at a key)",
        ),
    ]);
}
