//! TLS: verifying a server's certificate against the system's trust store,
//! extra certificate authorities given as PEM content, or neither.
//!
//! The test server's certificates come from an authority of the test's own,
//! which no system trusts.

use frostlang::Value;

use crate::common::tls::pem_literal;
use crate::common::{Server, TestCa, assert_raises, at, failure_kind, raises, value};

/// An HTTPS server answering `secure`, with a certificate `ca` signed for
/// `names`.
fn https_server(ca: &TestCa, names: &[&str]) -> Server {
    Server::start_tls(ca.server_config(names), |_, connection| {
        connection.respond(200, &[], b"secure");
    })
}

/// The result of a GET of `server`'s `/`, by a client with `config`.
fn get(server: &Server, config: &str) -> Value {
    value(&format!("http.client({config}).get('{}')", server.url("/")))
}

#[test]
fn a_certificate_no_trusted_authority_signed_fails_as_connect() {
    let ca = TestCa::new("untrusted");
    let server = https_server(&ca, &["127.0.0.1"]);
    let result = get(&server, "");
    assert_eq!(failure_kind(&result), "connect", "{result:?}");
}

#[test]
fn an_extra_authority_is_trusted_given_as_text_or_bytes() {
    let ca = TestCa::new("extra");
    let server = https_server(&ca, &["127.0.0.1"]);
    let pem = pem_literal(&ca.pem());
    for config in [
        format!("{{ca: [{pem}]}}"),
        format!("{{ca: [to_bytes({pem})]}}"),
        format!("{{ca: [{pem}], system_ca: true}}"),
        // Trusting only the extra authority.
        format!("{{ca: [{pem}], system_ca: false}}"),
    ] {
        let result = get(&server, &config);
        assert_eq!(
            at(&result, &["response", "body"]),
            Value::from("secure"),
            "with {config}: {result:?}"
        );
    }
}

#[test]
fn one_entry_may_hold_several_certificates() {
    let ca = TestCa::new("bundled");
    let other = TestCa::new("other");
    let server = https_server(&ca, &["127.0.0.1"]);
    let bundle = pem_literal(&format!("{}{}", other.pem(), ca.pem()));
    let result = get(&server, &format!("{{ca: [{bundle}]}}"));
    assert_eq!(at(&result, &["response", "body"]), Value::from("secure"));
}

#[test]
fn an_authority_that_did_not_sign_the_certificate_is_no_help() {
    let ca = TestCa::new("signer");
    let other = TestCa::new("bystander");
    let server = https_server(&ca, &["127.0.0.1"]);
    let result = get(&server, &format!("{{ca: [{}]}}", pem_literal(&other.pem())));
    assert_eq!(failure_kind(&result), "connect", "{result:?}");
}

#[test]
fn without_the_system_store_and_any_extra_authority_nothing_is_trusted() {
    let ca = TestCa::new("unused");
    let server = https_server(&ca, &["127.0.0.1"]);
    for config in ["{system_ca: false}", "{system_ca: false, ca: []}"] {
        let result = get(&server, config);
        assert_eq!(
            failure_kind(&result),
            "connect",
            "with {config}: {result:?}"
        );
    }
}

#[test]
fn a_certificate_for_another_name_fails_as_connect() {
    let ca = TestCa::new("names");
    // Signed for a name, while the request names the address.
    let server = https_server(&ca, &["server.test"]);
    let result = get(&server, &format!("{{ca: [{}]}}", pem_literal(&ca.pem())));
    assert_eq!(failure_kind(&result), "connect", "{result:?}");
}

#[test]
fn without_verification_any_certificate_is_accepted() {
    let ca = TestCa::new("unverified");
    let wrong_name = https_server(&ca, &["server.test"]);
    let unknown_issuer = https_server(&TestCa::new("unknown"), &["127.0.0.1"]);
    for server in [&wrong_name, &unknown_issuer] {
        for config in [
            "{verify_tls: false}",
            // The authorities then go unused.
            "{verify_tls: false, system_ca: false}",
            "{verify_tls: false, system_ca: false, ca: []}",
        ] {
            let result = get(server, config);
            assert_eq!(
                at(&result, &["response", "body"]),
                Value::from("secure"),
                "with {config}: {result:?}"
            );
        }
    }
}

#[test]
fn ca_entries_that_hold_no_certificate_raise() {
    assert_raises(&[
        (
            "http.client({ca: ['not a certificate']})",
            "Function ext.http.client got an invalid certificate in `ca[0]`: \
             no PEM certificate found",
        ),
        (
            "http.client({ca: ['']})",
            "Function ext.http.client got an invalid certificate in `ca[0]`: \
             no PEM certificate found",
        ),
    ]);
    let ca = TestCa::new("first");
    let malformed =
        pem_literal("-----BEGIN CERTIFICATE-----\nnot base64\n-----END CERTIFICATE-----");
    let message = raises(&format!(
        "http.client({{ca: [{}, {malformed}]}})",
        pem_literal(&ca.pem())
    ));
    assert_eq!(
        message,
        "Function ext.http.client got an invalid certificate in `ca[1]`: \
         builder error: invalid certificate encoding",
        "a malformed second entry is named, with the cause"
    );
}

#[test]
fn tls_options_check_their_types() {
    let invalid = "Function ext.http.client requires argument 1 (config) to be valid";
    assert_raises(&[
        (
            "http.client({verify_tls: 'no'})",
            &format!("{invalid}: expected Bool, got String (at `verify_tls`)"),
        ),
        (
            "http.client({system_ca: 0})",
            &format!("{invalid}: expected Bool, got Int (at `system_ca`)"),
        ),
        (
            "http.client({ca: 'PEM'})",
            &format!("{invalid}: expected Array, got String (at `ca`)"),
        ),
        (
            "http.client({ca: [1]})",
            &format!("{invalid}: expected String or Bytes, got Int (at `ca[0]`)"),
        ),
    ]);
}
