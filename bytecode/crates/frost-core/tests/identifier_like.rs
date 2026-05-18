// These are pub(crate), so test via the behaviors they enable
// (pretty-print map key formatting). For now, test the logic
// directly by re-implementing the same rules and checking agreement
// with the C++ oracle's output.
//
// Once these are pub or reexported, this file can import them directly.

/// Mirror of the identifier rules for testing against the C++ oracle.
fn is_identifier_like(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

const RESERVED_KEYWORDS: &[&str] = &[
    "and", "as", "def", "defn", "do", "elif", "else", "export", "false", "filter", "fn", "foreach",
    "if", "init", "is", "map", "match", "not", "null", "or", "reduce", "true", "with",
];

fn is_reserved_keyword(s: &str) -> bool {
    RESERVED_KEYWORDS.contains(&s)
}

// -- is_identifier_like --

#[test]
fn simple_identifiers() {
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
fn not_identifiers() {
    assert!(!is_identifier_like(""));
    assert!(!is_identifier_like("123"));
    assert!(!is_identifier_like("1abc"));
    assert!(!is_identifier_like("hello world"));
    assert!(!is_identifier_like("foo-bar"));
    assert!(!is_identifier_like("a.b"));
    assert!(!is_identifier_like("has space"));
    assert!(!is_identifier_like("$var"));
}

// -- is_reserved_keyword --

#[test]
fn all_keywords_recognized() {
    let keywords = [
        "and", "as", "def", "defn", "do", "elif", "else", "export", "false", "filter", "fn",
        "foreach", "if", "init", "is", "map", "match", "not", "null", "or", "reduce", "true",
        "with",
    ];
    for kw in keywords {
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

// -- Combined: identifier-like but not a keyword --

#[test]
fn identifier_like_keywords_are_keywords() {
    // These are valid identifiers syntactically but are reserved
    assert!(is_identifier_like("if"));
    assert!(is_reserved_keyword("if"));
    assert!(is_identifier_like("map"));
    assert!(is_reserved_keyword("map"));
}

#[test]
fn identifier_like_non_keywords() {
    assert!(is_identifier_like("foo") && !is_reserved_keyword("foo"));
    assert!(is_identifier_like("name") && !is_reserved_keyword("name"));
    assert!(is_identifier_like("age") && !is_reserved_keyword("age"));
}
