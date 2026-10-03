//! [`SpecialFloat`]: the Opaque that carries a NaN or infinite float through Frost.
//!
//! Every way serde brings a float into a `Value` makes a non-finite one a
//! `SpecialFloat`, and every way out turns it back into the same float, bit for bit.

mod source;

use serde::de::IntoDeserializer;
use serde::de::value::{Error as PlainError, F64Deserializer};
use serde::{Deserialize, Serialize};

use frost_runtime::{SpecialFloat, Value, from_value, to_value};
use source::{Script, run};

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
fn to_value_makes_a_non_finite_float_a_special_float() {
    for f in NON_FINITE {
        assert_same_bits(held(&to_value(&f).unwrap()), f);
    }
}

#[test]
fn to_value_makes_a_non_finite_f32_a_special_float() {
    for f in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_same_bits(held(&to_value(&f).unwrap()), f64::from(f));
    }
}

#[test]
fn to_value_keeps_a_finite_float_a_float() {
    assert!(matches!(to_value(&1.5f64).unwrap(), Value::Float(_)));
}

#[test]
fn deserializing_a_non_finite_float_makes_a_special_float() {
    // A foreign deserializer presenting a float, as a format decoder would.
    for f in NON_FINITE {
        let deserializer: F64Deserializer<PlainError> = f.into_deserializer();
        assert_same_bits(held(&Value::deserialize(deserializer).unwrap()), f);
    }
}

#[test]
fn a_nested_non_finite_float_becomes_a_special_float() {
    let value = to_value(&vec![1.0, f64::INFINITY]).unwrap();
    let elements = value.as_array().expect("a Vec is an Array");
    assert!(matches!(elements[0], Value::Float(_)));
    assert_same_bits(held(&elements[1]), f64::INFINITY);
}

// -- Out of a Value --

#[test]
fn from_value_returns_the_float_it_holds() {
    for f in NON_FINITE {
        assert_same_bits(from_value::<f64>(special(f)).unwrap(), f);
    }
}

#[test]
fn from_value_returns_it_to_a_self_describing_target() {
    // An untagged enum reads its input through `deserialize_any`.
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Number {
        Float(f64),
    }

    let Number::Float(f) = from_value(special(f64::NEG_INFINITY)).unwrap();
    assert_same_bits(f, f64::NEG_INFINITY);
}

#[test]
fn a_struct_field_round_trips_through_a_value() {
    #[derive(Serialize, Deserialize)]
    struct Reading {
        level: f64,
    }

    for f in NON_FINITE {
        let value = to_value(&Reading { level: f }).unwrap();
        let reading: Reading = from_value(value).unwrap();
        assert_same_bits(reading.level, f);
    }
}

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
