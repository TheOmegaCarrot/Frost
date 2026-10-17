//! Ordering tests for `Value::compare`: the fallible, three-way comparison that backs `<`/`<=`/`>`/`>=`.
//! Comparable operands yield an `Ordering`; non-orderable ones (mismatched or inherently unordered types) are a type error, not a silent `None`.
//! Equality lives separately on `PartialEq` (see `value_equality.rs`).

use std::cmp::Ordering;
use std::sync::Arc;

use frostlang::{Arity, FrostArray, NativeFunction, SpecialFloat, Value};

use crate::deep;
use crate::script::Script;

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

/// An Array of `elements`.
fn array(elements: Vec<Value>) -> Value {
    Value::from(FrostArray::from(elements))
}

#[test]
fn equal_arrays_order_as_equal_whatever_they_hold() {
    // Each element is compared with an equal copy, never ordered.
    let unorderable = [
        Value::Null,
        Value::from(true),
        Value::from(frostlang::FrostMap::empty()),
        Value::opaque(frostlang::SpecialFloat::new(f64::INFINITY).unwrap()),
        array(vec![Value::Null]),
        array(vec![Value::from(1i64), Value::from("x")]),
    ];
    for element in unorderable {
        let a = array(vec![element.clone(), Value::from(1i64)]);
        let b = array(vec![element.clone(), Value::from(1i64)]);
        assert_eq!(
            a.compare(&b).ok(),
            Some(Ordering::Equal),
            "[{element:?}, 1] orders as equal to itself"
        );
    }
}

#[test]
fn an_equal_unorderable_pair_is_passed_over_for_the_next() {
    let a = array(vec![Value::Null, Value::from(1i64)]);
    let b = array(vec![Value::Null, Value::from(2i64)]);
    assert_eq!(a.compare(&b).ok(), Some(Ordering::Less));
    assert_eq!(b.compare(&a).ok(), Some(Ordering::Greater));
}

#[test]
fn a_prefix_orders_first_even_before_unorderable_elements() {
    let short = array(vec![Value::Null]);
    let long = array(vec![Value::Null, Value::from(true), Value::Null]);
    assert_eq!(short.compare(&long).ok(), Some(Ordering::Less));
    assert_eq!(long.compare(&short).ok(), Some(Ordering::Greater));
}

#[test]
fn equal_unorderable_elements_are_passed_over_in_nested_arrays() {
    let a = array(vec![
        array(vec![Value::Null, Value::from(1i64)]),
        Value::Null,
    ]);
    let b = array(vec![
        array(vec![Value::Null, Value::from(2i64)]),
        Value::Null,
    ]);
    assert_eq!(a.compare(&b).ok(), Some(Ordering::Less));
}

#[test]
fn an_int_and_float_of_the_same_value_are_passed_over() {
    // Not equal by `==`, but they order as equal, so the next pair decides.
    let a = array(vec![Value::from(1i64), Value::from(3i64)]);
    let b = array(vec![
        Value::from(frostlang::FrostFloat::new(1.0).unwrap()),
        Value::from(2i64),
    ]);
    assert_eq!(a.compare(&b).ok(), Some(Ordering::Greater));
}

#[test]
fn the_first_unequal_pair_raises_if_it_cannot_be_ordered() {
    let cases = [
        (
            Value::from(true),
            Value::from(false),
            "Type Bool is not orderable",
        ),
        (
            Value::Null,
            Value::from(1i64),
            "Cannot compare incompatible types: Null and Int",
        ),
    ];
    for (x, y, message) in cases {
        let a = array(vec![Value::Null, x.clone(), Value::from(1i64)]);
        let b = array(vec![Value::Null, y.clone(), Value::from(2i64)]);
        let Err(error) = a.compare(&b) else {
            panic!("the pair {x:?}, {y:?} cannot be ordered");
        };
        assert_eq!(error.message(), message, "comparing {x:?} with {y:?}");
    }
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

// -- Deeply nested Arrays --
//
// Each is ordered on a small stack (see [`deep`](crate::deep)). The assertions report
// no values, since rendering one so deep is no part of what these test. Ordering a
// value this deep finishes only if it takes time linear in the depth.

/// How two values order, or the names of the types of the two elements that cannot be
/// ordered.
type Expected = Result<Ordering, (&'static str, &'static str)>;

/// The error for values of the types `l` and `r`, which cannot be ordered.
fn unorderable(l: &str, r: &str) -> String {
    if l == r {
        format!("Type {l} is not orderable")
    } else {
        format!("Cannot compare incompatible types: {l} and {r}")
    }
}

/// Asserts that `l` and `r` order as `expected`, and `r` and `l` as its reverse.
fn assert_orders(context: &str, l: &Value, r: &Value, expected: Expected) {
    let outcome = |l: &Value, r: &Value| l.compare(r).map_err(|e| e.message().into_owned());
    let forward = expected.map_err(|(l, r)| unorderable(l, r));
    let reversed = expected
        .map(Ordering::reverse)
        .map_err(|(l, r)| unorderable(r, l));
    assert_eq!(outcome(l, r), forward, "{context}: `l` with `r`");
    assert_eq!(outcome(r, l), reversed, "{context}: `r` with `l`");
}

/// A level of nesting, built around the level below.
type Wrap = fn(Value) -> Value;

/// Each of the ways the deep cases nest, by name, built one way for one value and the
/// other for the other: alone in an Array, or beside an element that orders as equal in
/// both values, before or after it, orderable or not, equal or not.
const SHAPES: [(&str, Wrap, Wrap); 6] = [
    ("[below]", deep::in_an_array, deep::in_an_array),
    (
        "[1, below]",
        |below| array(vec![Value::from(1i64), below]),
        |below| array(vec![Value::from(1i64), below]),
    ),
    (
        "[null, below]",
        |below| array(vec![Value::Null, below]),
        |below| array(vec![Value::Null, below]),
    ),
    (
        "[below, true]",
        |below| array(vec![below, Value::from(true)]),
        |below| array(vec![below, Value::from(true)]),
    ),
    (
        "[[null], below]",
        |below| array(vec![array(vec![Value::Null]), below]),
        |below| array(vec![array(vec![Value::Null]), below]),
    ),
    (
        "[[1], below] and [[1.0], below]",
        |below| array(vec![array(vec![Value::from(1i64)]), below]),
        |below| array(vec![array(vec![float(1.0)]), below]),
    ),
];

/// The depths around where ordering stops recursing and starts looping, and one far past.
const DEPTHS: [usize; 11] = [1, 2, 31, 32, 33, 34, 63, 64, 65, 66, deep::DEPTH];

fn float(f: f64) -> Value {
    f.try_into().expect("a finite Float")
}

/// Asserts that `inner` and `other_inner`, which `what` describes, each nested `depth`
/// levels deep in each shape, built apart, order as `expected`, either way round.
fn assert_nested_ordering(
    what: &str,
    depth: usize,
    inner: Value,
    other_inner: Value,
    expected: Expected,
) {
    deep::on_a_small_stack(|| {
        for (shape, wrap, other_wrap) in SHAPES {
            let l = deep::nested(depth, inner.clone(), wrap);
            let r = deep::nested(depth, other_inner.clone(), other_wrap);
            let context = format!("{what} in {shape} {depth} levels deep");
            assert_orders(&context, &l, &r, expected);
        }
    });
}

/// Asserts each of `cases`, an innermost pair and how they order, at each of [`DEPTHS`].
fn assert_innermost_cases(cases: &[(&str, Value, Value, Expected)]) {
    for depth in DEPTHS {
        for (what, inner, other_inner, expected) in cases {
            assert_nested_ordering(what, depth, inner.clone(), other_inner.clone(), *expected);
        }
    }
}

#[test]
fn deeply_nested_arrays_order_as_their_innermost_values() {
    let int = |i: i64| Value::from(i);
    assert_innermost_cases(&[
        ("1 and 1", int(1), int(1), Ok(Ordering::Equal)),
        ("1 and 2", int(1), int(2), Ok(Ordering::Less)),
        ("1 and 1.0", int(1), float(1.0), Ok(Ordering::Equal)),
        ("1 and 1.5", int(1), float(1.5), Ok(Ordering::Less)),
        (
            "'a' and 'b'",
            Value::from("a"),
            Value::from("b"),
            Ok(Ordering::Less),
        ),
        (
            "[1] and [1, 1]",
            array(vec![int(1)]),
            array(vec![int(1), int(1)]),
            Ok(Ordering::Less),
        ),
        (
            "[] and [null]",
            array(vec![]),
            array(vec![Value::Null]),
            Ok(Ordering::Less),
        ),
    ]);
}

/// Equal innermost values order as equal even when they cannot be ordered; unequal ones
/// that cannot be ordered raise, naming their own types, however deep.
#[test]
fn deeply_nested_arrays_pass_over_only_equal_unorderable_values() {
    let map = || Value::from(frostlang::FrostMap::empty());
    let infinity = || Value::opaque(SpecialFloat::new(f64::INFINITY).expect("special"));
    let nan = Value::opaque(SpecialFloat::new(f64::NAN).expect("special"));
    let native = Value::NativeFunction(a_native());
    let int = Value::from(1i64);
    assert_innermost_cases(&[
        (
            "null and null",
            Value::Null,
            Value::Null,
            Ok(Ordering::Equal),
        ),
        ("{} and {}", map(), map(), Ok(Ordering::Equal)),
        ("one Opaque", nan.clone(), nan.clone(), Ok(Ordering::Equal)),
        ("equal Opaques", infinity(), infinity(), Ok(Ordering::Equal)),
        ("one Function", native.clone(), native, Ok(Ordering::Equal)),
        (
            "true and false",
            Value::from(true),
            Value::from(false),
            Err(("Bool", "Bool")),
        ),
        ("null and 1", Value::Null, int.clone(), Err(("Null", "Int"))),
        ("{} and 1", map(), int.clone(), Err(("Map", "Int"))),
        (
            "[1] and 1",
            array(vec![int.clone()]),
            int,
            Err(("Array", "Int")),
        ),
        (
            "unequal Opaques",
            infinity(),
            nan,
            Err(("Opaque", "Opaque")),
        ),
        (
            "two Functions",
            Value::NativeFunction(a_native()),
            Value::NativeFunction(a_native()),
            Err(("Function", "Function")),
        ),
    ]);
}

fn a_native() -> Arc<NativeFunction> {
    Arc::new(NativeFunction::new(
        "f",
        Arity::Exact(0),
        |_, _: &mut [Value]| Ok(Value::Null),
    ))
}

/// Which element beside the deeper level a level of [`spine_with_siblings`] changes.
#[derive(Clone, Copy, PartialEq)]
enum Sibling {
    Before,
    After,
}

/// Arrays `depth` levels deep, each level `[before, below, after]` of Ints of its own,
/// counting from the innermost, except where `changes` puts another value, or none.
fn spine_with_siblings(depth: usize, changes: &[(usize, Sibling, Option<Value>)]) -> Value {
    (0..depth).fold(Value::Null, |below, level| {
        let sibling = |which| {
            changes
                .iter()
                .find(|(at, side, _)| (*at, *side) == (level, which))
                .map_or(Some(Value::from(level as i64)), |(_, _, value)| {
                    value.clone()
                })
        };
        let before = sibling(Sibling::Before);
        let after = sibling(Sibling::After);
        array(before.into_iter().chain([below]).chain(after).collect())
    })
}

/// The first pair to differ decides, at whatever level and on whichever side of the
/// deeper level it is: found as ordering recurses, as it loops, and where it takes up a
/// level again once the one below orders as equal.
#[test]
fn deeply_nested_arrays_order_by_their_first_difference() {
    let depth = deep::DEPTH;
    deep::on_a_small_stack(|| {
        let alike = spine_with_siblings(depth, &[]);
        let also_alike = spine_with_siblings(depth, &[]);
        assert_orders("alike", &alike, &also_alike, Ok(Ordering::Equal));

        let levels = [
            0,
            1,
            depth / 2,
            depth - 33,
            depth - 32,
            depth - 31,
            depth - 1,
        ];
        for level in levels {
            for (side, name) in [(Sibling::Before, "before"), (Sibling::After, "after")] {
                let context = format!("changed {name} the deeper level at {level}");
                let lower = spine_with_siblings(depth, &[(level, side, Some(Value::from(-1i64)))]);
                assert_orders(&context, &alike, &lower, Ok(Ordering::Greater));

                let unorderable = spine_with_siblings(depth, &[(level, side, Some(Value::Null))]);
                let context = format!("{context}, to null");
                assert_orders(&context, &alike, &unorderable, Err(("Int", "Null")));
            }

            // The deeper levels order as equal, so the shorter Array orders first.
            let context = format!("dropped after the deeper level at {level}");
            let shorter = spine_with_siblings(depth, &[(level, Sibling::After, None)]);
            assert_orders(&context, &alike, &shorter, Ok(Ordering::Greater));
        }

        // Only the first difference is ordered: one that cannot be, later, never is.
        for level in [1, depth / 2, depth - 1] {
            let context = format!("lower before the deeper level at {level}, null innermost");
            let changes = [
                (level, Sibling::Before, Some(Value::from(-1i64))),
                (0, Sibling::Before, Some(Value::Null)),
            ];
            let lower = spine_with_siblings(depth, &changes);
            assert_orders(&context, &alike, &lower, Ok(Ordering::Greater));
        }
    });
}

/// Each way a script orders values.
#[test]
fn a_script_orders_deeply_nested_arrays() {
    let source = format!(
        r"
defn nest(v, n) -> if n == 0: v else: nest([v], n - 1)
def a = nest(1, {depth})
def b = nest(2, {depth})
[
    a < b,
    a <= b,
    b > a,
    b >= a,
    sorted([b, a]) == [a, b],
    minimum([b, a]) == a,
    maximum([a, b]) == b,
    min_by([b, a], fn x -> x) == a,
    max_by([a, b], fn x -> x) == b,
    sort_by([b, a], fn x -> x) == [a, b],
]
",
        depth = deep::DEPTH
    );
    let tail = deep::on_a_small_stack(|| Script::new(&source).run());
    assert_eq!(tail, array(vec![Value::from(true); 10]), "{source}");
}
