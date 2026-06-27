//! Ordering tests for `Value::compare` -- the fallible, three-way comparison that
//! backs `<`/`<=`/`>`/`>=`. Comparable operands yield an `Ordering`; non-orderable
//! ones (mismatched or inherently unordered types) are a type error, not a silent
//! `None`. Equality lives separately on `PartialEq` (see `value_equality.rs`).

use std::cmp::Ordering;

use frost_runtime::{FrostArray, Value};

// -- Int ordering --

#[test]
fn int_less_than() {
    assert_eq!(
        Value::from(1i64).compare(&Value::from(2i64)).unwrap(),
        Ordering::Less
    );
}

#[test]
fn int_greater_when_greater() {
    assert_eq!(
        Value::from(2i64).compare(&Value::from(1i64)).unwrap(),
        Ordering::Greater
    );
}

#[test]
fn int_equal() {
    assert_eq!(
        Value::from(1i64).compare(&Value::from(1i64)).unwrap(),
        Ordering::Equal
    );
}

// -- Float ordering --

#[test]
fn float_less_than() {
    let a: Value = 1.0.try_into().unwrap();
    let b: Value = 2.0.try_into().unwrap();
    assert_eq!(a.compare(&b).unwrap(), Ordering::Less);
}

#[test]
fn float_negative_zero_equals_zero() {
    let neg: Value = (-0.0f64).try_into().unwrap();
    let pos: Value = 0.0.try_into().unwrap();
    assert_eq!(neg.compare(&pos).unwrap(), Ordering::Equal);
    assert_eq!(pos.compare(&neg).unwrap(), Ordering::Equal);
}

// -- Cross-type numeric ordering (works, unlike equality) --

#[test]
fn int_less_than_float() {
    let i = Value::from(3i64);
    let f: Value = 3.14.try_into().unwrap();
    assert_eq!(i.compare(&f).unwrap(), Ordering::Less);
}

#[test]
fn float_less_than_int() {
    let f: Value = 3.14.try_into().unwrap();
    let i = Value::from(4i64);
    assert_eq!(f.compare(&i).unwrap(), Ordering::Less);
}

#[test]
fn int_float_equal_values_compare_equal() {
    let i = Value::from(3i64);
    let f: Value = 3.0.try_into().unwrap();
    assert_eq!(i.compare(&f).unwrap(), Ordering::Equal);
    assert_eq!(f.compare(&i).unwrap(), Ordering::Equal);
}

// -- String ordering --

#[test]
fn string_lexicographic() {
    assert_eq!(
        Value::from("abc").compare(&Value::from("abd")).unwrap(),
        Ordering::Less
    );
}

#[test]
fn string_equal() {
    assert_eq!(
        Value::from("abc").compare(&Value::from("abc")).unwrap(),
        Ordering::Equal
    );
}

#[test]
fn string_prefix_is_less() {
    assert_eq!(
        Value::from("ab").compare(&Value::from("abc")).unwrap(),
        Ordering::Less
    );
}

// -- Array ordering --

#[test]
fn array_lexicographic() {
    let a = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(3i64)]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Less);
}

#[test]
fn array_prefix_is_less() {
    let a = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::new(&[
        Value::from(1i64),
        Value::from(2i64),
        Value::from(3i64),
    ]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Less);
}

#[test]
fn array_equal() {
    let a = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Equal);
}

#[test]
fn empty_arrays_equal() {
    let a = Value::from(FrostArray::new(&[]));
    let b = Value::from(FrostArray::new(&[]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Equal);
}

#[test]
fn empty_array_less_than_nonempty() {
    let a = Value::from(FrostArray::new(&[]));
    let b = Value::from(FrostArray::new(&[Value::from(1i64)]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Less);
}

#[test]
fn array_incomparable_element_is_error() {
    // [1, "x"] vs [1, 2]: the second elements (String vs Int) are not orderable,
    // so comparison fails -- and the error blames the element types, not Array.
    let a = Value::from(FrostArray::new(&[Value::from(1i64), Value::from("x")]));
    let b = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    let err = a.compare(&b).unwrap_err();
    assert!(
        err.message.contains("String") && err.message.contains("Int"),
        "got: {}",
        err.message
    );
    assert!(
        !err.message.contains("Array"),
        "should blame the element, not Array: {}",
        err.message
    );
}

#[test]
fn array_incomparable_element_short_circuited_away() {
    // [1, "x"] vs [2, 3]: decided at index 0 (1 < 2), so the incomparable second
    // elements are never reached -- no error.
    let a = Value::from(FrostArray::new(&[Value::from(1i64), Value::from("x")]));
    let b = Value::from(FrostArray::new(&[Value::from(2i64), Value::from(3i64)]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Less);
}

// -- Non-orderable types are a type error --

#[test]
fn null_not_orderable() {
    assert!(Value::Null.compare(&Value::Null).is_err());
}

#[test]
fn bool_not_orderable() {
    assert!(Value::from(true).compare(&Value::from(false)).is_err());
}

#[test]
fn map_not_orderable() {
    use frost_runtime::FrostMap;
    let a = Value::from(FrostMap::empty());
    let b = Value::from(FrostMap::empty());
    assert!(a.compare(&b).is_err());
}

// -- Cross-type non-numeric is a type error --

#[test]
fn int_vs_string_not_orderable() {
    assert!(Value::from(1i64).compare(&Value::from("a")).is_err());
}

#[test]
fn null_vs_int_not_orderable() {
    assert!(Value::Null.compare(&Value::from(1i64)).is_err());
}

#[test]
fn string_vs_array_not_orderable() {
    let arr = Value::from(FrostArray::new(&[]));
    assert!(Value::from("a").compare(&arr).is_err());
}
