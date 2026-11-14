//! The extension as a whole: what it provides, and the arity and type
//! checks of every function.

use frostlang::Value;

use crate::common::{assert_arity, assert_raises, value};

/// Binds `client`, a client with no config.
const CLIENT: &str = "def client = http.client()";

#[test]
fn the_module_provides_exactly_its_functions() {
    assert_eq!(
        value("keys(http)"),
        value("['auth', 'build_url', 'client', 'form', 'parse_url']")
    );
    assert_eq!(value("keys(http.auth)"), value("['basic', 'bearer']"));
}

#[test]
fn a_client_provides_exactly_its_functions() {
    assert_eq!(
        value("keys(http.client())"),
        value("['delete', 'get', 'head', 'patch', 'post', 'put', 'request', 'request_all']")
    );
    assert_eq!(
        value("map(values(http.client()), is_function) @ all()"),
        Value::Bool(true),
        "every entry of a client is a function"
    );
}

#[test]
fn every_function_checks_its_arity() {
    assert_arity(
        "",
        "http.client",
        "ext.http.client",
        "between 0 and 1",
        &[2, 3],
    );
    assert_arity("", "http.parse_url", "ext.http.parse_url", 1, &[0, 2]);
    assert_arity("", "http.build_url", "ext.http.build_url", 1, &[0, 2]);
    assert_arity("", "http.form", "ext.http.form", 1, &[0, 2]);
    assert_arity("", "http.auth.bearer", "ext.http.auth.bearer", 1, &[0, 2]);
    assert_arity("", "http.auth.basic", "ext.http.auth.basic", 2, &[0, 1, 3]);
    assert_arity(CLIENT, "client.request", "client.request", 1, &[0, 2]);
    for method in [
        "get",
        "head",
        "delete",
        "post",
        "put",
        "patch",
        "request_all",
    ] {
        assert_arity(
            CLIENT,
            &format!("client.{method}"),
            &format!("client.{method}"),
            "between 1 and 2",
            &[0, 3],
        );
    }
}

#[test]
fn module_functions_check_their_argument_types() {
    assert_raises(&[
        (
            "http.client(1)",
            "Function ext.http.client requires Map as argument 1 (config), got Int",
        ),
        (
            "http.parse_url(1)",
            "Function ext.http.parse_url requires String as argument 1 (url), got Int",
        ),
        (
            r#"http.parse_url(x"61")"#,
            "Function ext.http.parse_url requires String as argument 1 (url), got Bytes",
        ),
        (
            "http.build_url('https://a.example')",
            "Function ext.http.build_url requires Map as argument 1 (parts), got String",
        ),
        (
            "http.form([])",
            "Function ext.http.form requires Map as argument 1 (fields), got Array",
        ),
        (
            "http.auth.bearer(1)",
            "Function ext.http.auth.bearer requires String as argument 1 (token), got Int",
        ),
        (
            "http.auth.basic(1, 'p')",
            "Function ext.http.auth.basic requires String as argument 1 (user), got Int",
        ),
        (
            "http.auth.basic('u', null)",
            "Function ext.http.auth.basic requires String as argument 2 (password), got Null",
        ),
    ]);
}

#[test]
fn client_functions_check_their_argument_types() {
    let cases = [
        (
            "client.request('https://a.example')",
            "Function client.request requires Map as argument 1 (config), got String",
        ),
        (
            "client.get(1)",
            "Function client.get requires argument 1 (url) to be valid: \
             expected String or URL Map, got Int",
        ),
        (
            "client.post(null)",
            "Function client.post requires argument 1 (url) to be valid: \
             expected String or URL Map, got Null",
        ),
        (
            "client.get('https://a.example', 1)",
            "Function client.get requires Map as argument 2 (options), got Int",
        ),
        (
            "client.request_all({})",
            "Function client.request_all requires Array as argument 1 (configs), got Map",
        ),
        (
            "client.request_all([1])",
            "Function client.request_all requires Map as element 0 of argument 1 (configs), got Int",
        ),
        (
            "client.request_all([], [])",
            "Function client.request_all requires Map as argument 2 (options), got Array",
        ),
    ];
    for (call, message) in cases {
        assert_raises(&[(&format!("{CLIENT}\n{call}"), message)]);
    }
}
