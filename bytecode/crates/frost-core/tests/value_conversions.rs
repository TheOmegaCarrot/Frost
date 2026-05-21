use frost_core::Value;

// ---- to_frost_int ----

#[test]
fn to_int_from_int() {
    assert_eq!(Value::from(42i64).to_frost_int(), Value::from(42i64));
}

#[test]
fn to_int_from_float_truncates() {
    let f: Value = 3.9.try_into().unwrap();
    assert_eq!(f.to_frost_int(), Value::from(3i64));
}

#[test]
fn to_int_from_negative_float_truncates_toward_zero() {
    let f: Value = (-3.9).try_into().unwrap();
    assert_eq!(f.to_frost_int(), Value::from(-3i64));
}

#[test]
fn to_int_from_string() {
    assert_eq!(Value::from("42").to_frost_int(), Value::from(42i64));
}

#[test]
fn to_int_from_negative_string() {
    assert_eq!(Value::from("-7").to_frost_int(), Value::from(-7i64));
}

#[test]
fn to_int_from_float_string_is_null() {
    assert_eq!(Value::from("3.14").to_frost_int(), Value::Null);
}

#[test]
fn to_int_from_non_numeric_string_is_null() {
    assert_eq!(Value::from("hello").to_frost_int(), Value::Null);
}

#[test]
fn to_int_from_empty_string_is_null() {
    assert_eq!(Value::from("").to_frost_int(), Value::Null);
}

#[test]
fn to_int_from_bool_is_null() {
    assert_eq!(Value::from(true).to_frost_int(), Value::Null);
}

#[test]
fn to_int_from_null_is_null() {
    assert_eq!(Value::Null.to_frost_int(), Value::Null);
}

// ---- to_frost_float ----

#[test]
fn to_float_from_float() {
    let f: Value = 3.14.try_into().unwrap();
    assert_eq!(f.to_frost_float(), f);
}

#[test]
fn to_float_from_int() {
    let result = Value::from(3i64).to_frost_float();
    assert!(result.is_float());
}

#[test]
fn to_float_from_string() {
    let result = Value::from("3.14").to_frost_float();
    assert!(result.is_float());
}

#[test]
fn to_float_from_int_string() {
    let result = Value::from("42").to_frost_float();
    assert!(result.is_float());
}

#[test]
fn to_float_from_non_numeric_string_is_null() {
    assert_eq!(Value::from("hello").to_frost_float(), Value::Null);
}

#[test]
fn to_float_from_bool_is_null() {
    assert_eq!(Value::from(true).to_frost_float(), Value::Null);
}

#[test]
fn to_float_from_null_is_null() {
    assert_eq!(Value::Null.to_frost_float(), Value::Null);
}
