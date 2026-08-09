use std::sync::Arc;

use frost_runtime::{FrostArray, FrostFloat, FrostMap, MapKey, Value};

fn str_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s))
}

// -- Primitives: to_frost_string --

#[test]
fn null() {
    assert_eq!(Value::Null.to_frost_string(), "null");
}

#[test]
fn bool_true() {
    assert_eq!(Value::from(true).to_frost_string(), "true");
}

#[test]
fn bool_false() {
    assert_eq!(Value::from(false).to_frost_string(), "false");
}

#[test]
fn int() {
    assert_eq!(Value::from(42i64).to_frost_string(), "42");
}

#[test]
fn int_negative() {
    assert_eq!(Value::from(-7i64).to_frost_string(), "-7");
}

#[test]
fn int_zero() {
    assert_eq!(Value::from(0i64).to_frost_string(), "0");
}

#[test]
fn float_simple() {
    let v: Value = 3.14.try_into().unwrap();
    assert_eq!(v.to_frost_string(), "3.14");
}

#[test]
fn float_integer_valued_has_decimal() {
    let v: Value = 1.0.try_into().unwrap();
    assert!(v.to_frost_string().contains('.'));
}

#[test]
fn float_zero() {
    let v: Value = 0.0.try_into().unwrap();
    assert_eq!(v.to_frost_string(), "0.0");
}

#[test]
fn float_negative_zero() {
    let v: Value = (-0.0f64).try_into().unwrap();
    assert_eq!(v.to_frost_string(), "-0.0");
}

#[test]
fn string_top_level_is_raw() {
    assert_eq!(Value::from("hello").to_frost_string(), "hello");
}

#[test]
fn string_top_level_with_special_chars_is_raw() {
    assert_eq!(
        Value::from("line1\nline2").to_frost_string(),
        "line1\nline2"
    );
}

#[test]
fn string_top_level_non_ascii_is_raw() {
    // Text renders as the text it is, whatever its code points.
    assert_eq!(Value::from("héllo wörld").to_frost_string(), "héllo wörld");
}

#[test]
fn bytes_top_level_render_as_a_literal() {
    // Bytes has no bare form: it renders as its `x'..'` literal at every tier.
    let v: Value = vec![0x80u8, 0xff].into();
    assert_eq!(v.to_frost_string(), "x'80ff'");
}

#[test]
fn function_placeholder() {
    // Can't easily construct a Function value in tests, tested via type_name coverage
}

// -- Opaque renderings --
// The exact format is unpinned; these tests hold the two decided properties:
// every rendering names the payload's type, and an approximation never renders
// as if it were a bare String value.

/// An opaque payload with no string approximation.
#[derive(Debug)]
struct Widget;

impl frost_runtime::FrostOpaque for Widget {
    fn type_name(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Widget")
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// An opaque payload with a string approximation.
#[derive(Debug)]
struct Gizmo;

impl frost_runtime::FrostOpaque for Gizmo {
    fn type_name(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Gizmo")
    }

    fn try_to_string(&self) -> Option<String> {
        Some("running gizmo".to_string())
    }
}

#[test]
fn opaque_renderings_show_the_type_name() {
    let v = Value::opaque(Widget);
    for s in [
        v.to_frost_string(),
        v.to_pretty_string(),
        v.to_debug_string(),
    ] {
        assert!(s.contains("Widget"), "expected the type name, got: {s}");
    }
}

#[test]
fn opaque_approximation_is_never_a_bare_string() {
    // A rendering may use try_to_string, but must stay visibly opaque:
    // never byte-identical to the approximation as a plain String.
    let v = Value::opaque(Gizmo);
    for s in [
        v.to_frost_string(),
        v.to_pretty_string(),
        v.to_debug_string(),
    ] {
        assert_ne!(
            s, "running gizmo",
            "approximation rendered as a bare String"
        );
    }
}

// -- Primitives: to_debug_string --

#[test]
fn debug_string_quoted() {
    assert_eq!(Value::from("hello").to_debug_string(), "\"hello\"");
}

#[test]
fn debug_string_escapes_newline() {
    assert_eq!(Value::from("a\nb").to_debug_string(), "\"a\\nb\"");
}

#[test]
fn debug_string_escapes_tab() {
    assert_eq!(Value::from("a\tb").to_debug_string(), "\"a\\tb\"");
}

#[test]
fn debug_string_escapes_carriage_return() {
    assert_eq!(Value::from("a\rb").to_debug_string(), "\"a\\rb\"");
}

#[test]
fn debug_string_escapes_backslash() {
    assert_eq!(Value::from("a\\b").to_debug_string(), "\"a\\\\b\"");
}

#[test]
fn debug_string_escapes_double_quote() {
    assert_eq!(
        Value::from("say \"hi\"").to_debug_string(),
        "\"say \\\"hi\\\"\""
    );
}

#[test]
fn debug_string_escapes_null_as_unicode() {
    assert_eq!(Value::from("\x00").to_debug_string(), "\"\\u{0}\"");
}

#[test]
fn debug_string_passes_non_ascii_characters_through() {
    // A String is text, so escaping it byte-wise would both mangle it and make it
    // indistinguishable from the Bytes holding those same bytes.
    assert_eq!(Value::from("é").to_debug_string(), "\"é\"");
    assert_eq!(Value::from("日本").to_debug_string(), "\"日本\"");
}

#[test]
fn debug_string_escapes_non_ascii_control_as_unicode() {
    // U+0085 is a control character with no readable spelling, so it stays escaped
    // even though it is not ASCII. It escapes as its scalar value, `\u{85}`, not as
    // its UTF-8 bytes: a String is text, and byte notation belongs to Bytes alone.
    assert_eq!(Value::from("\u{85}").to_debug_string(), "\"\\u{85}\"");
}

#[test]
fn debug_string_escapes_ascii_control_as_unicode() {
    // Minimal hex digits, matching Rust's `escape_debug`: U+0001 is `\u{1}`.
    assert_eq!(Value::from("\u{1}").to_debug_string(), "\"\\u{1}\"");
}

#[test]
fn debug_bytes_render_as_a_literal() {
    // Bytes, not String: `From<Vec<u8>>` builds the binary type.
    let v: Value = vec![0x80u8, 0xff].into();
    assert_eq!(v.to_debug_string(), "x'80ff'");
}

#[test]
fn debug_string_printable_ascii_unescaped() {
    assert_eq!(
        Value::from("abc 123 !@#").to_debug_string(),
        "\"abc 123 !@#\""
    );
}

#[test]
fn debug_int_same_as_to_string() {
    assert_eq!(Value::from(42i64).to_debug_string(), "42");
}

// -- Empty structures --

#[test]
fn empty_array() {
    let v = Value::from(FrostArray::from(vec![]));
    assert_eq!(v.to_frost_string(), "[]");
}

#[test]
fn empty_map() {
    let v = Value::from(FrostMap::empty());
    assert_eq!(v.to_frost_string(), "{}");
}

// -- Arrays: compact --

#[test]
fn array_compact() {
    let arr = FrostArray::from(vec![Value::from(1i64), Value::from("hi"), Value::Null]);
    assert_eq!(Value::from(arr).to_frost_string(), "[ 1, \"hi\", null ]");
}

#[test]
fn array_single_element() {
    let arr = FrostArray::from(vec![Value::from(42i64)]);
    assert_eq!(Value::from(arr).to_frost_string(), "[ 42 ]");
}

#[test]
fn nested_array_compact() {
    let inner = FrostArray::from(vec![Value::from(2i64), Value::from(3i64)]);
    let outer = FrostArray::from(vec![Value::from(1i64), Value::from(inner)]);
    assert_eq!(Value::from(outer).to_frost_string(), "[ 1, [ 2, 3 ] ]");
}

// -- Maps: compact --

#[test]
fn map_compact_string_keys() {
    let map: FrostMap = vec![(str_key("foo"), Value::from(1i64))]
        .into_iter()
        .collect();
    // Compact mode always uses [key]: syntax
    assert_eq!(Value::from(map).to_frost_string(), "{ [\"foo\"]: 1 }");
}

#[test]
fn map_compact_non_string_keys() {
    let map: FrostMap = vec![
        (MapKey::Bool(true), Value::from(1i64)),
        (MapKey::Int(42), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    let s = Value::from(map).to_frost_string();
    assert!(s.contains("[true]: 1"));
    assert!(s.contains("[42]: 2"));
}

#[test]
fn map_compact_bytes_key() {
    // A Bytes key renders in `x'..'` literal form, so it cannot be confused with
    // the String key spelling the same characters.
    let map: FrostMap = vec![(MapKey::from(vec![b'h', b'i']), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_eq!(Value::from(map).to_frost_string(), "{ [x'6869']: 1 }");

    let text: FrostMap = vec![(str_key("hi"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_eq!(Value::from(text).to_frost_string(), r#"{ ["hi"]: 1 }"#);
}

#[test]
fn map_compact_reserved_keyword_key() {
    let map: FrostMap = vec![(str_key("if"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_eq!(Value::from(map).to_frost_string(), "{ [\"if\"]: 1 }");
}

// -- Arrays: pretty --

#[test]
fn array_pretty() {
    let arr = FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]);
    assert_eq!(Value::from(arr).to_pretty_string(), "[\n    1,\n    2\n]");
}

#[test]
fn empty_array_pretty() {
    assert_eq!(
        Value::from(FrostArray::from(vec![])).to_pretty_string(),
        "[]"
    );
}

#[test]
fn nested_array_pretty() {
    let inner = FrostArray::from(vec![Value::from(2i64), Value::from(3i64)]);
    let outer = FrostArray::from(vec![Value::from(1i64), Value::from(inner)]);
    assert_eq!(
        Value::from(outer).to_pretty_string(),
        "\
[\n    1,\n    [\n        2,\n        3\n    ]\n]"
    );
}

// -- Maps: pretty --

#[test]
fn map_pretty_identifier_keys() {
    let map: FrostMap = vec![
        (str_key("a"), Value::from(1i64)),
        (str_key("b"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        Value::from(map).to_pretty_string(),
        "{\n    a: 1,\n    b: 2\n}"
    );
}

#[test]
fn map_pretty_keyword_key_uses_brackets() {
    let map: FrostMap = vec![
        (str_key("if"), Value::from(1i64)),
        (str_key("ok"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    let s = Value::from(map).to_pretty_string();
    assert!(s.contains("[\"if\"]: 1"));
    assert!(s.contains("ok: 2"));
}

#[test]
fn map_pretty_non_string_key_uses_brackets() {
    let map: FrostMap = vec![(MapKey::Bool(true), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_eq!(Value::from(map).to_pretty_string(), "{\n    [true]: 1\n}");
}

#[test]
fn map_pretty_non_identifier_string_key() {
    let map: FrostMap = vec![(str_key("not-valid"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert!(
        Value::from(map)
            .to_pretty_string()
            .contains("[\"not-valid\"]: 1")
    );
}

#[test]
fn empty_map_pretty() {
    assert_eq!(Value::from(FrostMap::empty()).to_pretty_string(), "{}");
}

// -- Nested structures: pretty --

#[test]
fn map_with_nested_array_pretty() {
    let arr = FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]);
    let map: FrostMap = vec![(str_key("nums"), Value::from(arr))]
        .into_iter()
        .collect();
    assert_eq!(
        Value::from(map).to_pretty_string(),
        "{\n    nums: [\n        1,\n        2\n    ]\n}"
    );
}

// -- Strings inside structures --

#[test]
fn string_in_array_is_quoted() {
    let arr = FrostArray::from(vec![Value::from("hello")]);
    assert_eq!(Value::from(arr).to_frost_string(), "[ \"hello\" ]");
}

#[test]
fn string_with_escapes_in_array() {
    let arr = FrostArray::from(vec![Value::from("line1\nline2\t\"x\"")]);
    assert_eq!(
        Value::from(arr).to_frost_string(),
        "[ \"line1\\nline2\\t\\\"x\\\"\" ]"
    );
}

// -- Float map key --

#[test]
fn map_compact_float_key() {
    let map: FrostMap = vec![(
        MapKey::Float(FrostFloat::new(3.14).unwrap()),
        Value::from("pi"),
    )]
    .into_iter()
    .collect();
    assert_eq!(Value::from(map).to_frost_string(), "{ [3.14]: \"pi\" }");
}
