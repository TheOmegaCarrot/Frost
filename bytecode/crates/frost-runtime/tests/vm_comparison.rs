//! Tests for the `Compare*` opcode family.
//!
//! Equality (`CompareEqual`/`CompareNotEqual`) rides `Value: Eq` and is infallible.
//! Ordering (`CompareLessThan`, `...OrEqual`, `CompareGreaterThan`, `...OrEqual`) rides `Value::compare`,
//! where an unorderable pair is a Frost type error.
//!
//! Operands that have no `Push*` opcode (String/Array/Map) are supplied via the constant table and `LoadConst`.

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostArray, FrostError, FrostFloat, MapKey, Value, Vm,
};

// ============================================================
// Helpers
// ============================================================

/// Run a constants-carrying, local-less top-level program and return its tail
/// value (or the error it raised).
fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        name: "<cmp>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants,
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.assert_trusted().into_closure().unwrap();
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
        .map(|r| r.tail().clone())
}

/// Evaluate a constant-free program, expecting success.
fn val(code: Vec<Bytecode>) -> Value {
    eval(vec![], code).unwrap()
}

fn float(x: f64) -> Bytecode {
    Bytecode::PushFloat(FrostFloat::new(x).unwrap())
}

fn str_val(s: &str) -> Value {
    Value::from(s)
}

fn arr(xs: &[i64]) -> Value {
    Value::Array(FrostArray::from(
        xs.iter().copied().map(Value::Int).collect::<Vec<_>>(),
    ))
}

/// A single-entry map `{ a: <v> }`.
fn map_a(v: i64) -> Value {
    Value::Map(
        [(MapKey::String(Arc::from("a".as_bytes())), Value::Int(v))]
            .into_iter()
            .collect(),
    )
}

use Bytecode::{
    CompareEqual, CompareGreaterThan, CompareGreaterThanOrEqual, CompareLessThan,
    CompareLessThanOrEqual, CompareNotEqual, LoadConst, PushFalse, PushInt, PushNull, PushTrue,
};

// ============================================================
// Equality (infallible)
// ============================================================

#[test]
fn equal_ints() {
    assert_eq!(
        val(vec![PushInt(5), PushInt(5), CompareEqual]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushInt(5), PushInt(6), CompareEqual]),
        Value::Bool(false)
    );
}

#[test]
fn equal_bools() {
    assert_eq!(
        val(vec![PushTrue, PushTrue, CompareEqual]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushTrue, PushFalse, CompareEqual]),
        Value::Bool(false)
    );
}

#[test]
fn equal_nulls() {
    // Null == Null is true (equality, not ordering).
    assert_eq!(
        val(vec![PushNull, PushNull, CompareEqual]),
        Value::Bool(true)
    );
}

#[test]
fn equal_floats() {
    assert_eq!(
        val(vec![float(2.5), float(2.5), CompareEqual]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![float(2.5), float(2.6), CompareEqual]),
        Value::Bool(false)
    );
}

#[test]
fn equal_strings() {
    assert_eq!(
        eval(
            vec![str_val("ab"), str_val("ab")],
            vec![LoadConst(0), LoadConst(1), CompareEqual]
        )
        .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        eval(
            vec![str_val("ab"), str_val("ac")],
            vec![LoadConst(0), LoadConst(1), CompareEqual]
        )
        .unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn equal_arrays() {
    assert_eq!(
        eval(
            vec![arr(&[1, 2]), arr(&[1, 2])],
            vec![LoadConst(0), LoadConst(1), CompareEqual]
        )
        .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        eval(
            vec![arr(&[1, 2]), arr(&[1, 3])],
            vec![LoadConst(0), LoadConst(1), CompareEqual]
        )
        .unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn equal_maps() {
    assert_eq!(
        eval(
            vec![map_a(1), map_a(1)],
            vec![LoadConst(0), LoadConst(1), CompareEqual]
        )
        .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        eval(
            vec![map_a(1), map_a(2)],
            vec![LoadConst(0), LoadConst(1), CompareEqual]
        )
        .unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn equal_int_and_float_is_false() {
    // Frost has no cross-type numeric equality: 3 == 3.0 is false.
    assert_eq!(
        val(vec![PushInt(3), float(3.0), CompareEqual]),
        Value::Bool(false)
    );
}

#[test]
fn equal_mismatched_types_is_false() {
    assert_eq!(
        val(vec![PushInt(5), PushTrue, CompareEqual]),
        Value::Bool(false)
    );
    assert_eq!(
        val(vec![PushInt(5), PushNull, CompareEqual]),
        Value::Bool(false)
    );
}

// ============================================================
// Inequality (infallible)
// ============================================================

#[test]
fn not_equal_ints() {
    assert_eq!(
        val(vec![PushInt(5), PushInt(6), CompareNotEqual]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushInt(5), PushInt(5), CompareNotEqual]),
        Value::Bool(false)
    );
}

#[test]
fn not_equal_cross_type_is_true() {
    // 3 != 3.0 is true (mirrors `==` being false across Int/Float).
    assert_eq!(
        val(vec![PushInt(3), float(3.0), CompareNotEqual]),
        Value::Bool(true)
    );
}

// ============================================================
// Ordering (type error on unorderable operands)
// ============================================================

#[test]
fn less_than_ints() {
    assert_eq!(
        val(vec![PushInt(1), PushInt(2), CompareLessThan]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushInt(2), PushInt(1), CompareLessThan]),
        Value::Bool(false)
    );
    assert_eq!(
        val(vec![PushInt(2), PushInt(2), CompareLessThan]),
        Value::Bool(false)
    );
}

#[test]
fn less_than_or_equal_ints() {
    assert_eq!(
        val(vec![PushInt(2), PushInt(2), CompareLessThanOrEqual]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushInt(3), PushInt(2), CompareLessThanOrEqual]),
        Value::Bool(false)
    );
}

#[test]
fn greater_than_ints() {
    assert_eq!(
        val(vec![PushInt(3), PushInt(2), CompareGreaterThan]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushInt(2), PushInt(3), CompareGreaterThan]),
        Value::Bool(false)
    );
}

#[test]
fn greater_than_or_equal_ints() {
    assert_eq!(
        val(vec![PushInt(2), PushInt(2), CompareGreaterThanOrEqual]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushInt(1), PushInt(2), CompareGreaterThanOrEqual]),
        Value::Bool(false)
    );
}

#[test]
fn ordering_operands_are_not_reversed() {
    // lhs is the deeper operand; 3 < 10 must be true (a swapped impl would say false).
    assert_eq!(
        val(vec![PushInt(3), PushInt(10), CompareLessThan]),
        Value::Bool(true)
    );
}

#[test]
fn ordering_floats() {
    assert_eq!(
        val(vec![float(1.5), float(2.5), CompareLessThan]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![float(2.5), float(1.5), CompareGreaterThan]),
        Value::Bool(true)
    );
}

#[test]
fn ordering_int_and_float_cross_type() {
    // Ordering (unlike equality) works across Int/Float.
    assert_eq!(
        val(vec![PushInt(1), float(2.5), CompareLessThan]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushInt(3), float(2.5), CompareLessThan]),
        Value::Bool(false)
    );
    assert_eq!(
        val(vec![float(2.5), PushInt(3), CompareLessThan]),
        Value::Bool(true)
    );
}

#[test]
fn ordering_strings_lexicographic() {
    assert_eq!(
        eval(
            vec![str_val("abc"), str_val("abd")],
            vec![LoadConst(0), LoadConst(1), CompareLessThan]
        )
        .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        eval(
            vec![str_val("b"), str_val("a")],
            vec![LoadConst(0), LoadConst(1), CompareGreaterThan]
        )
        .unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn ordering_arrays() {
    assert_eq!(
        eval(
            vec![arr(&[1, 2]), arr(&[1, 3])],
            vec![LoadConst(0), LoadConst(1), CompareLessThan]
        )
        .unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn ordering_array_incomparable_element_blames_element_types() {
    // [1, 'hello'] < [1, 3.14]: fails at element 1 (String vs Float). The error
    // must name the element types, not the enclosing "Array".
    let lhs = Value::Array(FrostArray::from(vec![Value::Int(1), str_val("hello")]));
    let rhs = Value::Array(FrostArray::from(vec![
        Value::Int(1),
        Value::try_from(3.14).unwrap(),
    ]));
    let err = eval(
        vec![lhs, rhs],
        vec![LoadConst(0), LoadConst(1), CompareLessThan],
    )
    .unwrap_err();
    assert!(
        err.message.contains("String") && err.message.contains("Float"),
        "should name the element types: {}",
        err.message
    );
    assert!(
        !err.message.contains("Array"),
        "should not blame Array: {}",
        err.message
    );
}

#[test]
fn ordering_array_short_circuits_before_incomparable_element() {
    // [1, 'hello'] < [2, 3.14]: decided at element 0 (1 < 2), so the incomparable
    // second elements are never compared -- result is true, no error.
    let lhs = Value::Array(FrostArray::from(vec![Value::Int(1), str_val("hello")]));
    let rhs = Value::Array(FrostArray::from(vec![
        Value::Int(2),
        Value::try_from(3.14).unwrap(),
    ]));
    assert_eq!(
        eval(
            vec![lhs, rhs],
            vec![LoadConst(0), LoadConst(1), CompareLessThan]
        )
        .unwrap(),
        Value::Bool(true)
    );
}

// ============================================================
// Ordering type errors
// ============================================================

#[test]
fn every_ordering_opcode_errors_on_unorderable_operands() {
    // Bool is not orderable; each of the four ordering opcodes must raise.
    for op in [
        CompareLessThan,
        CompareLessThanOrEqual,
        CompareGreaterThan,
        CompareGreaterThanOrEqual,
    ] {
        let err = eval(vec![], vec![PushTrue, PushFalse, op]).unwrap_err();
        assert!(
            err.message.contains("Cannot compare"),
            "op {op:?} should be a type error, got: {}",
            err.message
        );
    }
}

#[test]
fn ordering_null_is_type_error() {
    let err = eval(vec![], vec![PushNull, PushNull, CompareLessThan]).unwrap_err();
    assert!(
        err.message.contains("Cannot compare"),
        "got: {}",
        err.message
    );
}

#[test]
fn ordering_maps_is_type_error() {
    let err = eval(
        vec![map_a(1), map_a(1)],
        vec![LoadConst(0), LoadConst(1), CompareLessThan],
    )
    .unwrap_err();
    assert!(
        err.message.contains("Cannot compare"),
        "got: {}",
        err.message
    );
}

#[test]
fn ordering_mismatched_types_is_type_error() {
    // Int vs String, and Int vs Null -- both unorderable.
    let err = eval(
        vec![str_val("a")],
        vec![PushInt(1), LoadConst(0), CompareLessThan],
    )
    .unwrap_err();
    assert!(
        err.message.contains("incompatible types"),
        "got: {}",
        err.message
    );

    let err = eval(vec![], vec![PushInt(1), PushNull, CompareGreaterThan]).unwrap_err();
    assert!(
        err.message.contains("incompatible types"),
        "got: {}",
        err.message
    );
}

// ============================================================
// Stack effect
// ============================================================

#[test]
fn comparison_leaves_exactly_one_value() {
    // Sentinel below; after the compare and a Pop, the sentinel is the tail --
    // proving the opcode consumed two operands and pushed exactly one result.
    assert_eq!(
        val(vec![
            PushInt(99),
            PushInt(1),
            PushInt(2),
            CompareLessThan,
            Bytecode::Pop
        ]),
        Value::Int(99)
    );
}
