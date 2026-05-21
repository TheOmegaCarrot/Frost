use std::sync::Arc;

use frost_core::{FrostArray, FrostError, FrostMap, MapKey, Value};

fn str_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s.as_bytes()))
}

// ---- Subtraction ----

#[test]
fn subtract_int_int() {
    assert_eq!(
        Value::from(5i64).subtract(&Value::from(3i64)).unwrap(),
        Value::from(2i64)
    );
}

#[test]
fn subtract_int_negative_result() {
    assert_eq!(
        Value::from(3i64).subtract(&Value::from(5i64)).unwrap(),
        Value::from(-2i64)
    );
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
    assert_eq!(
        Value::from(3i64).multiply(&Value::from(4i64)).unwrap(),
        Value::from(12i64)
    );
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
    assert_eq!(
        Value::from(7i64).divide(&Value::from(2i64)).unwrap(),
        Value::from(3i64)
    );
}

#[test]
fn divide_int_truncates_toward_zero() {
    assert_eq!(
        Value::from(-7i64).divide(&Value::from(2i64)).unwrap(),
        Value::from(-3i64)
    );
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

// ---- Addition: numeric ----

#[test]
fn add_int_int() {
    assert_eq!(
        Value::from(3i64).add(&Value::from(4i64)).unwrap(),
        Value::from(7i64)
    );
}

#[test]
fn add_float_float() {
    let a: Value = 1.5.try_into().unwrap();
    let b: Value = 2.5.try_into().unwrap();
    let r: Value = 4.0.try_into().unwrap();
    assert_eq!(a.add(&b).unwrap(), r);
}

#[test]
fn add_int_float_promotes() {
    let i = Value::from(3i64);
    let f: Value = 0.14.try_into().unwrap();
    let r = i.add(&f).unwrap();
    assert!(matches!(r, Value::Float(_)));
}

#[test]
fn add_float_int_promotes() {
    let f: Value = 0.5.try_into().unwrap();
    let i = Value::from(2i64);
    let r = f.add(&i).unwrap();
    assert!(matches!(r, Value::Float(_)));
}

#[test]
fn add_int_overflow_wraps() {
    let max = Value::from(i64::MAX);
    let one = Value::from(1i64);
    let result = max.add(&one).unwrap();
    assert_eq!(result, Value::from(i64::MIN));
}

#[test]
fn add_float_overflow_errors() {
    let big: Value = 1e308.try_into().unwrap();
    assert!(big.add(&big).is_err());
}

// ---- Addition: string concatenation ----

#[test]
fn add_string_string() {
    let r = Value::from("hello").add(&Value::from(" world")).unwrap();
    assert_eq!(r, Value::from("hello world"));
}

#[test]
fn add_string_empty_left() {
    let r = Value::from("").add(&Value::from("a")).unwrap();
    assert_eq!(r, Value::from("a"));
}

#[test]
fn add_string_empty_right() {
    let r = Value::from("a").add(&Value::from("")).unwrap();
    assert_eq!(r, Value::from("a"));
}

#[test]
fn add_string_both_empty() {
    let r = Value::from("").add(&Value::from("")).unwrap();
    assert_eq!(r, Value::from(""));
}

// ---- Addition: array concatenation ----

#[test]
fn add_array_array() {
    let l = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    let r = Value::from(FrostArray::new(&[Value::from(3i64), Value::from(4i64)]));
    let expected = Value::from(FrostArray::new(&[
        Value::from(1i64),
        Value::from(2i64),
        Value::from(3i64),
        Value::from(4i64),
    ]));
    assert_eq!(l.add(&r).unwrap(), expected);
}

#[test]
fn add_array_empty_left() {
    let l = Value::from(FrostArray::new(&[]));
    let r = Value::from(FrostArray::new(&[Value::from(1i64)]));
    assert_eq!(
        l.add(&r).unwrap(),
        Value::from(FrostArray::new(&[Value::from(1i64)]))
    );
}

#[test]
fn add_array_empty_right() {
    let l = Value::from(FrostArray::new(&[Value::from(1i64)]));
    let r = Value::from(FrostArray::new(&[]));
    assert_eq!(
        l.add(&r).unwrap(),
        Value::from(FrostArray::new(&[Value::from(1i64)]))
    );
}

#[test]
fn add_array_both_empty() {
    let l = Value::from(FrostArray::new(&[]));
    let r = Value::from(FrostArray::new(&[]));
    assert_eq!(l.add(&r).unwrap(), Value::from(FrostArray::new(&[])));
}

// ---- Addition: map merge ----

#[test]
fn add_map_disjoint_keys() {
    let l: FrostMap = vec![(str_key("a"), Value::from(1i64))].into_iter().collect();
    let r: FrostMap = vec![(str_key("b"), Value::from(2i64))].into_iter().collect();
    let result = Value::from(l).add(&Value::from(r)).unwrap();

    let expected: FrostMap = vec![
        (str_key("a"), Value::from(1i64)),
        (str_key("b"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    assert_eq!(result, Value::from(expected));
}

#[test]
fn add_map_overlapping_keys_right_wins() {
    let l: FrostMap = vec![
        (str_key("a"), Value::from(1i64)),
        (str_key("b"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    let r: FrostMap = vec![
        (str_key("b"), Value::from(3i64)),
        (str_key("c"), Value::from(4i64)),
    ]
    .into_iter()
    .collect();
    let result = Value::from(l).add(&Value::from(r)).unwrap();

    let expected: FrostMap = vec![
        (str_key("a"), Value::from(1i64)),
        (str_key("b"), Value::from(3i64)),
        (str_key("c"), Value::from(4i64)),
    ]
    .into_iter()
    .collect();
    assert_eq!(result, Value::from(expected));
}

#[test]
fn add_map_shallow_merge_not_recursive() {
    let inner_l: FrostMap = vec![(str_key("x"), Value::from(1i64))].into_iter().collect();
    let inner_r: FrostMap = vec![(str_key("y"), Value::from(2i64))].into_iter().collect();
    let l: FrostMap = vec![(str_key("a"), Value::from(inner_l))].into_iter().collect();
    let r: FrostMap = vec![(str_key("a"), Value::from(inner_r.clone()))]
        .into_iter()
        .collect();
    let result = Value::from(l).add(&Value::from(r)).unwrap();

    let expected: FrostMap = vec![(str_key("a"), Value::from(inner_r))]
        .into_iter()
        .collect();
    assert_eq!(result, Value::from(expected));
}

#[test]
fn add_map_empty_left() {
    let l = FrostMap::empty();
    let r: FrostMap = vec![(str_key("a"), Value::from(1i64))].into_iter().collect();
    let result = Value::from(l).add(&Value::from(r.clone())).unwrap();
    assert_eq!(result, Value::from(r));
}

#[test]
fn add_map_empty_right() {
    let l: FrostMap = vec![(str_key("a"), Value::from(1i64))].into_iter().collect();
    let r = FrostMap::empty();
    let result = Value::from(l.clone()).add(&Value::from(r)).unwrap();
    assert_eq!(result, Value::from(l));
}

#[test]
fn add_map_both_empty() {
    let result = Value::from(FrostMap::empty())
        .add(&Value::from(FrostMap::empty()))
        .unwrap();
    assert_eq!(result, Value::from(FrostMap::empty()));
}

// ---- Addition: type errors ----

#[test]
fn add_int_string_errors() {
    assert!(Value::from(1i64).add(&Value::from("a")).is_err());
}

#[test]
fn add_array_map_errors() {
    let arr = Value::from(FrostArray::new(&[]));
    let map = Value::from(FrostMap::empty());
    assert!(arr.add(&map).is_err());
}

#[test]
fn add_type_error_is_recoverable() {
    let r = Value::from(1i64).add(&Value::from("a"));
    assert!(matches!(r.unwrap_err(), FrostError::Recoverable(_)));
}
