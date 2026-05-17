use std::sync::Arc;

use frost_core::{FrostFloat, Value};

#[test]
fn from_bool() {
    let t: Value = true.into();
    let f: Value = false.into();
    assert!(matches!(t, Value::Bool(true)));
    assert!(matches!(f, Value::Bool(false)));
}

#[test]
fn from_i64() {
    let v: Value = 42i64.into();
    assert!(matches!(v, Value::Int(42)));
}

#[test]
fn try_from_f64_valid() {
    let v: Result<Value, _> = 3.14.try_into();
    assert!(v.is_ok());
    assert!(matches!(v.unwrap(), Value::Float(_)));
}

#[test]
fn try_from_f64_nan() {
    let v: Result<Value, _> = f64::NAN.try_into();
    assert!(v.is_err());
}

#[test]
fn try_from_f64_infinity() {
    let v: Result<Value, _> = f64::INFINITY.try_into();
    assert!(v.is_err());
}

#[test]
fn from_str_ref() {
    let v: Value = "hello".into();
    assert!(matches!(v, Value::String(_)));
}

#[test]
fn from_string() {
    let v: Value = String::from("hello").into();
    assert!(matches!(v, Value::String(_)));
}

#[test]
fn from_arc_str() {
    let arc: Arc<str> = Arc::from("hello");
    let v: Value = arc.into();
    assert!(matches!(v, Value::String(_)));
}

#[test]
fn from_frost_float() {
    let f = FrostFloat::new(2.5).unwrap();
    let v: Value = f.into();
    assert!(matches!(v, Value::Float(_)));
}
