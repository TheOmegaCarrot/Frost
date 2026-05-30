use std::sync::Arc;

use frost_runtime::{FrostArray, FrostFloat, FrostMap, MapKey, Value};

fn str_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s.as_bytes()))
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
fn string_top_level_non_utf8_falls_back_to_escaped() {
    let v: Value = vec![0x80u8, 0xff].into();
    assert_eq!(v.to_frost_string(), "\"\\x80\\xff\"");
}

#[test]
fn function_placeholder() {
    // Can't easily construct a Function value in tests, tested via type_name coverage
}

#[test]
fn opaque_placeholder() {
    // Can't easily construct an Opaque value in tests, tested via type_name coverage
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
fn debug_string_hex_escapes_null_byte() {
    assert_eq!(Value::from("\x00").to_debug_string(), "\"\\x00\"");
}

#[test]
fn debug_string_hex_escapes_high_bytes() {
    let v: Value = vec![0x80u8, 0xff].into();
    assert_eq!(v.to_debug_string(), "\"\\x80\\xff\"");
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
    let v = Value::from(FrostArray::new(&[]));
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
    let arr = FrostArray::new(&[Value::from(1i64), Value::from("hi"), Value::Null]);
    assert_eq!(Value::from(arr).to_frost_string(), "[ 1, \"hi\", null ]");
}

#[test]
fn array_single_element() {
    let arr = FrostArray::new(&[Value::from(42i64)]);
    assert_eq!(Value::from(arr).to_frost_string(), "[ 42 ]");
}

#[test]
fn nested_array_compact() {
    let inner = FrostArray::new(&[Value::from(2i64), Value::from(3i64)]);
    let outer = FrostArray::new(&[Value::from(1i64), Value::from(inner)]);
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
fn map_compact_reserved_keyword_key() {
    let map: FrostMap = vec![(str_key("if"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_eq!(Value::from(map).to_frost_string(), "{ [\"if\"]: 1 }");
}

// -- Arrays: pretty --

#[test]
fn array_pretty() {
    let arr = FrostArray::new(&[Value::from(1i64), Value::from(2i64)]);
    assert_eq!(Value::from(arr).to_pretty_string(), "[\n    1,\n    2\n]");
}

#[test]
fn empty_array_pretty() {
    assert_eq!(Value::from(FrostArray::new(&[])).to_pretty_string(), "[]");
}

#[test]
fn nested_array_pretty() {
    let inner = FrostArray::new(&[Value::from(2i64), Value::from(3i64)]);
    let outer = FrostArray::new(&[Value::from(1i64), Value::from(inner)]);
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
    let arr = FrostArray::new(&[Value::from(1i64), Value::from(2i64)]);
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
    let arr = FrostArray::new(&[Value::from("hello")]);
    assert_eq!(Value::from(arr).to_frost_string(), "[ \"hello\" ]");
}

#[test]
fn string_with_escapes_in_array() {
    let arr = FrostArray::new(&[Value::from("line1\nline2\t\"x\"")]);
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
