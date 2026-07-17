use std::sync::Arc;

use frost_runtime::{FrostFloat, MapKey, Value};

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
fn from_arc_bytes() {
    let arc: Arc<[u8]> = Arc::from(b"hello" as &[u8]);
    let v: Value = arc.into();
    assert!(matches!(v, Value::String(_)));
}

#[test]
fn from_byte_slice() {
    let v: Value = (b"binary" as &[u8]).into();
    assert!(matches!(v, Value::String(_)));
}

#[test]
fn from_vec_u8() {
    let v: Value = vec![0u8, 1, 2, 255].into();
    assert!(matches!(v, Value::String(_)));
}

#[test]
fn from_frost_float() {
    let f = FrostFloat::new(2.5).unwrap();
    let v: Value = f.into();
    assert!(matches!(v, Value::Float(_)));
}

// ---- Collection conversions ----

#[test]
fn from_vec_of_values() {
    let v: Value = vec![Value::from(1i64), Value::from(2i64)].into();
    assert!(matches!(&v, Value::Array(a) if a.len() == 2));
}

#[test]
fn from_value_slice() {
    let elems = [Value::from(1i64), Value::from(2i64), Value::from(3i64)];
    let v: Value = elems.as_slice().into();
    assert!(matches!(&v, Value::Array(a) if a.len() == 3));
}

#[test]
fn collect_values_into_array() {
    let v: Value = (0..4).map(|i| Value::from(i as i64)).collect();
    assert!(matches!(&v, Value::Array(a) if a.len() == 4));
}

#[test]
fn collect_pairs_into_map() {
    let v: Value = vec![
        (MapKey::from("a"), Value::from(1i64)),
        (MapKey::from("b"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    assert!(matches!(&v, Value::Map(m) if m.len() == 2));
}

#[test]
fn array_constructor_maps_elements_in_order() {
    // Value::array accepts anything Into<Value>, so bare i64s work.
    let v = Value::array([1i64, 2, 3]);
    let arr = v.as_array().expect("Value::array builds an Array");
    assert_eq!(arr.len(), 3);
    assert!(matches!(arr.frost_get(0), Some(Value::Int(1))));
    assert!(matches!(arr.frost_get(2), Some(Value::Int(3))));
}
