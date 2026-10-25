//! MessagePack from Rust: `msgpack::encode` and `msgpack::decode` as a host
//! uses them. The behavior they share with scripts is tested through scripts,
//! in `msgpack.rs`; these cover what only Rust sees: the error types, and a
//! `Uint64` made in Rust.

use frostlang::{FrostArray, Value};
use frostlang_formats::msgpack;
use frostlang_formats::uint64::Uint64;
use frostlang_formats::{DecodeError, EncodeError};

use crate::common::uint64;

// --- encode and decode ---

#[test]
fn encode_gives_messagepack_data() {
    let value = Value::map([(
        "a",
        Value::from(FrostArray::from(vec![
            Value::Int(1),
            Value::from(&[0xffu8][..]),
        ])),
    )]);
    let data = msgpack::encode(&value).unwrap();
    assert_eq!(data, [0x81, 0xa1, b'a', 0x92, 0x01, 0xc4, 0x01, 0xff]);
    assert_eq!(msgpack::decode(&data).unwrap(), value);
}

#[test]
fn a_refused_value_says_what_and_where() {
    let function = frostlang::frost_closure!("f", || Value::Null);
    let error = msgpack::encode(&Value::map([("a", function)])).expect_err("a Function");
    assert_eq!(
        error,
        EncodeError::Unrepresentable {
            format: "MessagePack",
            what: "Function".to_string(),
            path: "a".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "cannot encode Function in MessagePack (at `a`)"
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
    assert_eq!(
        msgpack::decode(&msgpack::encode(&deepest).unwrap()).unwrap(),
        deepest
    );
    assert_eq!(
        msgpack::encode(&nested(128)).expect_err("128 is past the limit"),
        EncodeError::TooDeep {
            format: "MessagePack",
            limit: 127,
        }
    );
}

#[test]
fn a_decode_error_names_the_format_and_the_problem() {
    let error: DecodeError = msgpack::decode(&[0x01, 0x02]).expect_err("a trailing byte");
    assert_eq!(error.format(), "MessagePack");
    assert_eq!(error.message(), "1 byte after the value");
    assert_eq!(
        error.to_string(),
        "cannot decode MessagePack: 1 byte after the value"
    );
}

// --- Uint64 ---

#[test]
fn a_uint64_opaque_round_trips_as_a_messagepack_uint64() {
    let value = Value::opaque(uint64(u64::MAX));
    let data = msgpack::encode(&value).unwrap();
    assert_eq!(data, [0xcf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    let decoded = msgpack::decode(&data).unwrap();
    assert_eq!(decoded, value);
    assert_eq!(decoded.downcast_opaque::<Uint64>(), Some(&uint64(u64::MAX)));
}
