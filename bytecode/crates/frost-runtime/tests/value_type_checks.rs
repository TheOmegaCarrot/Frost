use frost_runtime::{FrostArray, FrostMap, Value};

#[test]
fn is_null() {
    assert!(Value::Null.is_null());
    assert!(!Value::from(0i64).is_null());
}

#[test]
fn is_bool() {
    assert!(Value::from(true).is_bool());
    assert!(!Value::Null.is_bool());
}

#[test]
fn is_int() {
    assert!(Value::from(42i64).is_int());
    assert!(!Value::from("42").is_int());
}

#[test]
fn is_float() {
    let f: Value = 3.14.try_into().unwrap();
    assert!(f.is_float());
    assert!(!Value::from(3i64).is_float());
}

#[test]
fn is_string() {
    assert!(Value::from("hello").is_string());
    assert!(!Value::from(42i64).is_string());
}

#[test]
fn is_array() {
    assert!(Value::from(FrostArray::new(&[])).is_array());
    assert!(!Value::Null.is_array());
}

#[test]
fn is_map() {
    assert!(Value::from(FrostMap::empty()).is_map());
    assert!(!Value::Null.is_map());
}

// ---- Type categories ----

#[test]
fn is_numeric() {
    assert!(Value::from(1i64).is_numeric());
    let f: Value = 1.0.try_into().unwrap();
    assert!(f.is_numeric());
    assert!(!Value::from(true).is_numeric());
    assert!(!Value::from("1").is_numeric());
}

#[test]
fn is_primitive() {
    assert!(Value::Null.is_primitive());
    assert!(Value::from(true).is_primitive());
    assert!(Value::from(1i64).is_primitive());
    let f: Value = 1.0.try_into().unwrap();
    assert!(f.is_primitive());
    assert!(Value::from("hi").is_primitive());
    assert!(!Value::from(FrostArray::new(&[])).is_primitive());
    assert!(!Value::from(FrostMap::empty()).is_primitive());
}

#[test]
fn is_structured() {
    assert!(Value::from(FrostArray::new(&[])).is_structured());
    assert!(Value::from(FrostMap::empty()).is_structured());
    assert!(!Value::from(1i64).is_structured());
    assert!(!Value::from("hi").is_structured());
    assert!(!Value::Null.is_structured());
}

#[test]
fn is_nonnull() {
    assert!(!Value::Null.is_nonnull());
    assert!(Value::from(false).is_nonnull());
    assert!(Value::from(0i64).is_nonnull());
    assert!(Value::from("").is_nonnull());
}
