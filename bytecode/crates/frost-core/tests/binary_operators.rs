use frost_core::{FrostError, Value};

// ---- Subtraction ----

#[test]
fn subtract_int_int() {
    assert_eq!(Value::from(5i64).subtract(&Value::from(3i64)).unwrap(), Value::from(2i64));
}

#[test]
fn subtract_int_negative_result() {
    assert_eq!(Value::from(3i64).subtract(&Value::from(5i64)).unwrap(), Value::from(-2i64));
}

#[test]
fn subtract_float_float() {
    let a: Value = 5.5.try_into().unwrap();
    let b: Value = 2.5.try_into().unwrap();
    let r: Value = 3.0.try_into().unwrap();
    assert_eq!(a.subtract(&b).unwrap(), r);
}

#[test]
fn subtract_int_float_promotes() {
    let i = Value::from(5i64);
    let f: Value = 2.5.try_into().unwrap();
    let r = i.subtract(&f).unwrap();
    assert!(matches!(r, Value::Float(_)));
}

#[test]
fn subtract_float_int_promotes() {
    let f: Value = 5.5.try_into().unwrap();
    let i = Value::from(2i64);
    let r = f.subtract(&i).unwrap();
    assert!(matches!(r, Value::Float(_)));
}

#[test]
fn subtract_int_overflow_wraps() {
    let min = Value::from(i64::MIN);
    let one = Value::from(1i64);
    let result = min.subtract(&one).unwrap();
    assert_eq!(result, Value::from(i64::MAX));
}

#[test]
fn subtract_type_error() {
    let r = Value::from("a").subtract(&Value::from(1i64));
    assert!(r.is_err());
    assert!(matches!(r.unwrap_err(), FrostError::Recoverable(_)));
}

// ---- Multiplication ----

#[test]
fn multiply_int_int() {
    assert_eq!(Value::from(3i64).multiply(&Value::from(4i64)).unwrap(), Value::from(12i64));
}

#[test]
fn multiply_float_float() {
    let a: Value = 2.5.try_into().unwrap();
    let b: Value = 4.0.try_into().unwrap();
    let r: Value = 10.0.try_into().unwrap();
    assert_eq!(a.multiply(&b).unwrap(), r);
}

#[test]
fn multiply_int_float_promotes() {
    let i = Value::from(3i64);
    let f: Value = 2.5.try_into().unwrap();
    let r = i.multiply(&f).unwrap();
    assert!(matches!(r, Value::Float(_)));
}

#[test]
fn multiply_float_int_promotes() {
    let f: Value = 2.5.try_into().unwrap();
    let i = Value::from(3i64);
    let r = f.multiply(&i).unwrap();
    assert!(matches!(r, Value::Float(_)));
}

#[test]
fn multiply_int_overflow_wraps() {
    let max = Value::from(i64::MAX);
    let two = Value::from(2i64);
    let result = max.multiply(&two).unwrap();
    assert_eq!(result, Value::from(-2i64));
}

#[test]
fn multiply_float_overflow_errors() {
    let big: Value = 1e308.try_into().unwrap();
    let two: Value = 2.0.try_into().unwrap();
    assert!(big.multiply(&two).is_err());
}

#[test]
fn multiply_type_error() {
    let r = Value::from("a").multiply(&Value::from(1i64));
    assert!(r.is_err());
    assert!(matches!(r.unwrap_err(), FrostError::Recoverable(_)));
}

// ---- Division ----

#[test]
fn divide_int_int() {
    assert_eq!(Value::from(7i64).divide(&Value::from(2i64)).unwrap(), Value::from(3i64));
}

#[test]
fn divide_int_truncates_toward_zero() {
    assert_eq!(Value::from(-7i64).divide(&Value::from(2i64)).unwrap(), Value::from(-3i64));
}

#[test]
fn divide_float_float() {
    let a: Value = 7.0.try_into().unwrap();
    let b: Value = 2.0.try_into().unwrap();
    let r: Value = 3.5.try_into().unwrap();
    assert_eq!(a.divide(&b).unwrap(), r);
}

#[test]
fn divide_int_float_promotes() {
    let i = Value::from(7i64);
    let f: Value = 2.0.try_into().unwrap();
    let r: Value = 3.5.try_into().unwrap();
    assert_eq!(i.divide(&f).unwrap(), r);
}

#[test]
fn divide_float_int_promotes() {
    let f: Value = 7.0.try_into().unwrap();
    let i = Value::from(2i64);
    let r: Value = 3.5.try_into().unwrap();
    assert_eq!(f.divide(&i).unwrap(), r);
}

#[test]
fn divide_int_by_zero() {
    let r = Value::from(1i64).divide(&Value::from(0i64));
    assert!(r.is_err());
}

#[test]
fn divide_float_by_zero_int() {
    let f: Value = 1.0.try_into().unwrap();
    assert!(f.divide(&Value::from(0i64)).is_err());
}

#[test]
fn divide_int_by_zero_float() {
    let z: Value = 0.0.try_into().unwrap();
    assert!(Value::from(1i64).divide(&z).is_err());
}

#[test]
fn divide_float_by_zero_float() {
    let z: Value = 0.0.try_into().unwrap();
    let one: Value = 1.0.try_into().unwrap();
    assert!(one.divide(&z).is_err());
}

#[test]
fn divide_zero_by_zero_float() {
    let z: Value = 0.0.try_into().unwrap();
    assert!(z.divide(&z).is_err());
}

#[test]
fn divide_type_error() {
    let r = Value::from("a").divide(&Value::from(1i64));
    assert!(r.is_err());
    assert!(matches!(r.unwrap_err(), FrostError::Recoverable(_)));
}
