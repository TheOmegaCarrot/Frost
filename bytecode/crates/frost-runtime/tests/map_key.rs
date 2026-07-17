use std::sync::Arc;

use frost_runtime::{FrostFloat, MapKey, Value};

#[test]
fn value_to_map_key_bool() {
    let v = Value::from(true);
    let k: MapKey = v.try_into().unwrap();
    assert!(matches!(k, MapKey::Bool(true)));
}

#[test]
fn value_to_map_key_int() {
    let v = Value::from(42i64);
    let k: MapKey = v.try_into().unwrap();
    assert!(matches!(k, MapKey::Int(42)));
}

#[test]
fn value_to_map_key_float() {
    let v: Value = 3.14.try_into().unwrap();
    let k: MapKey = v.try_into().unwrap();
    assert!(matches!(k, MapKey::Float(_)));
}

#[test]
fn value_to_map_key_string() {
    let v = Value::from("hello");
    let k: MapKey = v.try_into().unwrap();
    assert!(matches!(k, MapKey::String(_)));
}

#[test]
fn value_to_map_key_null_fails() {
    let v = Value::Null;
    let k: Result<MapKey, _> = v.try_into();
    assert!(k.is_err());
}

#[test]
fn value_to_map_key_null_error_has_message() {
    let v = Value::Null;
    let err: Result<MapKey, _> = v.try_into();
    assert!(!err.unwrap_err().message().is_empty());
}

#[test]
fn map_key_to_value() {
    let k = MapKey::Int(7);
    let v: Value = k.into();
    assert!(matches!(v, Value::Int(7)));
}

#[test]
fn map_key_ordering_same_type() {
    let a = MapKey::Int(1);
    let b = MapKey::Int(2);
    assert!(a < b);
}

#[test]
fn map_key_ordering_cross_type_is_consistent() {
    let bool_key = MapKey::Bool(false);
    let int_key = MapKey::Int(0);
    let float_key = MapKey::Float(FrostFloat::new(0.0).unwrap());
    let string_key = MapKey::String(Arc::from(b"" as &[u8]));

    // Exact order doesn't matter semantically, but it must be consistent
    assert!(bool_key < int_key);
    assert!(int_key < float_key);
    assert!(float_key < string_key);
}

// -- T -> MapKey conversions --

#[test]
fn key_from_str() {
    let k: MapKey = "hi".into();
    assert!(matches!(&k, MapKey::String(b) if &**b == b"hi"));
}

#[test]
fn key_from_string() {
    let k: MapKey = String::from("hi").into();
    assert!(matches!(&k, MapKey::String(b) if &**b == b"hi"));
}

#[test]
fn key_from_i64() {
    let k: MapKey = 42i64.into();
    assert!(matches!(k, MapKey::Int(42)));
}

#[test]
fn key_from_bool() {
    let k: MapKey = true.into();
    assert!(matches!(k, MapKey::Bool(true)));
}

#[test]
fn key_from_frost_float() {
    let k: MapKey = FrostFloat::new(2.5).unwrap().into();
    assert!(matches!(k, MapKey::Float(g) if g == FrostFloat::new(2.5).unwrap()));
}

#[test]
fn key_try_from_f64_valid() {
    let k = MapKey::try_from(2.5).unwrap();
    assert!(matches!(k, MapKey::Float(_)));
}

#[test]
fn key_try_from_f64_nan_fails() {
    assert!(MapKey::try_from(f64::NAN).is_err());
}

#[test]
fn key_try_from_f64_infinity_fails() {
    assert!(MapKey::try_from(f64::INFINITY).is_err());
}
