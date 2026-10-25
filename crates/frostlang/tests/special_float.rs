//! [`SpecialFloat`]: the Opaque that carries a NaN or infinite float through Frost.
//!
//! A `Value` takes a non-finite float as a `SpecialFloat` and gives it back as the
//! same float, bit for bit. To the serde bridge an `f64` is finite, so a non-finite
//! float crosses it only inside a `Value`.

use crate::script;

use std::collections::BTreeMap;
use std::fmt::Debug;

use serde::de::value::{Error as PlainError, F64Deserializer};
use serde::de::{DeserializeOwned, IntoDeserializer};
use serde::{Deserialize, Serialize};

use frostlang::{FrostFloat, Nullable, SpecialFloat, Value, from_value, to_value};
use script::{Script, run};

/// A NaN with a payload, and with the sign bit set: neither is `f64::NAN`'s bits.
const ODD_NAN: f64 = f64::from_bits(0xfff8_0000_0000_beef);

/// Every kind of non-finite float, NaN in more than one encoding.
const NON_FINITE: [f64; 4] = [f64::NAN, ODD_NAN, f64::INFINITY, f64::NEG_INFINITY];

/// The float a `SpecialFloat` Opaque holds; panics if `value` is anything else.
fn held(value: &Value) -> f64 {
    value
        .downcast_opaque::<SpecialFloat>()
        .unwrap_or_else(|| panic!("expected a SpecialFloat, got {value:?}"))
        .get()
}

/// A `SpecialFloat` Opaque holding `f`, which must be non-finite.
fn special(f: f64) -> Value {
    Value::opaque(SpecialFloat::new(f).unwrap())
}

fn assert_same_bits(actual: f64, expected: f64) {
    assert_eq!(
        actual.to_bits(),
        expected.to_bits(),
        "{actual} is not bit for bit {expected}"
    );
}

// -- The type --

#[test]
fn new_wraps_a_non_finite_float() {
    for f in NON_FINITE {
        let special = SpecialFloat::new(f).unwrap_or_else(|| panic!("{f} is non-finite"));
        assert_same_bits(special.get(), f);
    }
}

#[test]
fn new_refuses_a_finite_float() {
    for f in [
        0.0,
        -0.0,
        1.5,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        5e-324,
    ] {
        assert!(SpecialFloat::new(f).is_none(), "{f} is finite");
    }
}

#[test]
fn renders_as_its_type_and_float() {
    for (f, rendered) in [
        (f64::NAN, "<SpecialFloat: NaN>"),
        (f64::INFINITY, "<SpecialFloat: inf>"),
        (f64::NEG_INFINITY, "<SpecialFloat: -inf>"),
    ] {
        assert_eq!(special(f).to_frost_string(), rendered);
    }
}

#[test]
fn equals_another_with_the_same_bits() {
    for f in NON_FINITE {
        assert_eq!(special(f), special(f), "{f}");
    }
}

#[test]
fn differs_from_another_with_different_bits() {
    assert_ne!(special(f64::INFINITY), special(f64::NEG_INFINITY));
    assert_ne!(special(f64::NAN), special(ODD_NAN));
    assert_ne!(special(f64::NAN), special(f64::INFINITY));
}

#[test]
fn frost_compares_it_with_equals() {
    let compared = Script::new("[inf == also_inf, inf == nan, inf != nan]")
        .captures(&[
            ("inf", special(f64::INFINITY)),
            ("also_inf", special(f64::INFINITY)),
            ("nan", special(f64::NAN)),
        ])
        .run();
    assert_eq!(compared, run("[true, false, true]"));
}

// -- Into a Value --

#[test]
fn from_f64_makes_a_non_finite_float_a_special_float() {
    for f in NON_FINITE {
        assert_same_bits(held(&Value::from_f64(f)), f);
    }
}

#[test]
fn from_f64_keeps_a_finite_float_a_float() {
    for f in [0.0, -0.0, 1.5, f64::MAX, f64::MIN, 5e-324] {
        match Value::from_f64(f) {
            Value::Float(float) => assert_same_bits(float.get(), f),
            other => panic!("{f} should be a Float, got {other:?}"),
        }
    }
}

#[test]
fn deserializing_a_non_finite_float_makes_a_special_float() {
    // A foreign deserializer presenting a float, as a format decoder would.
    for f in NON_FINITE {
        let deserializer: F64Deserializer<PlainError> = f.into_deserializer();
        assert_same_bits(held(&Value::deserialize(deserializer).unwrap()), f);
    }
}

// -- Out of a Value --

#[test]
fn a_foreign_serializer_gets_the_float() {
    for f in NON_FINITE {
        assert_eq!(
            postcard::to_allocvec(&special(f)).unwrap(),
            postcard::to_allocvec(&f).unwrap(),
            "{f} serializes as the float it is"
        );
    }
}

// -- Through the serde bridge --

/// The error message of serializing `value`, which must fail.
fn serialize_error<T: Serialize>(value: &T) -> String {
    to_value(value)
        .expect_err("serializing fails")
        .message()
        .into_owned()
}

/// The error message of deserializing `value` as `T`, which must fail.
fn deserialize_error<T: DeserializeOwned + Debug>(value: Value) -> String {
    from_value::<T>(value)
        .expect_err("deserializing fails")
        .message()
        .into_owned()
}

#[test]
fn to_value_refuses_a_non_finite_float() {
    for f in NON_FINITE {
        assert_eq!(serialize_error(&f), format!("{f} is not finite"));
    }
    for f in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(serialize_error(&f), format!("{f} is not finite"), "an f32");
    }
}

#[test]
fn to_value_refuses_a_nested_non_finite_float() {
    #[derive(Serialize)]
    struct Fields {
        level: f64,
    }
    for (case, message) in [
        ("an element", serialize_error(&vec![1.0, f64::INFINITY])),
        (
            "a field",
            serialize_error(&Fields {
                level: f64::INFINITY,
            }),
        ),
        (
            "a Map entry",
            serialize_error(&BTreeMap::from([("level", f64::INFINITY)])),
        ),
        ("an Option", serialize_error(&Some(f64::INFINITY))),
        (
            "a Nullable",
            serialize_error(&Nullable(Some(f64::INFINITY))),
        ),
    ] {
        assert_eq!(message, "inf is not finite", "{case}");
    }
}

#[test]
fn to_value_keeps_a_finite_float_a_float() {
    for f in [0.0, -0.0, 1.5, f64::MAX, f64::MIN, 5e-324] {
        match to_value(&f).unwrap() {
            Value::Float(float) => assert_same_bits(float.get(), f),
            other => panic!("{f} should be a Float, got {other:?}"),
        }
    }
}

#[test]
fn from_value_refuses_it_for_a_float() {
    for f in NON_FINITE {
        assert_eq!(
            deserialize_error::<f64>(special(f)),
            "expected Float, got Opaque",
            "{f}"
        );
        assert_eq!(
            deserialize_error::<f32>(special(f)),
            "expected Float, got Opaque",
            "{f} for an f32"
        );
    }
}

#[test]
fn from_value_refuses_it_for_a_nested_float() {
    // An `Option` and a `Nullable` each reach the float by a route of their own.
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Fields {
        level: f64,
    }
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct OptionalFields {
        level: Option<f64>,
    }
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct NullableFields {
        level: Nullable<f64>,
    }
    let infinity = || special(f64::INFINITY);
    let one = || Value::Float(FrostFloat::new(1.0).unwrap());
    let fields = || Value::map([("level", infinity())]);
    for (case, message, expected) in [
        (
            "an element",
            deserialize_error::<Vec<f64>>(Value::array([one(), infinity()])),
            "expected Float, got Opaque (at `[1]`)",
        ),
        (
            "a field",
            deserialize_error::<Fields>(fields()),
            "expected Float, got Opaque (at `level`)",
        ),
        (
            "an Option field",
            deserialize_error::<OptionalFields>(fields()),
            "expected Float, got Opaque (at `level`)",
        ),
        (
            "a Nullable field",
            deserialize_error::<NullableFields>(fields()),
            "expected Float, got Opaque (at `level`)",
        ),
        (
            "a Map entry",
            deserialize_error::<BTreeMap<String, f64>>(fields()),
            "expected Float, got Opaque (at `level`)",
        ),
        (
            "a tuple's element",
            deserialize_error::<(f64, f64)>(Value::array([one(), infinity()])),
            "expected Float, got Opaque (at `[1]`)",
        ),
    ] {
        assert_eq!(message, expected, "{case}");
    }
}

#[test]
fn a_tuple_target_refuses_it_as_an_opaque() {
    // Not as the float it holds: only an Array is a tuple.
    assert_eq!(
        deserialize_error::<(f64, f64)>(special(f64::INFINITY)),
        "expected Array, got Opaque"
    );
}

#[test]
fn a_value_carries_it_through_the_bridge() {
    #[derive(Serialize, Deserialize)]
    struct Reading {
        level: Value,
    }
    for f in NON_FINITE {
        assert_same_bits(held(&to_value(&special(f)).unwrap()), f);
        assert_same_bits(held(&from_value::<Value>(special(f)).unwrap()), f);

        let value = to_value(&Reading { level: special(f) }).unwrap();
        let reading: Reading = from_value(value).unwrap();
        assert_same_bits(held(&reading.level), f);
    }
}

// -- serde's buffered representations --
//
// serde reads these into a buffer of its own, which holds a SpecialFloat as the
// float it carries, as a foreign serializer sees it. A `Value` read back out of
// the buffer is the SpecialFloat again, and an `f64` takes the float.

#[test]
fn a_value_in_a_buffered_representation_keeps_it() {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Reading { level: Value },
    }
    let level = Value::map([("level", special(f64::NEG_INFINITY))]);
    let Untagged::Reading { level } = from_value(level).unwrap();
    assert_same_bits(held(&level), f64::NEG_INFINITY);
}

#[test]
fn a_float_in_a_buffered_representation_takes_the_float() {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Float(f64),
    }
    let Untagged::Float(f) = from_value(special(f64::NEG_INFINITY)).unwrap();
    assert_same_bits(f, f64::NEG_INFINITY);

    #[derive(Deserialize)]
    #[serde(tag = "type")]
    enum Tagged {
        Fields { level: f64 },
    }
    let Tagged::Fields { level } = from_value(Value::map([
        ("type", Value::from("Fields")),
        ("level", special(f64::NEG_INFINITY)),
    ]))
    .unwrap();
    assert_same_bits(level, f64::NEG_INFINITY);

    #[derive(Deserialize)]
    struct Flattened {
        #[serde(flatten)]
        inner: Inner,
    }
    #[derive(Deserialize)]
    struct Inner {
        level: f64,
    }
    let flattened: Flattened =
        from_value(Value::map([("level", special(f64::NEG_INFINITY))])).unwrap();
    assert_same_bits(flattened.inner.level, f64::NEG_INFINITY);
}

#[test]
fn an_f32_in_a_buffered_representation_rounds_a_float_beyond_its_range() {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Float(f32),
    }
    let huge = Value::Float(FrostFloat::new(1e39).unwrap());
    let Untagged::Float(f) = from_value(huge).unwrap();
    assert_eq!(
        f,
        f32::INFINITY,
        "1e39 rounds to an infinity, as a cast does"
    );
}
