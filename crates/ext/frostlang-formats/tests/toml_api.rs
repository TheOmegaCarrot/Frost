//! TOML from Rust: `toml::encode`, `toml::decode`, and `toml::Datetime` as a
//! host uses them. The behavior they share with scripts is tested through
//! scripts, in `toml.rs`; these cover what only Rust sees: the options, the
//! error types, and `Datetime` as a Rust type.

use frostlang::{FrostArray, Value};
use frostlang_formats::toml::{self, Datetime, EncodeOptions};
use frostlang_formats::{DecodeError, EncodeError};

use crate::toml::{DATETIMES, NOT_DATETIMES};

/// `document` encoded with the default options.
fn default_text(document: &Value) -> Result<String, EncodeError> {
    toml::encode(document, EncodeOptions::default())
}

/// The datetime `text` spells, which must be one.
fn datetime(text: &str) -> Datetime {
    text.parse()
        .unwrap_or_else(|err| panic!("{text:?} is a TOML datetime, but {err}"))
}

// --- options ---

#[test]
fn pretty_puts_each_array_element_on_its_own_line() {
    let document = Value::map([(
        "a",
        Value::from(FrostArray::from(vec![Value::Int(1), Value::Int(2)])),
    )]);
    assert_eq!(default_text(&document).unwrap(), "a = [1, 2]\n");
    assert_eq!(
        toml::encode(&document, EncodeOptions { pretty: true }).unwrap(),
        r"a = [
    1,
    2,
]
"
    );
}

// --- encode errors ---

#[test]
fn a_document_that_is_not_a_map_is_refused() {
    let error = default_text(&Value::Int(1)).expect_err("a document is a Map");
    assert_eq!(
        error,
        EncodeError::NotADocument {
            format: "TOML",
            what: "Int".to_string(),
        }
    );
    assert_eq!(error.to_string(), "cannot encode Int as a TOML document");
}

#[test]
fn a_refused_value_says_what_and_where() {
    let document = Value::map([("a", Value::map([("b", Value::Null)]))]);
    let error = default_text(&document).expect_err("TOML has no Null");
    assert_eq!(
        error,
        EncodeError::Unrepresentable {
            format: "TOML",
            what: "Null".to_string(),
            path: "a.b".to_string(),
        }
    );
    assert_eq!(error.to_string(), "cannot encode Null in TOML (at `a.b`)");
}

#[test]
fn nesting_past_the_limit_is_too_deep() {
    let nested = |depth| {
        (1..depth).fold(Value::map([("a", Value::Int(1))]), |inner, _| {
            Value::map([("a", inner)])
        })
    };
    let deepest = nested(81);
    assert_eq!(
        toml::decode(&default_text(&deepest).unwrap()).unwrap(),
        deepest
    );
    assert_eq!(
        default_text(&nested(82)).expect_err("82 is past the limit"),
        EncodeError::TooDeep {
            format: "TOML",
            limit: 81,
        }
    );
}

// --- decode ---

#[test]
fn decode_gives_the_document() {
    assert_eq!(
        toml::decode("a = 1\n[t]\nb = 'x'").unwrap(),
        Value::map([
            ("a", Value::Int(1)),
            ("t", Value::map([("b", Value::from("x"))])),
        ])
    );
}

#[test]
fn a_decode_error_names_the_format_the_problem_and_where() {
    let error: DecodeError = toml::decode("a = 1\nb = [1,,]").expect_err("malformed");
    assert_eq!(error.format(), "TOML");
    assert!(
        error.message().ends_with("at line 2 column 8"),
        "{:?} says where",
        error.message()
    );
    assert_eq!(
        error.to_string(),
        format!("cannot decode TOML: {}", error.message())
    );
}

#[test]
fn an_integer_beyond_the_int_range_is_a_decode_error() {
    let error = toml::decode("a = 9223372036854775808").expect_err("beyond i64");
    assert_eq!(
        error.message(),
        "integer 9223372036854775808 is beyond the Int range at line 1 column 5"
    );
}

// --- Datetime ---

#[test]
fn datetime_parses_each_kind_and_displays_its_text() {
    for text in DATETIMES {
        assert_eq!(datetime(text).to_string(), *text);
    }
}

#[test]
fn datetime_parses_nothing_else() {
    for text in NOT_DATETIMES {
        let error = text.parse::<Datetime>().expect_err("not a datetime");
        assert_eq!(
            error.to_string(),
            format!("not a TOML date, time, or datetime: {text:?}")
        );
    }
}

#[test]
fn datetimes_are_equal_by_their_text() {
    assert_eq!(datetime("1979-05-27"), datetime("1979-05-27"));
    assert_ne!(datetime("1979-05-27"), datetime("1979-05-28"));
    assert_eq!(
        Value::opaque(datetime("1979-05-27")),
        Value::opaque(datetime("1979-05-27")),
        "as Opaques too"
    );
}

#[test]
fn a_datetime_opaque_round_trips() {
    let document = Value::map([("when", Value::opaque(datetime("1979-05-27T07:32:00Z")))]);
    let text = default_text(&document).unwrap();
    assert_eq!(text, "when = 1979-05-27T07:32:00Z\n");
    let decoded = toml::decode(&text).unwrap();
    assert_eq!(decoded, document);
    let Value::Map(map) = decoded else {
        panic!("a document is a Map")
    };
    let when = map.get(&"when".into()).expect("the entry is there");
    assert_eq!(
        when.downcast_opaque::<Datetime>(),
        Some(&datetime("1979-05-27T07:32:00Z"))
    );
}
