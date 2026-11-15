//! Proxies: a client's `proxy` sends every request through the proxy it
//! names, or, as `false`, through none.
//!
//! The default, the environment's proxies, is tested through the `frost` CLI
//! in `frostlang-cli`, whose child process can be given an environment.

use frostlang::Value;

use crate::common::{Server, assert_raises, at, failure_kind, value};

#[test]
fn a_proxy_url_sends_every_request_through_the_proxy() {
    let proxy = Server::ok("proxied");
    let result = value(&format!(
        "http.client({{proxy: '{}'}}).get('http://far.invalid/page?q=1')",
        proxy.url("")
    ));
    assert_eq!(at(&result, &["response", "body"]), Value::from("proxied"));
    let request = proxy.request();
    assert_eq!(
        request.target, "http://far.invalid/page?q=1",
        "a proxy is sent the whole URL"
    );
    assert_eq!(request.header("host"), ["far.invalid"]);
}

#[test]
fn a_proxy_without_a_scheme_is_http() {
    let proxy = Server::ok("proxied");
    let port = proxy
        .authority()
        .rsplit_once(':')
        .expect("an authority has a port")
        .1
        .to_owned();
    // Parsed as a URL, `localhost:8080` has `localhost` as its scheme and no
    // host; the client must still read it as a host and port.
    for address in [proxy.authority(), format!("localhost:{port}")] {
        let result = value(&format!(
            "http.client({{proxy: '{address}'}}).get('http://far.invalid/')",
        ));
        assert_eq!(
            at(&result, &["response", "body"]),
            Value::from("proxied"),
            "through {address}: {result:?}"
        );
    }
    let targets: Vec<String> = proxy
        .requests()
        .into_iter()
        .map(|request| request.target)
        .collect();
    assert_eq!(targets, ["http://far.invalid/", "http://far.invalid/"]);
}

#[test]
fn an_https_proxy_is_accepted() {
    // Made, though never connected to.
    let result = value(
        r"
        http.client({proxy: 'https://127.0.0.1:1'})
        null
        ",
    );
    assert_eq!(result, Value::Null);
}

#[test]
fn an_https_request_tunnels_through_the_proxy() {
    // Refusing the tunnel, since the test needs only to see it asked for.
    let proxy = Server::start(|_, connection| connection.respond(403, &[], b""));
    let result = value(&format!(
        "http.client({{proxy: '{}'}}).get('https://far.invalid/page')",
        proxy.url("")
    ));
    let request = proxy.request();
    assert_eq!(
        (request.method.as_str(), request.target.as_str()),
        ("CONNECT", "far.invalid:443"),
        "the proxy is asked for a tunnel to the host"
    );
    assert_eq!(failure_kind(&result), "connect", "a refused tunnel");
}

#[test]
fn proxy_false_sends_requests_directly() {
    let server = Server::ok("direct");
    let result = value(&format!(
        "http.client({{proxy: false}}).get('{}')",
        server.url("/page")
    ));
    assert_eq!(at(&result, &["response", "body"]), Value::from("direct"));
    assert_eq!(server.request().target, "/page");
}

#[test]
fn a_proxy_that_cannot_be_used_raises() {
    let invalid = "Function ext.http.client requires argument 1 (config) to be valid";
    assert_raises(&[
        // The cause, not only reqwest's stage.
        (
            "http.client({proxy: 'http://[oops'})",
            "Function ext.http.client got an invalid proxy \"http://[oops\": \
             builder error: invalid IPv6 address",
        ),
        (
            "http.client({proxy: 'socks5://127.0.0.1:1080'})",
            "Function ext.http.client got an invalid proxy \"socks5://127.0.0.1:1080\": \
             its scheme must be http or https",
        ),
        (
            "http.client({proxy: 'ftp://proxy.test'})",
            "Function ext.http.client got an invalid proxy \"ftp://proxy.test\": \
             its scheme must be http or https",
        ),
        (
            "http.client({proxy: true})",
            &format!("{invalid}: expected String or false, got true (at `proxy`)"),
        ),
        (
            "http.client({proxy: 8080})",
            &format!("{invalid}: expected String or false, got Int (at `proxy`)"),
        ),
    ]);
}
