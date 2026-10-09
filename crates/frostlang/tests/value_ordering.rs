//! Ordering tests for `Value::compare`: the fallible, three-way comparison that backs `<`/`<=`/`>`/`>=`.
//! Comparable operands yield an `Ordering`; non-orderable ones (mismatched or inherently unordered types) are a type error, not a silent `None`.
//! Equality lives separately on `PartialEq` (see `value_equality.rs`).

use std::cmp::Ordering;

use frostlang::{FrostArray, Value};

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

/// Assert each Int compares to its Float as `expected`, in both operand orders.
fn assert_int_float_orderings(cases: &[(i64, f64, Ordering)]) {
    for &(int, float, expected) in cases {
        let i = Value::from(int);
        let f: Value = float.try_into().unwrap();
        assert_eq!(i.compare(&f).unwrap(), expected, "{int} vs {float:?}");
        assert_eq!(
            f.compare(&i).unwrap(),
            expected.reverse(),
            "{float:?} vs {int}"
        );
    }
}

#[test]
fn int_float_ordering_is_exact_past_2_pow_53() {
    // Each Int here that is not a Float would round to its neighboring Float,
    // so a comparison that rounds the Int calls these Equal.
    const TWO_POW_53: i64 = 1 << 53;
    assert_int_float_orderings(&[
        (TWO_POW_53, 9_007_199_254_740_992.0, Ordering::Equal),
        (TWO_POW_53 + 1, 9_007_199_254_740_992.0, Ordering::Greater),
        (TWO_POW_53 + 1, 9_007_199_254_740_994.0, Ordering::Less),
        (-TWO_POW_53 - 1, -9_007_199_254_740_992.0, Ordering::Less),
        // 2^63 is one past the largest Int.
        (i64::MAX, 9_223_372_036_854_775_808.0, Ordering::Less),
        // 2^63 - 1024, the largest Float below 2^63.
        (i64::MAX, 9_223_372_036_854_774_784.0, Ordering::Greater),
        (i64::MIN, -9_223_372_036_854_775_808.0, Ordering::Equal),
        (
            i64::MIN + 1,
            -9_223_372_036_854_775_808.0,
            Ordering::Greater,
        ),
    ]);
}

#[test]
fn int_float_ordering_holds_past_the_int_range() {
    assert_int_float_orderings(&[
        // -2^63 - 2048, the largest Float below the smallest Int.
        (i64::MIN, -9_223_372_036_854_777_856.0, Ordering::Greater),
        (i64::MAX, 1e19, Ordering::Less),
        (i64::MIN, -1e19, Ordering::Greater),
        (i64::MAX, f64::MAX, Ordering::Less),
        (i64::MIN, f64::MIN, Ordering::Greater),
        (0, f64::MAX, Ordering::Less),
        (0, f64::MIN, Ordering::Greater),
    ]);
}

#[test]
fn int_float_ordering_counts_the_fraction() {
    assert_int_float_orderings(&[
        (3, 3.5, Ordering::Less),
        (3, 2.5, Ordering::Greater),
        (-3, -2.5, Ordering::Less),
        (-3, -3.5, Ordering::Greater),
        (-1, -0.5, Ordering::Less),
        (0, 0.5, Ordering::Less),
        (0, -0.5, Ordering::Greater),
        (0, 0.0, Ordering::Equal),
        (0, -0.0, Ordering::Equal),
        // The smallest fractions there are.
        (0, 5e-324, Ordering::Less),
        (0, -5e-324, Ordering::Greater),
        // A half above 2^51: below 2^52, the last range where a Float has a fraction.
        (
            2_251_799_813_685_248,
            2_251_799_813_685_248.5,
            Ordering::Less,
        ),
        (
            2_251_799_813_685_249,
            2_251_799_813_685_248.5,
            Ordering::Greater,
        ),
    ]);
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
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(3i64)]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Less);
}

#[test]
fn array_prefix_is_less() {
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::from(vec![
        Value::from(1i64),
        Value::from(2i64),
        Value::from(3i64),
    ]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Less);
}

#[test]
fn array_equal() {
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Equal);
}

#[test]
fn empty_arrays_equal() {
    let a = Value::from(FrostArray::from(vec![]));
    let b = Value::from(FrostArray::from(vec![]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Equal);
}

#[test]
fn empty_array_less_than_nonempty() {
    let a = Value::from(FrostArray::from(vec![]));
    let b = Value::from(FrostArray::from(vec![Value::from(1i64)]));
    assert_eq!(a.compare(&b).unwrap(), Ordering::Less);
}

#[test]
fn array_incomparable_element_is_error() {
    // [1, "x"] vs [1, 2]: the second elements (String vs Int) are not orderable,
    // so comparison fails, and the error blames the element types, not Array.
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from("x")]));
    let b = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    let err = a.compare(&b).unwrap_err();
    assert!(
        err.message().contains("String") && err.message().contains("Int"),
        "got: {}",
        err.message()
    );
    assert!(
        !err.message().contains("Array"),
        "should blame the element, not Array: {}",
        err.message()
    );
}

#[test]
fn array_incomparable_element_short_circuited_away() {
    // [1, "x"] vs [2, 3]: decided at index 0 (1 < 2), so the incomparable second
    // elements are never reached: no error.
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from("x")]));
    let b = Value::from(FrostArray::from(vec![Value::from(2i64), Value::from(3i64)]));
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
    use frostlang::FrostMap;
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
    let arr = Value::from(FrostArray::from(vec![]));
    assert!(Value::from("a").compare(&arr).is_err());
}
