//! Headers, query parameters, form bodies, and the `auth` helpers: what a
//! request sends, what a response's headers become, and what raises.

use frostlang::Value;

use crate::common::{Server, assert_raises, at, value};

/// The header lines the server saw for the one request `body` made, with
/// `url` bound to a URL on it, and `client` to a client.
fn headers_sent(server: &Server, client_config: &str, call: &str) -> Vec<(String, String)> {
    let script = format!(
        "def url = '{}'\ndef client = http.client({client_config})\n{call}",
        server.url("/h")
    );
    let result = value(&script);
    assert_eq!(
        at(&result, &["ok"]),
        Value::Bool(true),
        "the request succeeds: {result:?}"
    );
    server
        .request()
        .headers
        .into_iter()
        .map(|(name, value)| {
            (
                name,
                String::from_utf8(value).expect("UTF-8 in these tests"),
            )
        })
        .collect()
}

/// The values of each header named `name` among `headers`, in order.
fn values_of(headers: &[(String, String)], name: &str) -> Vec<String> {
    headers
        .iter()
        .filter(|(named, _)| named == name)
        .map(|(_, value)| value.clone())
        .collect()
}

#[test]
fn a_requests_headers_are_sent_with_names_in_any_case() {
    let server = Server::ok("");
    let headers = headers_sent(
        &server,
        "",
        "client.get(url, {headers: {['X-Token']: 'abc', accept: 'text/plain', ['x-Caf']: 'caf\u{e9}'}})",
    );
    assert_eq!(values_of(&headers, "x-token"), ["abc"]);
    assert_eq!(
        values_of(&headers, "accept"),
        ["text/plain"],
        "replacing any default"
    );
    assert_eq!(
        values_of(&headers, "x-caf"),
        ["caf\u{e9}"],
        "UTF-8 is sent as is"
    );
}

#[test]
fn an_array_of_values_repeats_a_header_in_order() {
    let server = Server::ok("");
    let headers = headers_sent(
        &server,
        "",
        "client.get(url, {headers: {['x-many']: ['one', 'two', 'three']}})",
    );
    assert_eq!(values_of(&headers, "x-many"), ["one", "two", "three"]);
}

#[test]
fn a_clients_headers_are_sent_unless_a_request_names_them() {
    let config = "{headers: {['X-Default']: 'd', ['x-repeated']: ['r1', 'r2'], ['x-replaced']: 'old', ['x-removed']: 'gone'}}";
    let server = Server::ok("");
    let headers = headers_sent(
        &server,
        config,
        "client.get(url, {headers: {['X-REPLACED']: 'new', ['x-removed']: []}})",
    );
    assert_eq!(values_of(&headers, "x-default"), ["d"]);
    assert_eq!(
        values_of(&headers, "x-repeated"),
        ["r1", "r2"],
        "every value of a default"
    );
    assert_eq!(
        values_of(&headers, "x-replaced"),
        ["new"],
        "replaced whatever its case"
    );
    assert_eq!(
        values_of(&headers, "x-removed"),
        Vec::<String>::new(),
        "an empty Array sends none of a default"
    );

    let server = Server::ok("");
    let headers = headers_sent(&server, config, "client.get(url)");
    assert_eq!(
        values_of(&headers, "x-replaced"),
        ["old"],
        "without the request's own"
    );
    assert_eq!(values_of(&headers, "x-removed"), ["gone"]);
}

#[test]
fn headers_that_cannot_be_sent_raise() {
    let get =
        |headers: &str| format!("http.client().get('http://a.invalid', {{headers: {headers}}})");
    let client = |headers: &str| format!("http.client({{headers: {headers}}})");
    assert_raises(&[
        (
            &get("{['X-Token']: 'a', ['x-token']: 'b'}"),
            r#"Function client.get got headers "X-Token" and "x-token", which differ only in case"#,
        ),
        (
            &client("{accept: 'a', ACCEPT: 'b'}"),
            r#"Function ext.http.client got headers "ACCEPT" and "accept", which differ only in case"#,
        ),
        (
            &get("{['bad name']: 'a'}"),
            r#"Function client.get got an invalid header name "bad name""#,
        ),
        (
            &get("{['']: 'a'}"),
            r#"Function client.get got an invalid header name """#,
        ),
        (
            &get("{['x-a']: 'line\\nbreak'}"),
            r#"Function client.get got an invalid value "line\nbreak" for header "x-a""#,
        ),
        (
            &client("{['x-a']: ['ok', 'nul\\u{0}']}"),
            r#"Function ext.http.client got an invalid value "nul\u{0}" for header "x-a""#,
        ),
        (
            &get("{['x-a']: 1}"),
            "Function client.get requires argument 2 (options) to be valid: \
             expected String or Array of String, got Int (at `headers[\"x-a\"]`)",
        ),
        (
            &get("{['x-a']: ['ok', 2]}"),
            "Function client.get requires argument 2 (options) to be valid: \
             expected String, got Int (at `headers[\"x-a\"][1]`)",
        ),
        (
            &get("{[1]: 'a'}"),
            "Function client.get requires argument 2 (options) to be valid: \
             expected String, got Int (at a key of `headers`)",
        ),
    ]);
}

#[test]
fn response_headers_are_lowercase_and_repeated_ones_are_arrays() {
    let server = Server::start(|_, connection| {
        connection.write(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nX-Single: one\r\nX-Repeated: a\r\n\
              x-repeated: b\r\nX-Utf8: caf\xc3\xa9\r\nX-Binary: \xff\xfe\r\n\r\n",
        );
    });
    let result = value(&format!("http.client().get('{}')", server.url("/")));
    assert_eq!(
        at(&result, &["response", "headers"]),
        value(
            r#"{
                ['content-length']: '0',
                ['x-single']: 'one',
                ['x-repeated']: ['a', 'b'],
                ['x-utf8']: 'café',
                ['x-binary']: x"fffe",
            }"#
        )
    );
}

#[test]
fn query_parameters_are_added_to_the_url() {
    let cases = [
        (
            "/q",
            "{a: 1, b: 'x y', c: true, d: false}",
            "/q?a=1&b=x+y&c=true&d=false",
        ),
        // Null is a parameter without a value; an Array repeats one.
        (
            "/q",
            "{flag: null, e: [1, 'two', null]}",
            "/q?e=1&e=two&e&flag",
        ),
        // Added after any query the URL has.
        ("/q?z=0", "{a: -1}", "/q?z=0&a=-1"),
        ("/q", "{['a&b']: '=?/#'}", "/q?a%26b=%3D%3F%2F%23"),
        ("/q", "{['caf\u{e9}']: '\u{e9}'}", "/q?caf%C3%A9=%C3%A9"),
        // Nothing to add adds nothing.
        ("/q", "{}", "/q"),
        ("/q", "{a: []}", "/q"),
        ("/q?z=0", "{}", "/q?z=0"),
    ];
    for (path, query, expected) in cases {
        let server = Server::ok("");
        let result = value(&format!(
            "http.client().get('{}', {{query: {query}}})",
            server.url(path)
        ));
        assert_eq!(at(&result, &["ok"]), Value::Bool(true));
        assert_eq!(server.request().target, expected, "{query} on {path}");
    }
}

#[test]
fn query_values_of_other_types_raise() {
    let invalid = "Function client.get requires argument 2 (options) to be valid";
    let get = |query: &str| format!("http.client().get('http://a.invalid', {{query: {query}}})");
    assert_raises(&[
        (
            &get("{a: 1.5}"),
            &format!(
                "{invalid}: expected String, Int, Bool, Null, or an Array of them, got Float (at `query.a`)"
            ),
        ),
        (
            &get("{a: {b: 1}}"),
            &format!(
                "{invalid}: expected String, Int, Bool, Null, or an Array of them, got Map (at `query.a`)"
            ),
        ),
        (
            &get("{a: [1, [2]]}"),
            &format!("{invalid}: expected String, Int, Bool, or Null, got Array (at `query.a[1]`)"),
        ),
        (
            &get(r#"{a: x"00"}"#),
            &format!(
                "{invalid}: expected String, Int, Bool, Null, or an Array of them, got Bytes (at `query.a`)"
            ),
        ),
        (
            &get("{[1]: 'a'}"),
            &format!("{invalid}: expected String, got Int (at a key of `query`)"),
        ),
        (
            &get("'a=1'"),
            &format!("{invalid}: expected Map, got String (at `query`)"),
        ),
    ]);
}

#[test]
fn form_encodes_fields_as_query_parameters_are() {
    let cases = [
        ("{a: 1, b: 'x y', c: [true, null]}", "a=1&b=x+y&c=true&c"),
        ("{['a&b']: '=?'}", "a%26b=%3D%3F"),
        ("{}", ""),
        ("{a: []}", ""),
    ];
    for (fields, expected) in cases {
        assert_eq!(
            value(&format!("http.form({fields})")),
            Value::from(expected),
            "form({fields})"
        );
    }
    assert_raises(&[(
        "http.form({a: 1.5})",
        "Function ext.http.form requires argument 1 (fields) to be valid: \
         expected String, Int, Bool, Null, or an Array of them, got Float (at `a`)",
    )]);
}

#[test]
fn a_form_body_posts_with_its_content_type() {
    let server = Server::ok("");
    let result = value(&format!(
        "http.client().post('{}', {{
            headers: {{['content-type']: 'application/x-www-form-urlencoded'}},
            body: http.form({{name: 'Ada Lovelace', langs: ['en', 'fr']}}),
        }})",
        server.url("/form")
    ));
    assert_eq!(at(&result, &["ok"]), Value::Bool(true));
    let request = server.request();
    assert_eq!(request.body, b"langs=en&langs=fr&name=Ada+Lovelace");
    assert_eq!(
        request.header("content-type"),
        ["application/x-www-form-urlencoded"]
    );
}

#[test]
fn auth_helpers_give_an_authorization_header() {
    let cases = [
        (
            "http.auth.bearer('abc.def')",
            "{authorization: 'Bearer abc.def'}",
        ),
        ("http.auth.bearer('')", "{authorization: 'Bearer '}"),
        // RFC 7617's own example.
        (
            "http.auth.basic('Aladdin', 'open sesame')",
            "{authorization: 'Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=='}",
        ),
        // A password may contain a colon; text is sent as UTF-8.
        (
            "http.auth.basic('u', 'a:b')",
            "{authorization: 'Basic dTphOmI='}",
        ),
        (
            "http.auth.basic('caf\u{e9}', '')",
            "{authorization: 'Basic Y2Fmw6k6'}",
        ),
    ];
    for (call, expected) in cases {
        assert_eq!(value(call), value(expected), "{call}");
    }
    assert_raises(&[(
        "http.auth.basic('a:b', 'p')",
        "Function ext.http.auth.basic got a user containing `:`, which Basic authentication cannot send",
    )]);
}

#[test]
fn headers_the_client_manages_cannot_be_set() {
    for name in [
        "host",
        "Host",
        "content-length",
        "Transfer-Encoding",
        "ACCEPT-ENCODING",
    ] {
        let message = format!("cannot set header \"{name}\": the HTTP client manages it");
        assert_raises(&[
            (
                &format!("http.client().get('http://a.invalid', {{headers: {{['{name}']: 'x'}}}})"),
                &format!("Function client.get {message}"),
            ),
            (
                &format!("http.client({{headers: {{['{name}']: ['x']}}}})"),
                &format!("Function ext.http.client {message}"),
            ),
            // Even with no value to send.
            (
                &format!("http.client({{headers: {{['{name}']: []}}}})"),
                &format!("Function ext.http.client {message}"),
            ),
        ]);
    }
}

#[test]
fn a_client_sends_its_own_user_agent_unless_told_otherwise() {
    let default = format!("frost-http/{}", env!("CARGO_PKG_VERSION"));
    let cases = [
        ("", "client.get(url)", vec![default.clone()]),
        // Named by the client, or by the request, in any case.
        (
            "{headers: {['User-Agent']: 'app/1'}}",
            "client.get(url)",
            vec![String::from("app/1")],
        ),
        (
            "",
            "client.get(url, {headers: {['USER-AGENT']: 'script/2'}})",
            vec![String::from("script/2")],
        ),
        (
            "{headers: {['user-agent']: 'app/1'}}",
            "client.get(url, {headers: {['user-agent']: 'script/2'}})",
            vec![String::from("script/2")],
        ),
        // An empty Array sends none at all.
        (
            "",
            "client.get(url, {headers: {['user-agent']: []}})",
            vec![],
        ),
        ("{headers: {['user-agent']: []}}", "client.get(url)", vec![]),
    ];
    for (config, call, expected) in cases {
        let server = Server::ok("");
        let headers = headers_sent(&server, config, call);
        assert_eq!(
            values_of(&headers, "user-agent"),
            expected,
            "{config} {call}"
        );
    }
}

#[test]
fn credentials_in_a_url_are_sent_as_basic_authorization() {
    let server = Server::ok("");
    let authority = server.authority();
    let headers = headers_sent(
        &server,
        "",
        &format!("client.get('http://Aladdin:open%20sesame@{authority}/')"),
    );
    assert_eq!(
        values_of(&headers, "authorization"),
        ["Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="]
    );

    let server = Server::ok("");
    let authority = server.authority();
    let headers = headers_sent(
        &server,
        "",
        &format!("client.get('http://ada:pw@{authority}/', {{headers: http.auth.bearer('t')}})"),
    );
    assert_eq!(
        values_of(&headers, "authorization"),
        ["Bearer t"],
        "an authorization header replaces the URL's credentials"
    );
}

#[test]
fn auth_headers_combine_with_others() {
    let server = Server::ok("");
    let headers = headers_sent(
        &server,
        "",
        "client.get(url, {headers: http.auth.bearer('t0k') + {accept: 'text/plain'}})",
    );
    assert_eq!(values_of(&headers, "authorization"), ["Bearer t0k"]);
    assert_eq!(values_of(&headers, "accept"), ["text/plain"]);
}
