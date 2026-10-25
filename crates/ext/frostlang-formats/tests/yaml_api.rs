//! YAML from Rust: `yaml::encode` and `yaml::decode` as a host calls them.
//! The behavior they share with scripts is tested through scripts, in
//! `yaml.rs`; these cover what only Rust sees: the error types.

use frostlang::{FrostArray, Value, frost_closure};
use frostlang_formats::yaml;
use frostlang_formats::{DecodeError, EncodeError};

// --- encode and decode ---

#[test]
fn encode_gives_yaml_text() {
    let value = Value::map([(
        "a",
        Value::from(FrostArray::from(vec![
            Value::Int(1),
            Value::from_f64(2.5),
            Value::from("x"),
        ])),
    )]);
    let text = yaml::encode(&value).unwrap();
    assert_eq!(text, "a:\n- 1\n- 2.5\n- x\n");
    assert_eq!(yaml::decode(&text).unwrap(), value);
}

#[test]
fn a_refused_value_says_what_and_where() {
    let value = Value::map([(
        "a",
        Value::from(FrostArray::from(vec![
            Value::Int(1),
            Value::from(&[0u8][..]),
        ])),
    )]);
    let error = yaml::encode(&value).expect_err("YAML has no form for Bytes");
    assert_eq!(
        error,
        EncodeError::Unrepresentable {
            format: "YAML",
            what: "Bytes".to_string(),
            path: "a[1]".to_string(),
        }
    );
    assert_eq!(error.to_string(), "cannot encode Bytes in YAML (at `a[1]`)");
}

#[test]
fn a_refused_whole_value_has_an_empty_path() {
    let function = frost_closure!("f", || Value::Null);
    assert_eq!(
        yaml::encode(&function).expect_err("YAML has no form for a Function"),
        EncodeError::Unrepresentable {
            format: "YAML",
            what: "Function".to_string(),
            path: String::new(),
        }
    );
}

#[test]
fn nesting_past_the_limit_is_too_deep() {
    let nested = |depth| {
        (1..depth).fold(Value::from(FrostArray::from(vec![])), |inner, _| {
            Value::from(FrostArray::from(vec![inner]))
        })
    };
    let deepest = nested(64);
    assert_eq!(
        yaml::decode(&yaml::encode(&deepest).unwrap()).unwrap(),
        deepest
    );
    assert_eq!(
        yaml::encode(&nested(65)).expect_err("65 is past the limit"),
        EncodeError::TooDeep {
            format: "YAML",
            limit: 64,
        }
    );
}

#[test]
fn a_decode_error_names_the_format_the_problem_and_where() {
    let error: DecodeError = yaml::decode("a: 1\nb: [").expect_err("an unclosed bracket");
    assert_eq!(error.format(), "YAML");
    assert!(
        error.message().contains("line 2"),
        "{:?} says where",
        error.message()
    );
    assert_eq!(
        error.to_string(),
        format!("cannot decode YAML: {}", error.message())
    );
}
