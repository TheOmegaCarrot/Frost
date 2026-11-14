//! URLs: `parse_url` and `build_url`, and how a request's URL is resolved,
//! joined onto its client's `base_url` if relative.

use frostlang::Value;

use crate::common::{Server, assert_raises, at, value};

#[test]
fn parse_url_gives_the_parts_a_request_uses() {
    let cases = [
        (
            "https://a.example:8443/v1/users?page=2&q=x",
            "{host: 'a.example', path: '/v1/users', tls: true, port: 8443, query: 'page=2&q=x'}",
        ),
        (
            "http://a.example/",
            "{host: 'a.example', path: '/', tls: false}",
        ),
        // No path is the root.
        (
            "https://a.example",
            "{host: 'a.example', path: '/', tls: true}",
        ),
        // A scheme's default port is no port.
        (
            "http://a.example:80/x",
            "{host: 'a.example', path: '/x', tls: false}",
        ),
        (
            "https://a.example:443/x",
            "{host: 'a.example', path: '/x', tls: true}",
        ),
        (
            "http://a.example:443/x",
            "{host: 'a.example', path: '/x', tls: false, port: 443}",
        ),
        // An empty query is still a query.
        (
            "https://a.example/x?",
            "{host: 'a.example', path: '/x', tls: true, query: ''}",
        ),
        // Credentials and a fragment are kept, percent-encoded as written.
        (
            "https://user:secret@a.example/x#part",
            "{host: 'a.example', path: '/x', tls: true, user: 'user', password: 'secret', fragment: 'part'}",
        ),
        (
            "http://a.example/x?q#a%20b c",
            "{host: 'a.example', path: '/x', tls: false, query: 'q', fragment: 'a%20b%20c'}",
        ),
        (
            "http://a.example/#",
            "{host: 'a.example', path: '/', tls: false, fragment: ''}",
        ),
        (
            "http://user@a.example/",
            "{host: 'a.example', path: '/', tls: false, user: 'user'}",
        ),
        (
            "http://:secret@a.example/",
            "{host: 'a.example', path: '/', tls: false, password: 'secret'}",
        ),
        (
            "http://a%40b:p%3Aw@a.example/",
            "{host: 'a.example', path: '/', tls: false, user: 'a%40b', password: 'p%3Aw'}",
        ),
        (
            "http://user:@a.example/",
            "{host: 'a.example', path: '/', tls: false, user: 'user'}",
        ),
        // Normalized as the URL standard does.
        (
            "HTTPS://A.Example/a/./b/../c",
            "{host: 'a.example', path: '/a/c', tls: true}",
        ),
        (
            "http://a.example/a b?c d",
            "{host: 'a.example', path: '/a%20b', tls: false, query: 'c%20d'}",
        ),
        (
            "http://[::1]:8080/",
            "{host: '[::1]', path: '/', tls: false, port: 8080}",
        ),
        (
            "http://127.0.0.1/",
            "{host: '127.0.0.1', path: '/', tls: false}",
        ),
    ];
    for (url, expected) in cases {
        assert_eq!(
            value(&format!("http.parse_url('{url}')")),
            value(expected),
            "parse_url({url:?})"
        );
    }
}

#[test]
fn parse_url_gives_null_for_anything_but_an_http_or_https_url() {
    for url in [
        "",
        "not a url",
        "/relative/path",
        "a.example/x",
        "ftp://a.example/x",
        "mailto:someone@a.example",
        "file:///etc/hosts",
        "http://",
        "https://exa mple.com/",
    ] {
        assert_eq!(
            value(&format!("http.parse_url('{url}')")),
            Value::Null,
            "parse_url({url:?})"
        );
    }
}

#[test]
fn build_url_gives_the_text_of_a_url_map() {
    let cases = [
        (
            "{host: 'a.example', path: '/v1/users', tls: true, port: 8443, query: 'page=2'}",
            "https://a.example:8443/v1/users?page=2",
        ),
        (
            "{host: 'a.example', path: '/', tls: false}",
            "http://a.example/",
        ),
        (
            "{host: 'a.example', path: '', tls: true}",
            "https://a.example/",
        ),
        // The path gains its leading `/`, and is percent-encoded as needed.
        (
            "{host: 'a.example', path: 'x y', tls: true}",
            "https://a.example/x%20y",
        ),
        // A scheme's default port is dropped.
        (
            "{host: 'a.example', path: '/', tls: true, port: 443}",
            "https://a.example/",
        ),
        (
            "{host: 'A.EXAMPLE', path: '/', tls: true}",
            "https://a.example/",
        ),
        (
            "{host: '[::1]', path: '/', tls: false, port: 1}",
            "http://[::1]:1/",
        ),
        (
            "{host: 'a.example', path: '/', tls: true, query: ''}",
            "https://a.example/?",
        ),
        (
            "{host: 'a.example', path: '/', tls: true, user: 'ada', password: 'pw'}",
            "https://ada:pw@a.example/",
        ),
        (
            "{host: 'a.example', path: '/', tls: true, user: 'ada'}",
            "https://ada@a.example/",
        ),
        (
            "{host: 'a.example', path: '/', tls: true, password: 'pw'}",
            "https://:pw@a.example/",
        ),
        // Credentials are percent-encoded as needed; encoded text is kept.
        (
            "{host: 'a.example', path: '/', tls: true, user: 'a@b', password: 'p:w %41'}",
            "https://a%40b:p%3Aw%20%41@a.example/",
        ),
        (
            "{host: 'a.example', path: '/', tls: true, query: 'q', fragment: 'top of page'}",
            "https://a.example/?q#top%20of%20page",
        ),
        (
            "{host: 'a.example', path: '/', tls: true, fragment: ''}",
            "https://a.example/#",
        ),
    ];
    for (map, expected) in cases {
        assert_eq!(
            value(&format!("http.build_url({map})")),
            Value::from(expected),
            "build_url({map})"
        );
    }
}

#[test]
fn build_url_inverts_parse_url() {
    for url in [
        "https://a.example:8443/v1/users?page=2&q=x",
        "http://a.example/",
        "https://a.example/x?",
        "http://[::1]:8080/a%20b",
        "https://ada:pw@a.example/",
        "http://a%40b:p%3Aw@a.example/",
        "http://:pw@a.example/",
        "https://ada:pw@a.example:8443/a%20b?q=1#part%20two",
        "http://a.example/#",
    ] {
        assert_eq!(
            value(&format!("http.build_url(http.parse_url('{url}'))")),
            Value::from(url),
            "build_url(parse_url({url:?}))"
        );
    }
}

#[test]
fn build_url_raises_for_a_map_it_cannot_build() {
    let invalid = "Function ext.http.build_url requires argument 1 (parts) to be valid";
    assert_raises(&[
        (
            "http.build_url({host: '', path: '/', tls: true})",
            r#"Function ext.http.build_url got an invalid host "": empty host"#,
        ),
        (
            "http.build_url({host: 'a/b', path: '/', tls: true})",
            r#"Function ext.http.build_url got an invalid host "a/b": invalid international domain name"#,
        ),
        // A colon is neither cut off nor taken for a port.
        (
            "http.build_url({host: 'localhost:8080', path: '/', tls: false})",
            "Function ext.http.build_url got an invalid host \"localhost:8080\": \
             a port goes in `port`, and an IPv6 address in brackets",
        ),
        (
            "http.build_url({host: '2001:db8::1', path: '/', tls: false})",
            "Function ext.http.build_url got an invalid host \"2001:db8::1\": \
             a port goes in `port`, and an IPv6 address in brackets",
        ),
        (
            "http.client().get({host: 'localhost:8080', path: '/', tls: false})",
            "Function client.get got an invalid host \"localhost:8080\": \
             a port goes in `port`, and an IPv6 address in brackets",
        ),
        // An optional part is omitted, not Null.
        (
            "http.build_url({host: 'a.example', path: '/', tls: true, port: null})",
            &format!("{invalid}: expected Int, got Null (at `port`)"),
        ),
        (
            "http.build_url({host: 'a.example', tls: true})",
            &format!("{invalid}: missing field `path`"),
        ),
        (
            "http.build_url({path: '/', tls: true})",
            &format!("{invalid}: missing field `host`"),
        ),
        (
            "http.build_url({host: 'a.example', path: '/'})",
            &format!("{invalid}: missing field `tls`"),
        ),
        (
            "http.build_url({host: 'a.example', path: '/', tls: true, scheme: 'ftp'})",
            &format!(
                "{invalid}: unknown field `scheme`, expected one of \
                 `host`, `path`, `tls`, `port`, `query`, `user`, `password`, `fragment` (at a key)"
            ),
        ),
        (
            "http.build_url({host: 'a.example', path: '/', tls: 'yes'})",
            &format!("{invalid}: expected Bool, got String (at `tls`)"),
        ),
        (
            "http.build_url({host: 1, path: '/', tls: true})",
            &format!("{invalid}: expected String, got Int (at `host`)"),
        ),
        (
            "http.build_url({host: 'a.example', path: ['/'], tls: true})",
            &format!("{invalid}: expected String, got Array (at `path`)"),
        ),
        (
            "http.build_url({host: 'a.example', path: '/', tls: true, user: 1})",
            &format!("{invalid}: expected String, got Int (at `user`)"),
        ),
        (
            r#"http.build_url({host: 'a.example', path: '/', tls: true, password: x"00"})"#,
            &format!("{invalid}: expected String, got Bytes (at `password`)"),
        ),
        (
            "http.build_url({host: 'a.example', path: '/', tls: true, fragment: false})",
            &format!("{invalid}: expected String, got Bool (at `fragment`)"),
        ),
        (
            "http.build_url({host: 'a.example', path: '/', tls: true, port: 65536})",
            &format!("{invalid}: expected Int from 0 to 65535, got 65536 (at `port`)"),
        ),
        (
            "http.build_url({host: 'a.example', path: '/', tls: true, query: {a: 1}})",
            &format!("{invalid}: expected String, got Map (at `query`)"),
        ),
    ]);
}

/// The request target `server` saw for a GET of `url` by a client with
/// `base_url`, if any.
fn target(server: &Server, base_url: Option<&str>, url: &str) -> String {
    let config = base_url.map_or_else(String::new, |base| format!("{{base_url: '{base}'}}"));
    let result = value(&format!("http.client({config}).get('{url}')"));
    assert_eq!(
        at(&result, &["ok"]),
        Value::Bool(true),
        "the request succeeds: {result:?}"
    );
    server
        .requests()
        .pop()
        .expect("the request was read")
        .target
}

#[test]
fn a_relative_url_joins_its_clients_base_url_as_a_directory() {
    let server = Server::ok("joined");
    let root = server.url("");
    let cases = [
        // The base gains a trailing `/`, and the relative URL loses any
        // leading one.
        ("/v1", "users", "/v1/users"),
        ("/v1/", "users", "/v1/users"),
        ("/v1", "/users", "/v1/users"),
        ("/v1/", "/users", "/v1/users"),
        ("/v1", "//users", "/v1/users"),
        ("/v1/api", "users/7", "/v1/api/users/7"),
        ("", "users", "/users"),
        ("/", "/users", "/users"),
        ("/v1", "users?page=2", "/v1/users?page=2"),
        ("/v1", "", "/v1/"),
        // Dot segments resolve as in any URL.
        ("/v1", "../users", "/users"),
    ];
    for (base_path, url, expected) in cases {
        let base = format!("{root}{base_path}");
        assert_eq!(
            target(&server, Some(&base), url),
            expected,
            "{url:?} joining {base:?}"
        );
    }
}

#[test]
fn an_absolute_url_ignores_its_clients_base_url() {
    let base = Server::ok("base");
    let other = Server::ok("other");
    let result = value(&format!(
        "http.client({{base_url: '{}'}}).get('{}')",
        base.url("/v1"),
        other.url("/elsewhere")
    ));
    assert_eq!(at(&result, &["response", "body"]), Value::from("other"));
    assert_eq!(other.request().target, "/elsewhere");
    assert!(
        base.requests().is_empty(),
        "the base's server must see no request"
    );
}

#[test]
fn a_url_map_names_a_requests_url() {
    let server = Server::ok("mapped");
    let authority = server.authority();
    let (host, port) = authority.rsplit_once(':').expect("an address has a port");
    let result = value(&format!(
        "http.client({{base_url: 'http://unused.invalid'}}).get({{host: '{host}', port: {port}, path: '/m', tls: false, query: 'a=1'}})"
    ));
    assert_eq!(at(&result, &["response", "body"]), Value::from("mapped"));
    assert_eq!(server.request().target, "/m?a=1");
}

#[test]
fn a_fragment_is_never_sent() {
    let server = Server::ok("");
    let authority = server.authority();
    let (host, port) = authority.rsplit_once(':').expect("an address has a port");
    for url in [
        format!("'{}'", server.url("/page?q=1#section")),
        format!(
            "{{host: '{host}', port: {port}, path: '/page', tls: false, query: 'q=1', fragment: 'section'}}"
        ),
    ] {
        let result = value(&format!("http.client().get({url})"));
        assert_eq!(at(&result, &["ok"]), Value::Bool(true), "{url}");
        let target = server
            .requests()
            .pop()
            .expect("the request was read")
            .target;
        assert_eq!(target, "/page?q=1", "{url}");
    }
}

#[test]
fn a_url_too_long_to_send_raises_before_sending() {
    let server = Server::ok("");
    assert_raises(&[(
        &format!(
            "http.client().get('{}', {{query: {{q: tile('x', 70000)}}}})",
            server.url("/")
        ),
        "Function client.get cannot send its URL: uri too long",
    )]);
    assert!(server.requests().is_empty(), "nothing may be sent");
}

#[test]
fn a_url_that_cannot_be_requested_raises() {
    assert_raises(&[
        (
            "http.client().get('/users')",
            r#"Function client.get got the relative URL "/users", but its client has no base_url"#,
        ),
        (
            "http.client().get('')",
            r#"Function client.get got the relative URL "", but its client has no base_url"#,
        ),
        (
            "http.client().get('http://[oops/')",
            r#"Function client.get got an invalid URL "http://[oops/": invalid IPv6 address"#,
        ),
        (
            "http.client().get('ftp://a.example/x')",
            r#"Function client.get requires an http or https URL, but got "ftp://a.example/x""#,
        ),
        (
            "http.client().get('localhost:8080/x')",
            r#"Function client.get requires an http or https URL, but got "localhost:8080/x""#,
        ),
        (
            "http.client({base_url: 'http://a.example'}).get('http://')",
            r#"Function client.get got an invalid URL "http://": empty host"#,
        ),
        (
            "http.client().request({url: {host: '', path: '/', tls: true}})",
            r#"Function client.request got an invalid host "": empty host"#,
        ),
    ]);
}

#[test]
fn a_base_url_that_cannot_be_joined_onto_raises_on_making_the_client() {
    assert_raises(&[
        (
            "http.client({base_url: '/v1'})",
            r#"Function ext.http.client got an invalid URL "/v1": relative URL without a base"#,
        ),
        (
            "http.client({base_url: 'ftp://a.example'})",
            r#"Function ext.http.client requires an http or https URL, but got "ftp://a.example""#,
        ),
        (
            "http.client({base_url: 'https://a.example/v1?key=1'})",
            "Function ext.http.client requires a base_url without a query or fragment, \
             but got \"https://a.example/v1?key=1\"",
        ),
        (
            "http.client({base_url: 'https://a.example/v1#top'})",
            "Function ext.http.client requires a base_url without a query or fragment, \
             but got \"https://a.example/v1#top\"",
        ),
        (
            "http.client({base_url: 1})",
            "Function ext.http.client requires argument 1 (config) to be valid: \
             expected String, got Int (at `base_url`)",
        ),
    ]);
}
