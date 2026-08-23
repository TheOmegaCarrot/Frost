use frost_runtime::Value;

// ---- Negation ----

#[test]
fn negate_int() {
    assert_eq!(Value::from(5i64).negate().unwrap(), Value::from(-5i64));
}

#[test]
fn negate_negative_int() {
    assert_eq!(Value::from(-3i64).negate().unwrap(), Value::from(3i64));
}

#[test]
fn negate_int_min_wraps() {
    // i64::MIN has no positive counterpart; negation wraps back to itself.
    assert_eq!(
        Value::from(i64::MIN).negate().unwrap(),
        Value::from(i64::MIN)
    );
}

#[test]
fn negate_float() {
    let f: Value = 2.5.try_into().unwrap();
    let r: Value = (-2.5).try_into().unwrap();
    assert_eq!(f.negate().unwrap(), r);
}

#[test]
fn negate_float_stays_float() {
    let f: Value = 1.0.try_into().unwrap();
    assert!(matches!(f.negate().unwrap(), Value::Float(_)));
}

#[test]
fn negate_string_is_type_error() {
    let r = Value::from("a").negate();
    assert!(r.is_err());
    assert!(!r.unwrap_err().message().is_empty());
}

#[test]
fn negate_bool_is_type_error() {
    assert!(Value::from(true).negate().is_err());
}

#[test]
fn negate_null_is_type_error() {
    assert!(Value::Null.negate().is_err());
}
