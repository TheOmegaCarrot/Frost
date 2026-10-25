//! The encoder refuses a String, Bytes, Array, Map, or Map key longer than
//! its mapping's `max_len`, here [`SHORT`]'s.

use frostlang::{FrostArray, FrostType, MapKey, TypeSet, Value, to_value};
use serde::ser::{Error, Serialize, Serializer};

use crate::mapping::encode::{EncodeFailure, encode};
use crate::mapping::{BeyondInt, Mapping};

/// Every type but Function and Opaque.
const TYPES: TypeSet = FrostType::Null
    .or(FrostType::Bool)
    .or(FrostType::Int)
    .or(FrostType::Float)
    .or(FrostType::String)
    .or(FrostType::Bytes)
    .or(FrostType::Array)
    .or(FrostType::Map);

/// A mapping with a form for every value but Functions and Opaques, and a
/// length limit of 2.
const SHORT: Mapping = Mapping {
    name: "Short",
    document_types: TYPES,
    types: TYPES,
    key_types: MapKey::TYPES,
    opaques: &[],
    max_depth: 8,
    max_len: 2,
    beyond_int: BeyondInt::Refused,
    marked: &[],
};

/// A value encoded by [`SHORT`], through any serializer.
struct EncodedByShort<'a>(&'a Value);

impl Serialize for EncodedByShort<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        encode(&SHORT, self.0, serializer).map_err(|failure| match failure {
            EncodeFailure::Refused(refusal) => S::Error::custom(refusal),
            EncodeFailure::Serializer(err) => err,
        })
    }
}

/// `value`, encoded by [`SHORT`] into a Frost value. The core serde bridge
/// serves as a serializer that takes anything, so any refusal is the
/// encoder's.
fn encoded(value: &Value) -> Result<Value, String> {
    to_value(&EncodedByShort(value)).map_err(|err| err.message().into_owned())
}

/// The message refusing `value`, which must be refused.
fn refusal(value: &Value) -> String {
    match encoded(value) {
        Ok(_) => panic!("{value:?} should be refused"),
        Err(message) => message,
    }
}

/// An Array of `len` Nulls.
fn array(len: usize) -> Value {
    Value::from(FrostArray::from(vec![Value::Null; len]))
}

#[test]
fn values_as_long_as_the_limit_are_encoded() {
    for value in [
        Value::from("ab"),
        Value::from(&b"ab"[..]),
        array(2),
        Value::map([("a", Value::Null), ("b", Value::Null)]),
        Value::map([("ab", Value::Null)]),
    ] {
        if let Err(message) = encoded(&value) {
            panic!("{value:?} should be encoded, but: {message}");
        }
    }
}

#[test]
fn values_longer_than_the_limit_are_refused() {
    assert_eq!(
        refusal(&Value::from("abc")),
        "cannot encode String of length 3 in Short"
    );
    assert_eq!(
        refusal(&Value::from(&b"abc"[..])),
        "cannot encode Bytes of length 3 in Short"
    );
    assert_eq!(
        refusal(&array(3)),
        "cannot encode Array of length 3 in Short"
    );
    assert_eq!(
        refusal(&Value::map([
            ("a", Value::Null),
            ("b", Value::Null),
            ("c", Value::Null)
        ])),
        "cannot encode Map of length 3 in Short"
    );
}

#[test]
fn keys_longer_than_the_limit_are_refused() {
    assert_eq!(
        refusal(&Value::map([("abc", Value::Null)])),
        "cannot encode a String key of length 3 in Short"
    );
    assert_eq!(
        refusal(&Value::map([(
            MapKey::Bytes(b"abc"[..].into()),
            Value::Null
        )])),
        "cannot encode a Bytes key of length 3 in Short"
    );
}

#[test]
fn a_refusal_within_a_value_says_where() {
    assert_eq!(
        refusal(&Value::map([("a", array(3))])),
        "cannot encode Array of length 3 in Short (at `a`)"
    );
}
