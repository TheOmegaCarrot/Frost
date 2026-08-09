//! Tests for the identifier rules: what shapes a name may take, and which names
//! Frost reserves. Expectations are cross-checked against the C++ oracle.

use frost_runtime::{
    KEYWORDS, is_identifier_like, is_identifier_like_and_not_keyword, is_reserved_keyword,
};

// -- is_identifier_like --

#[test]
fn valid_identifiers() {
    assert!(is_identifier_like("foo"));
    assert!(is_identifier_like("x"));
    assert!(is_identifier_like("hello_world"));
    assert!(is_identifier_like("_private"));
    assert!(is_identifier_like("_"));
    assert!(is_identifier_like("camelCase"));
    assert!(is_identifier_like("ALL_CAPS"));
    assert!(is_identifier_like("a1"));
    assert!(is_identifier_like("item42"));
}

#[test]
fn invalid_identifiers() {
    assert!(!is_identifier_like(""));
    assert!(!is_identifier_like("123"));
    assert!(!is_identifier_like("1abc"));
    assert!(!is_identifier_like("hello world"));
    assert!(!is_identifier_like("foo-bar"));
    assert!(!is_identifier_like("a.b"));
    assert!(!is_identifier_like("has space"));
    assert!(!is_identifier_like("$var"));
}

#[test]
fn non_ascii_is_not_identifier_like() {
    // The rules are ASCII-only, so letters outside it do not qualify however
    // letter-like they read.
    assert!(!is_identifier_like("naïve"));
    assert!(!is_identifier_like("é"));
    assert!(!is_identifier_like("日本"));
}

// -- is_reserved_keyword --

#[test]
fn every_listed_keyword_is_recognized() {
    // Drawn from `KEYWORDS` itself rather than a copy, so the list cannot be
    // extended without this covering the addition.
    for kw in KEYWORDS {
        assert!(is_reserved_keyword(kw), "{kw} should be a keyword");
    }
}

#[test]
fn non_keywords() {
    assert!(!is_reserved_keyword("foo"));
    assert!(!is_reserved_keyword("hello"));
    assert!(!is_reserved_keyword("IF"));
    assert!(!is_reserved_keyword("True"));
    assert!(!is_reserved_keyword(""));
}

#[test]
fn keywords_are_case_sensitive() {
    assert!(is_reserved_keyword("if"));
    assert!(!is_reserved_keyword("If"));
    assert!(!is_reserved_keyword("IF"));
}

// -- is_identifier_like_and_not_keyword --

#[test]
fn a_keyword_is_identifier_shaped_but_not_usable() {
    // The distinction between the two checks: shape alone accepts `if`.
    for kw in KEYWORDS {
        assert!(is_identifier_like(kw), "{kw} is identifier-shaped");
        assert!(
            !is_identifier_like_and_not_keyword(kw),
            "{kw} is reserved, so it is not a usable identifier"
        );
    }
}

#[test]
fn an_ordinary_name_passes_both() {
    for name in ["foo", "name", "age", "_private", "item42"] {
        assert!(is_identifier_like_and_not_keyword(name), "{name}");
    }
}

#[test]
fn a_malformed_name_fails_regardless_of_keywords() {
    assert!(!is_identifier_like_and_not_keyword(""));
    assert!(!is_identifier_like_and_not_keyword("1abc"));
    assert!(!is_identifier_like_and_not_keyword("foo-bar"));
}
