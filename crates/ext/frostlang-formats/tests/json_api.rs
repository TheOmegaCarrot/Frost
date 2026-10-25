//! JSON from Rust: `json::encode` and `json::decode` as a host calls them.
//! The behavior they share with scripts is tested through scripts, in
//! `json.rs`; these cover what only Rust sees: the options and error types.

use frostlang::{FrostArray, Value, frost_closure};
use frostlang_formats::json::{self, EncodeOptions, Indent};
use frostlang_formats::{DecodeError, EncodeError};

/// Compact text, as the default options give.
fn compact(value: &Value) -> Result<String, EncodeError> {
    json::encode(value, EncodeOptions::default())
}

/// `{a: [1, x'00']}`, whose Bytes JSON has no form for.
fn bytes_at_a_1() -> Value {
    Value::map([(
        "a",
        Value::from(FrostArray::from(vec![
            Value::Int(1),
            Value::from(&[0u8][..]),
        ])),
    )])
}

// --- options ---

#[test]
fn the_default_options_give_compact_text() {
    let value = Value::map([("a", Value::from(FrostArray::from(vec![Value::Int(1)])))]);
    assert_eq!(compact(&value).unwrap(), r#"{"a":[1]}"#);
}

#[test]
fn pretty_text_indents_each_level() {
    let value = Value::map([("a", Value::from(FrostArray::from(vec![Value::Int(1)])))]);
    let pretty = |spaces| EncodeOptions {
        pretty: Some(Indent::new(spaces).expect("within the maximum")),
    };
    assert_eq!(
        json::encode(&value, pretty(2)).unwrap(),
        "{\n  \"a\": [\n    1\n  ]\n}"
    );
    assert_eq!(
        json::encode(&value, pretty(0)).unwrap(),
        "{\n\"a\": [\n1\n]\n}"
    );
}

#[test]
fn an_indent_is_at_most_16_spaces() {
    assert_eq!(Indent::MAX, 16);
    assert_eq!(Indent::new(0).map(Indent::spaces), Some(0));
    assert_eq!(Indent::new(16).map(Indent::spaces), Some(16));
    assert_eq!(Indent::new(17), None, "17 is beyond the maximum");
    assert_eq!(Indent::new(u8::MAX), None);
}

#[test]
fn the_default_indent_is_2_spaces() {
    assert_eq!(Indent::DEFAULT.spaces(), 2);
    assert_eq!(Indent::default(), Indent::DEFAULT);
}

// --- encode errors ---

#[test]
fn a_refused_value_says_what_and_where() {
    let error = compact(&bytes_at_a_1()).expect_err("JSON has no form for Bytes");
    assert_eq!(
        error,
        EncodeError::Unrepresentable {
            format: "JSON",
            what: "Bytes".to_string(),
            path: "a[1]".to_string(),
        }
    );
    assert_eq!(error.to_string(), "cannot encode Bytes in JSON (at `a[1]`)");
}

#[test]
fn a_refused_whole_value_has_an_empty_path() {
    let function = frost_closure!("f", || Value::Null);
    let error = compact(&function).expect_err("JSON has no form for a Function");
    assert_eq!(
        error,
        EncodeError::Unrepresentable {
            format: "JSON",
            what: "Function".to_string(),
            path: String::new(),
        }
    );
    assert_eq!(error.to_string(), "cannot encode Function in JSON");
}

#[test]
fn a_refused_key_says_what_and_where() {
    let value = Value::map([("a", Value::map([(1, Value::Null)]))]);
    assert_eq!(
        compact(&value).expect_err("JSON keys are Strings"),
        EncodeError::Unrepresentable {
            format: "JSON",
            what: "the Int key 1".to_string(),
            path: "a".to_string(),
        }
    );
}

#[test]
fn a_special_float_is_refused() {
    let error = compact(&Value::from_f64(f64::NAN)).expect_err("JSON has no NaN");
    assert_eq!(
        error.to_string(),
        "cannot encode <SpecialFloat: NaN> in JSON"
    );
}

#[test]
fn nesting_past_the_limit_is_too_deep() {
    let nested = |depth| {
        (1..depth).fold(Value::from(FrostArray::from(vec![])), |inner, _| {
            Value::from(FrostArray::from(vec![inner]))
        })
    };
    let deepest = nested(127);
    assert_eq!(json::decode(&compact(&deepest).unwrap()).unwrap(), deepest);
    let error = compact(&nested(128)).expect_err("128 is past the limit");
    assert_eq!(
        error,
        EncodeError::TooDeep {
            format: "JSON",
            limit: 127,
        }
    );
    assert_eq!(
        error.to_string(),
        "cannot encode Arrays and Maps nested more than 127 deep in JSON"
    );
}

// --- decode ---

#[test]
fn decode_gives_the_value() {
    assert_eq!(
        json::decode(r#"{"a": [1, 2.5, "x", null, true]}"#).unwrap(),
        Value::map([(
            "a",
            Value::from(FrostArray::from(vec![
                Value::Int(1),
                Value::from_f64(2.5),
                Value::from("x"),
                Value::Null,
                Value::Bool(true),
            ])),
        )])
    );
}

#[test]
fn decode_skips_a_leading_byte_order_mark() {
    assert_eq!(json::decode("\u{feff}1").unwrap(), Value::Int(1));
}

#[test]
fn a_decode_error_names_the_format_and_the_problem() {
    let error: DecodeError = json::decode(r#"{"a": 1, "a": 2}"#).expect_err("a key twice");
    assert_eq!(error.format(), "JSON");
    assert!(
        error.message().starts_with(r#"duplicate key "a""#),
        "{:?} names the key",
        error.message()
    );
    assert_eq!(
        error.to_string(),
        format!("cannot decode JSON: {}", error.message())
    );
}

#[test]
fn malformed_text_is_a_decode_error() {
    for text in ["", "{", "[1,]", "1 2", "NaN"] {
        let error = json::decode(text).expect_err("malformed");
        assert_eq!(error.format(), "JSON", "{text:?}");
        assert!(!error.message().is_empty(), "{text:?} says why");
    }
}

// --- round trips ---

#[test]
fn decoding_what_encode_gives_gives_the_value_encoded() {
    let value = Value::map([
        ("int", Value::Int(i64::MIN)),
        ("float", Value::from_f64(-0.0)),
        ("text", Value::from("caf\u{e9}")),
        (
            "list",
            Value::from(FrostArray::from(vec![Value::Null, Value::Bool(false)])),
        ),
    ]);
    for options in [
        EncodeOptions::default(),
        EncodeOptions {
            pretty: Some(Indent::DEFAULT),
        },
    ] {
        let text = json::encode(&value, options).unwrap();
        assert_eq!(json::decode(&text).unwrap(), value, "{text}");
    }
}
