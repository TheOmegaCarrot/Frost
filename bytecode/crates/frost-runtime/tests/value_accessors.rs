use std::sync::Arc;

use frost_runtime::{FrostArray, FrostMap, MapKey, Value};

// -- as_int --

#[test]
fn as_int_from_int() {
    assert_eq!(Value::from(42i64).as_int(), Some(42));
}

#[test]
fn as_int_from_non_int() {
    let float_val: Value = 3.14f64.try_into().unwrap();
    assert_eq!(float_val.as_int(), None);
    assert_eq!(Value::from("42").as_int(), None);
    assert_eq!(Value::Null.as_int(), None);
}

// -- as_float --

#[test]
fn as_float_from_float() {
    let v: Value = 2.718.try_into().unwrap();
    assert_eq!(v.as_float(), Some(2.718));
}

#[test]
fn as_float_from_non_float() {
    assert_eq!(Value::from(1i64).as_float(), None);
    assert_eq!(Value::from("3.14").as_float(), None);
    assert_eq!(Value::Null.as_float(), None);
}

// -- as_bool --

#[test]
fn as_bool_from_bool() {
    assert_eq!(Value::from(true).as_bool(), Some(true));
    assert_eq!(Value::from(false).as_bool(), Some(false));
}

#[test]
fn as_bool_from_non_bool() {
    assert_eq!(Value::from(0i64).as_bool(), None);
    assert_eq!(Value::Null.as_bool(), None);
    assert_eq!(Value::from("true").as_bool(), None);
}

// -- as_str --

#[test]
fn as_str_from_utf8_string() {
    assert_eq!(Value::from("hello").as_str(), Some("hello"));
}

#[test]
fn as_str_from_empty_string() {
    assert_eq!(Value::from("").as_str(), Some(""));
}

#[test]
fn as_str_from_non_utf8_string() {
    let v = Value::from(vec![0xff, 0xfe]);
    assert_eq!(v.as_str(), None);
}

#[test]
fn as_str_from_non_string() {
    assert_eq!(Value::from(42i64).as_str(), None);
    assert_eq!(Value::Null.as_str(), None);
}

// -- as_byte_string --

#[test]
fn as_byte_string_from_utf8() {
    assert_eq!(
        Value::from("hello").as_byte_string(),
        Some(b"hello".as_slice())
    );
}

#[test]
fn as_byte_string_from_non_utf8() {
    let bytes = vec![0xff, 0xfe];
    let v = Value::from(bytes.clone());
    assert_eq!(v.as_byte_string(), Some(bytes.as_slice()));
}

#[test]
fn as_byte_string_from_non_string() {
    assert_eq!(Value::from(42i64).as_byte_string(), None);
    assert_eq!(Value::Null.as_byte_string(), None);
}

// -- as_array --

#[test]
fn as_array_from_array() {
    let arr = FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]);
    let v = Value::from(arr);
    assert!(v.as_array().is_some());
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn as_array_from_non_array() {
    assert!(Value::from(42i64).as_array().is_none());
    assert!(Value::from("not an array").as_array().is_none());
    assert!(Value::Null.as_array().is_none());
}

// -- as_map --

#[test]
fn as_map_from_map() {
    let map: FrostMap = vec![(
        MapKey::String(Arc::from(b"key".as_slice())),
        Value::from(1i64),
    )]
    .into_iter()
    .collect();
    let v = Value::from(map);
    assert!(v.as_map().is_some());
    assert_eq!(v.as_map().unwrap().len(), 1);
}

#[test]
fn as_map_from_non_map() {
    assert!(Value::from(42i64).as_map().is_none());
    assert!(Value::from(FrostArray::empty()).as_map().is_none());
    assert!(Value::Null.as_map().is_none());
}

// -- as_opaque --

#[test]
fn as_opaque_from_opaque() {
    let data: Arc<dyn std::any::Any + Send + Sync> = Arc::new(42u32);
    let v = Value::Opaque(data);
    let opaque = v.as_opaque().expect("should be opaque");
    let downcasted = opaque
        .downcast_ref::<u32>()
        .expect("should downcast to u32");
    assert_eq!(*downcasted, 42u32);
}

#[test]
fn as_opaque_from_non_opaque() {
    assert!(Value::from(42i64).as_opaque().is_none());
    assert!(Value::Null.as_opaque().is_none());
    assert!(Value::from("hello").as_opaque().is_none());
}

// -- wrong-type returns None consistently --

#[test]
fn null_returns_none_for_all() {
    let v = Value::Null;
    assert!(v.as_int().is_none());
    assert!(v.as_float().is_none());
    assert!(v.as_bool().is_none());
    assert!(v.as_str().is_none());
    assert!(v.as_byte_string().is_none());
    assert!(v.as_array().is_none());
    assert!(v.as_map().is_none());
    assert!(v.as_opaque().is_none());
}

// -- try_into_array --

#[test]
fn try_into_array_from_array() {
    let v = Value::from(vec![Value::from(1i64), Value::from(2i64)]);
    let arr = v.try_into_array().expect("an Array extracts");
    assert_eq!(arr.len(), 2);
}

#[test]
fn try_into_array_from_non_array_returns_the_value() {
    // The miss hands the original value back unchanged, not a clone or Null.
    let back = Value::from(42i64)
        .try_into_array()
        .expect_err("a non-Array does not extract");
    assert_eq!(back.as_int(), Some(42));
}

// -- try_into_map --

#[test]
fn try_into_map_from_map() {
    let v = Value::map([("a", Value::from(1i64))]);
    let map = v.try_into_map().expect("a Map extracts");
    assert_eq!(map.len(), 1);
}

#[test]
fn try_into_map_from_non_map_returns_the_value() {
    let back = Value::from("nope")
        .try_into_map()
        .expect_err("a non-Map does not extract");
    assert_eq!(back.as_str(), Some("nope"));
}
