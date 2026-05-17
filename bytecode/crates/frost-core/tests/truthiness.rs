use frost_core::Value;

#[test]
fn null_is_falsy() {
    assert!(!Value::Null.is_truthy());
}

#[test]
fn false_is_falsy() {
    assert!(!Value::from(false).is_truthy());
}

#[test]
fn true_is_truthy() {
    assert!(Value::from(true).is_truthy());
}

#[test]
fn zero_is_truthy() {
    assert!(Value::from(0i64).is_truthy());
}

#[test]
fn empty_string_is_truthy() {
    assert!(Value::from("").is_truthy());
}

#[test]
fn nonempty_string_is_truthy() {
    assert!(Value::from("hello").is_truthy());
}

#[test]
fn integer_is_truthy() {
    assert!(Value::from(42i64).is_truthy());
}

#[test]
fn negative_integer_is_truthy() {
    assert!(Value::from(-1i64).is_truthy());
}

#[test]
fn float_zero_is_truthy() {
    let v: Value = 0.0.try_into().unwrap();
    assert!(v.is_truthy());
}

#[test]
fn float_is_truthy() {
    let v: Value = 3.14.try_into().unwrap();
    assert!(v.is_truthy());
}
