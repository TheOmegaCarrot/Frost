//! The Operators globals, from Frost source: `plus`, `minus`, `times`, `divide`,
//! `mod`, `equal`, `not_equal`, `less_than`, `less_than_or_equal`,
//! `greater_than`, and `greater_than_or_equal`.
//!
//! Each is its binary operator as a Function: `plus(a, b)` is `a + b`, with the
//! same result or the same error. The operators themselves are pinned by the
//! runtime's own tests; these pin that each global is its operator, over
//! operands of every type.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over literals is checked both folded and at run time. Cases that capture their
//! input are never folded.

use crate::script;

use frostlang::Value;
use script::Script;
use script::assertions::{Library, library_assertions};

library_assertions!(Library::GLOBALS);

/// Each operator global.
const OPERATORS: &[&str] = &[
    "plus",
    "minus",
    "times",
    "divide",
    "mod",
    "equal",
    "not_equal",
    "less_than",
    "less_than_or_equal",
    "greater_than",
    "greater_than_or_equal",
];

/// Operands of every type, as Frost source, including zeros and values that
/// combine with themselves.
const OPERANDS: &[&str] = &[
    "7", "-2", "0", "2.5", "0.0", "'ab'", "'b'", "x'01'", "[1]", "{a: 1}", "null", "true", "plus",
];

/// Assert `function` called on each pair of operands has the outcome, value or
/// error, that `operator` has on that pair.
fn assert_behaves_as_operator(function: &str, operator: &str) {
    for lhs in OPERANDS {
        for rhs in OPERANDS {
            let call = format!("{function}({lhs}, {rhs})");
            let expression = format!("({lhs}) {operator} ({rhs})");
            assert_eq!(
                script(&call).outcome(),
                script(&expression).outcome(),
                "{call:?} is {expression:?}"
            );
        }
    }
}

// --- Arithmetic ---

#[test]
fn arithmetic_globals_compute_their_operation() {
    assert_values(&[
        ("plus(1, 2)", "3"),
        ("plus(1, 2.5)", "3.5"),
        ("plus('a', 'b')", "'ab'"),
        ("plus([1], [2])", "[1, 2]"),
        ("plus({a: 1, b: 2}, {a: 3})", "{a: 3, b: 2}"),
        ("minus(5, 7)", "-2"),
        ("minus(1.5, 1)", "0.5"),
        ("times(4, 3)", "12"),
        ("times(2.5, 2)", "5.0"),
        ("divide(7, 2)", "3"),
        ("divide(7.0, 2)", "3.5"),
        ("mod(7, 3)", "1"),
    ]);
}

#[test]
fn plus_is_the_plus_operator() {
    assert_behaves_as_operator("plus", "+");
}

#[test]
fn minus_is_the_minus_operator() {
    assert_behaves_as_operator("minus", "-");
}

#[test]
fn times_is_the_times_operator() {
    assert_behaves_as_operator("times", "*");
}

#[test]
fn divide_is_the_divide_operator() {
    assert_behaves_as_operator("divide", "/");
}

#[test]
fn mod_is_the_modulus_operator() {
    assert_behaves_as_operator("mod", "%");
}

#[test]
fn arithmetic_globals_raise_as_their_operators_do() {
    // A sample of the errors the exhaustive cases compare, spelled out.
    assert_raises_as(&[
        ("plus(1, 'a')", "1 + 'a'"),
        ("plus('a', x'00')", "'a' + x'00'"),
        ("minus('a', 'b')", "'a' - 'b'"),
        ("times('ab', 3)", "'ab' * 3"),
        ("divide(1, 0)", "1 / 0"),
        ("divide(1.0, 0)", "1.0 / 0"),
        ("mod(1, 0)", "1 % 0"),
        ("mod(7.5, 2)", "7.5 % 2"),
        ("times(1e308, 10)", "1e308 * 10"),
    ]);
}

// --- Comparison ---

#[test]
fn comparison_globals_compare() {
    assert_values(&[
        ("equal(1, 1)", "true"),
        ("equal([1], [1])", "true"),
        // No equality across Int and Float.
        ("equal(1, 1.0)", "false"),
        ("not_equal(1, 1.0)", "true"),
        ("not_equal('a', 'a')", "false"),
        ("less_than(1, 2)", "true"),
        // Ordering does cross Int and Float.
        ("less_than(1, 1.5)", "true"),
        ("less_than('b', 'a')", "false"),
        ("less_than_or_equal(2, 2)", "true"),
        ("greater_than(3, 2.5)", "true"),
        ("greater_than_or_equal('a', 'b')", "false"),
    ]);
}

#[test]
fn equal_is_the_equality_operator() {
    assert_behaves_as_operator("equal", "==");
}

#[test]
fn not_equal_is_the_inequality_operator() {
    assert_behaves_as_operator("not_equal", "!=");
}

#[test]
fn less_than_is_the_less_than_operator() {
    assert_behaves_as_operator("less_than", "<");
}

#[test]
fn less_than_or_equal_is_the_less_than_or_equal_operator() {
    assert_behaves_as_operator("less_than_or_equal", "<=");
}

#[test]
fn greater_than_is_the_greater_than_operator() {
    assert_behaves_as_operator("greater_than", ">");
}

#[test]
fn greater_than_or_equal_is_the_greater_than_or_equal_operator() {
    assert_behaves_as_operator("greater_than_or_equal", ">=");
}

#[test]
fn comparison_globals_raise_as_their_operators_do() {
    // A sample of the errors the exhaustive cases compare, spelled out.
    assert_raises_as(&[
        ("less_than('a', 1)", "'a' < 1"),
        ("less_than(null, null)", "null < null"),
        ("greater_than_or_equal({}, {})", "{} >= {}"),
    ]);
}

// --- As values ---

#[test]
fn operator_globals_work_as_function_values() {
    assert_values(&[
        ("reduce [1, 2, 3] with plus", "6"),
        ("reduce [10, 1, 2] with minus", "7"),
        ("select([1, 5, 3], curry(less_than, 2))", "[5, 3]"),
        ("map [[1, 2], [3, 3]] with spread(equal)", "[false, true]"),
    ]);
}

#[test]
fn operator_globals_operate_on_runtime_values() {
    let computed = Script::new("[plus(a, b), times(a, b), less_than(a, b), equal(a, b)]")
        .captures(&[("a", Value::Int(6)), ("b", Value::Int(7))])
        .run();
    assert_eq!(
        computed,
        Value::array([
            Value::Int(13),
            Value::Int(42),
            Value::Bool(true),
            Value::Bool(false)
        ])
    );
}

#[test]
fn operator_globals_take_exactly_two_arguments() {
    for function in OPERATORS {
        assert_arity(function, 2, &[0, 1, 3]);
    }
}
