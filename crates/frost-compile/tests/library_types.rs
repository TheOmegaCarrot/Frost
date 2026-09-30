//! The Types globals, from Frost source: conversion between String and Bytes.
//!
//! `to_bytes` encodes a String as its UTF-8 bytes, and passes Bytes through
//! unchanged. `from_utf8` decodes Bytes
//! holding UTF-8 into a String, and returns Null for Bytes that are not UTF-8.
//! A value of the wrong type is a type error, never Null.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over a literal is checked both folded and at run time. Cases that capture
//! their input are never folded.

mod common;

use common::{Script, raises, run};
use frost_runtime::Value;

fn bytes(octets: &[u8]) -> Value {
    Value::from(octets)
}

/// Assert each `source` runs to `expected`.
fn assert_values(cases: &[(&str, Value)]) {
    for (source, expected) in cases {
        assert_eq!(&run(source), expected, "{source:?} is {expected:?}");
    }
}

/// Assert each `source` raises exactly `message`.
fn assert_raises(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        assert_eq!(raises(source), *message, "{source:?} raises {message:?}");
    }
}

// --- to_bytes ---

#[test]
fn to_bytes_encodes_a_string_as_utf8() {
    assert_values(&[
        ("to_bytes('hi')", bytes(b"hi")),
        ("to_bytes('')", bytes(b"")),
        // One two-byte and one three-byte encoding.
        (r"to_bytes('\u{e9}')", bytes(&[0xc3, 0xa9])),
        (r"to_bytes('\u{20ac}')", bytes(&[0xe2, 0x82, 0xac])),
        // Four bytes: a character outside the Basic Multilingual Plane.
        (r"to_bytes('\u{1f600}')", bytes(&[0xf0, 0x9f, 0x98, 0x80])),
        (r"to_bytes('a\u{0}b')", bytes(&[b'a', 0x00, b'b'])),
    ]);
}

#[test]
fn to_bytes_result_is_bytes_not_a_string() {
    assert_values(&[
        ("is_bytes(to_bytes('hi'))", Value::Bool(true)),
        ("is_string(to_bytes('hi'))", Value::Bool(false)),
        ("to_bytes('hi') == x'6869'", Value::Bool(true)),
        ("to_bytes('hi') == 'hi'", Value::Bool(false)),
    ]);
}

#[test]
fn to_bytes_passes_bytes_through_unchanged() {
    assert_values(&[
        ("to_bytes(x'6869')", bytes(b"hi")),
        ("to_bytes(x'')", bytes(b"")),
        // Bytes need not be UTF-8: nothing is decoded or validated.
        ("to_bytes(x'ff00c0af')", bytes(&[0xff, 0x00, 0xc0, 0xaf])),
        ("to_bytes(to_bytes('hi'))", bytes(b"hi")),
    ]);
}

#[test]
fn to_bytes_converts_runtime_values() {
    let convert = |value: Value| Script::new("to_bytes(v)").capture("v", value).run();
    assert_eq!(
        convert(Value::from("\u{20ac}!")),
        bytes(&[0xe2, 0x82, 0xac, b'!'])
    );
    assert_eq!(convert(bytes(&[b'h', 0xff])), bytes(&[b'h', 0xff]));
}

#[test]
fn to_bytes_rejects_every_type_but_string_and_bytes() {
    let message =
        |ty: &str| format!("Function to_bytes requires String or Bytes as argument 1, got {ty}");
    for (argument, ty) in [
        ("null", "Null"),
        ("1", "Int"),
        ("1.5", "Float"),
        ("true", "Bool"),
        ("[104, 105]", "Array"),
        ("{a: 1}", "Map"),
        ("fn -> 'hi'", "Function"),
    ] {
        let source = format!("to_bytes({argument})");
        assert_eq!(raises(&source), message(ty), "{source:?}");
    }
}

#[test]
fn to_bytes_takes_exactly_one_argument() {
    assert_raises(&[
        (
            "to_bytes()",
            "Function to_bytes expects 1 arguments, but was called with 0",
        ),
        (
            "to_bytes('a', 'b')",
            "Function to_bytes expects 1 arguments, but was called with 2",
        ),
    ]);
}

// --- from_utf8 ---

#[test]
fn from_utf8_decodes_utf8_bytes() {
    assert_values(&[
        ("from_utf8(x'6869')", Value::from("hi")),
        ("from_utf8(x'')", Value::from("")),
        ("from_utf8(x'c3a9')", Value::from("\u{e9}")),
        ("from_utf8(x'e282ac')", Value::from("\u{20ac}")),
        ("from_utf8(x'f09f9880')", Value::from("\u{1f600}")),
        // NUL is valid UTF-8, not a terminator.
        ("from_utf8(x'610062')", Value::from("a\0b")),
    ]);
}

#[test]
fn from_utf8_returns_null_for_bytes_that_are_not_utf8() {
    for source in [
        // A byte that never appears in UTF-8.
        "from_utf8(x'ff')",
        // A continuation byte with no lead byte.
        "from_utf8(x'80')",
        // A three-byte sequence cut short.
        "from_utf8(x'e282')",
        // An overlong encoding of '/'.
        "from_utf8(x'c0af')",
        // An encoded UTF-16 surrogate (U+D800).
        "from_utf8(x'eda080')",
        // Past the last code point (U+110000).
        "from_utf8(x'f4908080')",
        // Valid text followed by an invalid byte: no partial decode.
        "from_utf8(x'6869ff')",
    ] {
        assert_eq!(run(source), Value::Null, "{source:?} is Null");
    }
}

#[test]
fn from_utf8_decodes_runtime_bytes() {
    let decode = |octets: &[u8]| {
        Script::new("from_utf8(b)")
            .capture("b", bytes(octets))
            .run()
    };
    assert_eq!(decode(&[0xe2, 0x82, 0xac, b'!']), Value::from("\u{20ac}!"));
    assert_eq!(decode(&[b'h', 0xff]), Value::Null, "invalid UTF-8 is Null");
}

#[test]
fn from_utf8_inverts_to_bytes() {
    assert_values(&[
        (
            r"from_utf8(to_bytes('caf\u{e9} \u{1f600}'))",
            Value::from("caf\u{e9} \u{1f600}"),
        ),
        ("from_utf8(to_bytes(''))", Value::from("")),
        ("to_bytes(from_utf8(x'e282ac'))", bytes(&[0xe2, 0x82, 0xac])),
    ]);
}

#[test]
fn from_utf8_rejects_every_type_but_bytes() {
    // A String is rejected rather than passed through: it is already decoded.
    let message = |ty: &str| format!("Function from_utf8 requires Bytes as argument 1, got {ty}");
    for (argument, ty) in [
        ("'hi'", "String"),
        ("null", "Null"),
        ("1", "Int"),
        ("1.5", "Float"),
        ("true", "Bool"),
        ("[104, 105]", "Array"),
        ("{a: 1}", "Map"),
        ("fn -> x'6869'", "Function"),
    ] {
        let source = format!("from_utf8({argument})");
        assert_eq!(raises(&source), message(ty), "{source:?}");
    }
}

#[test]
fn from_utf8_takes_exactly_one_argument() {
    assert_raises(&[
        (
            "from_utf8()",
            "Function from_utf8 expects 1 arguments, but was called with 0",
        ),
        (
            "from_utf8(x'68', x'69')",
            "Function from_utf8 expects 1 arguments, but was called with 2",
        ),
    ]);
}
