use frost_runtime::{FrostArray, Value};

// -- Int ordering --

#[test]
fn int_less_than() {
    assert!(Value::from(1i64) < Value::from(2i64));
}

#[test]
fn int_not_less_when_greater() {
    assert!(!(Value::from(2i64) < Value::from(1i64)));
}

#[test]
fn int_not_less_when_equal() {
    assert!(!(Value::from(1i64) < Value::from(1i64)));
}

#[test]
fn int_greater_than() {
    assert!(Value::from(2i64) > Value::from(1i64));
}

// -- Float ordering --

#[test]
fn float_less_than() {
    let a: Value = 1.0.try_into().unwrap();
    let b: Value = 2.0.try_into().unwrap();
    assert!(a < b);
}

#[test]
fn float_negative_zero_not_less_than_zero() {
    let neg: Value = (-0.0f64).try_into().unwrap();
    let pos: Value = 0.0.try_into().unwrap();
    assert!(!(neg < pos));
    assert!(!(pos < neg));
}

// -- Cross-type numeric ordering --

#[test]
fn int_less_than_float() {
    let i = Value::from(3i64);
    let f: Value = 3.14.try_into().unwrap();
    assert!(i < f);
}

#[test]
fn float_less_than_int() {
    let f: Value = 3.14.try_into().unwrap();
    let i = Value::from(4i64);
    assert!(f < i);
}

#[test]
fn int_float_equal_values_not_less() {
    let i = Value::from(3i64);
    let f: Value = 3.0.try_into().unwrap();
    assert!(!(i < f));
    assert!(!(f < i));
}

// -- String ordering --

#[test]
fn string_lexicographic() {
    assert!(Value::from("abc") < Value::from("abd"));
}

#[test]
fn string_equal_not_less() {
    assert!(!(Value::from("abc") < Value::from("abc")));
}

#[test]
fn string_prefix_is_less() {
    assert!(Value::from("ab") < Value::from("abc"));
}

// -- Array ordering --

#[test]
fn array_lexicographic() {
    let a = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(3i64)]));
    assert!(a < b);
}

#[test]
fn array_prefix_is_less() {
    let a = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::new(&[
        Value::from(1i64),
        Value::from(2i64),
        Value::from(3i64),
    ]));
    assert!(a < b);
}

#[test]
fn array_equal_not_less() {
    let a = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::new(&[Value::from(1i64), Value::from(2i64)]));
    assert!(!(a < b));
}

#[test]
fn empty_arrays_not_less() {
    let a = Value::from(FrostArray::new(&[]));
    let b = Value::from(FrostArray::new(&[]));
    assert!(!(a < b));
}

#[test]
fn empty_array_less_than_nonempty() {
    let a = Value::from(FrostArray::new(&[]));
    let b = Value::from(FrostArray::new(&[Value::from(1i64)]));
    assert!(a < b);
}

// -- Non-comparable types return None (maps to error in Frost) --

#[test]
fn null_not_orderable() {
    assert!(Value::Null.partial_cmp(&Value::Null).is_none());
}

#[test]
fn bool_not_orderable() {
    assert!(Value::from(true).partial_cmp(&Value::from(false)).is_none());
}

#[test]
fn map_not_orderable() {
    use frost_runtime::FrostMap;
    let a = Value::from(FrostMap::empty());
    let b = Value::from(FrostMap::empty());
    assert!(a.partial_cmp(&b).is_none());
}

// -- Cross-type non-numeric returns None --

#[test]
fn int_vs_string_not_orderable() {
    assert!(Value::from(1i64).partial_cmp(&Value::from("a")).is_none());
}

#[test]
fn null_vs_int_not_orderable() {
    assert!(Value::Null.partial_cmp(&Value::from(1i64)).is_none());
}

#[test]
fn string_vs_array_not_orderable() {
    let arr = Value::from(FrostArray::new(&[]));
    assert!(Value::from("a").partial_cmp(&arr).is_none());
}
