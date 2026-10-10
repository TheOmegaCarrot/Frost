//! Returning Rust types from a native, with `frostlang::native`'s
//! `IntoNativeResult`: the value each type returns, and the errors for a
//! result no value can hold.

use std::collections::BTreeMap;

use frostlang::native::{IntoNativeResult, Ser};
use frostlang::{
    FrostArray, FrostBytes, FrostError, FrostFloat, FrostMap, FrostString, Value, ValueMap,
};
use serde::Serialize;

/// `result` as returned by a native named `f`, which must succeed.
fn returned(result: impl IntoNativeResult) -> Value {
    result
        .into_result("f")
        .unwrap_or_else(|err| panic!("the result converts, but: {err}"))
}

/// The error message of returning `result` from a native named `f`, which
/// must fail.
fn rejection(result: impl IntoNativeResult) -> String {
    match result.into_result("f") {
        Ok(value) => panic!("the result is rejected, but converts to {value:?}"),
        Err(err) => err.message().into_owned(),
    }
}

#[test]
fn a_value_returns_as_is() {
    for value in [Value::Null, Value::Int(1), Value::array([1, 2])] {
        assert_eq!(returned(value.clone()), value);
    }
}

#[test]
fn unit_returns_null() {
    assert_eq!(returned(()), Value::Null);
}

#[test]
fn each_type_returns_its_value() {
    assert_eq!(returned(true), Value::Bool(true));
    assert_eq!(
        returned(FrostFloat::new(2.5).unwrap()),
        Value::Float(FrostFloat::new(2.5).unwrap())
    );
    assert_eq!(returned("text".to_string()), Value::from("text"));
    assert_eq!(returned("text"), Value::from("text"));
    assert_eq!(returned(FrostString::from("text")), Value::from("text"));
    assert_eq!(
        returned(FrostBytes::from(vec![0, 255])),
        Value::from(vec![0u8, 255])
    );
    assert_eq!(
        returned(FrostArray::from(vec![Value::Int(1)])),
        Value::array([1])
    );
    assert_eq!(
        returned(FrostMap::from(ValueMap::new())),
        Value::from(ValueMap::new())
    );
}

#[test]
fn every_integer_type_returns_an_int() {
    assert_eq!(returned(255u8), Value::Int(255));
    assert_eq!(returned(-128i8), Value::Int(-128));
    assert_eq!(returned(65_535u16), Value::Int(65_535));
    assert_eq!(returned(-32_768i16), Value::Int(-32_768));
    assert_eq!(returned(u32::MAX), Value::Int(4_294_967_295));
    assert_eq!(returned(i32::MIN), Value::Int(-2_147_483_648));
    assert_eq!(returned(i64::MIN), Value::Int(i64::MIN));
    assert_eq!(returned(i64::MAX as u64), Value::Int(i64::MAX));
    assert_eq!(returned(7usize), Value::Int(7));
    assert_eq!(returned(-7isize), Value::Int(-7));
    assert_eq!(returned(i128::from(i64::MIN)), Value::Int(i64::MIN));
    assert_eq!(returned(i64::MAX as u128), Value::Int(i64::MAX));
}

#[test]
fn an_integer_outside_the_int_range_is_rejected() {
    assert_eq!(
        rejection(u64::MAX),
        "Function f has no Int result: 18446744073709551615 is out of range"
    );
    assert_eq!(
        rejection(i64::MAX as u64 + 1),
        "Function f has no Int result: 9223372036854775808 is out of range"
    );
    assert_eq!(
        rejection(i128::from(i64::MIN) - 1),
        "Function f has no Int result: -9223372036854775809 is out of range"
    );
    assert_eq!(
        rejection(u128::MAX),
        "Function f has no Int result: 340282366920938463463374607431768211455 is out of range"
    );
}

#[test]
fn a_finite_f64_returns_a_float() {
    assert_eq!(returned(2.5), Value::Float(FrostFloat::new(2.5).unwrap()));
    assert_eq!(returned(-0.0), Value::Float(FrostFloat::new(-0.0).unwrap()));
}

#[test]
fn a_non_finite_f64_is_rejected() {
    for f in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(rejection(f), "Function f has no finite result", "{f}");
    }
}

#[test]
fn option_returns_null_for_none() {
    assert_eq!(returned(None::<i64>), Value::Null);
    assert_eq!(returned(Some(5i64)), Value::Int(5));
    assert_eq!(returned(Some(Value::Null)), Value::Null);
    assert_eq!(rejection(Some(f64::NAN)), "Function f has no finite result");
}

#[test]
fn a_result_returns_its_value_or_raises_its_error() {
    assert_eq!(returned(Ok::<_, FrostError>(5i64)), Value::Int(5));
    assert_eq!(
        rejection(Err::<i64, _>(FrostError::from_static("it broke"))),
        "it broke"
    );
    assert_eq!(
        rejection(Ok::<_, FrostError>(u64::MAX)),
        "Function f has no Int result: 18446744073709551615 is out of range"
    );
}

#[test]
fn a_vec_returns_an_array() {
    assert_eq!(returned(vec![1i64, 2]), Value::array([1, 2]));
    assert_eq!(returned(Vec::<i64>::new()), Value::array::<i64, 0>([]));
    assert_eq!(
        returned(vec![vec!["a"], vec![]]),
        Value::array([Value::array(["a"]), Value::array::<i64, 0>([])])
    );
    // An Array of Ints, unlike `Value::from(Vec<u8>)`, which makes Bytes.
    assert_eq!(returned(vec![0u8, 255]), Value::array([0, 255]));
}

#[test]
fn a_vec_rejects_an_element_no_value_can_hold() {
    assert_eq!(
        rejection(vec![1.0, f64::NAN]),
        "Function f has no finite result"
    );
}

#[derive(Serialize)]
struct Report {
    count: u32,
    names: Vec<&'static str>,
}

#[test]
fn ser_serializes_the_result() {
    assert_eq!(
        returned(Ser(Report {
            count: 2,
            names: vec!["a", "b"],
        })),
        Value::map([
            ("count", Value::Int(2)),
            ("names", Value::array(["a", "b"])),
        ])
    );
}

#[test]
fn ser_rejects_a_result_that_does_not_serialize() {
    let unkeyable = BTreeMap::from([(vec![1], 2)]);
    let message = rejection(Ser(unkeyable));
    assert!(
        message.starts_with("Function f has a result no value can hold: "),
        "{message:?}"
    );
}
