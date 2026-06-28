//! Tests for the unary opcodes: `LogicalNot` and `Negate`.
//!
//! `LogicalNot` is total -- it maps any value to a `Bool` via Frost truthiness (only `null`/`false` are falsy).
//! `Negate` is numeric: it wraps `Int` (matching the codebase's wrapping integer policy), negates `Float`, and is a type error for everything else.
//!
//! Operands without a `Push*` opcode (String/Array/Map) come from the constant table via `LoadConst`.

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostArray, FrostError, FrostFloat, FrostMap, Value, Vm,
};

// ============================================================
// Helpers
// ============================================================

fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        name: "<unary>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants,
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.into_closure().unwrap();
    Vm::new(closure).unwrap().run().map(|r| r.tail().clone())
}

fn val(code: Vec<Bytecode>) -> Value {
    eval(vec![], code).unwrap()
}

fn float(x: f64) -> Bytecode {
    Bytecode::PushFloat(FrostFloat::new(x).unwrap())
}

fn float_val(x: f64) -> Value {
    Value::try_from(x).unwrap()
}

use Bytecode::{LoadConst, LogicalNot, Negate, Pop, PushFalse, PushInt, PushNull, PushTrue};

// ============================================================
// LogicalNot -- total, always yields a Bool
// ============================================================

#[test]
fn not_true_is_false() {
    assert_eq!(val(vec![PushTrue, LogicalNot]), Value::Bool(false));
}

#[test]
fn not_false_is_true() {
    assert_eq!(val(vec![PushFalse, LogicalNot]), Value::Bool(true));
}

#[test]
fn not_null_is_true() {
    // null is falsy.
    assert_eq!(val(vec![PushNull, LogicalNot]), Value::Bool(true));
}

#[test]
fn not_zero_is_false() {
    // 0 is truthy in Frost (only null/false are falsy), so `not 0` is false.
    assert_eq!(val(vec![PushInt(0), LogicalNot]), Value::Bool(false));
}

#[test]
fn not_nonzero_int_is_false() {
    assert_eq!(val(vec![PushInt(5), LogicalNot]), Value::Bool(false));
}

#[test]
fn not_empty_string_is_false() {
    // Empty string is truthy.
    assert_eq!(
        eval(vec![Value::from("")], vec![LoadConst(0), LogicalNot]).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn not_empty_array_is_false() {
    // Empty array is truthy.
    assert_eq!(
        eval(
            vec![Value::Array(FrostArray::empty())],
            vec![LoadConst(0), LogicalNot]
        )
        .unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn not_empty_map_is_false() {
    // Empty map is truthy.
    assert_eq!(
        eval(
            vec![Value::Map(FrostMap::empty())],
            vec![LoadConst(0), LogicalNot]
        )
        .unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn double_not_collapses_to_bool() {
    // not (not 5) -> not false -> true.
    assert_eq!(
        val(vec![PushInt(5), LogicalNot, LogicalNot]),
        Value::Bool(true)
    );
}

#[test]
fn logical_not_leaves_exactly_one_value() {
    // Sentinel below; after the op and a Pop, the sentinel is the tail.
    assert_eq!(
        val(vec![PushInt(42), PushTrue, LogicalNot, Pop]),
        Value::Int(42)
    );
}

// ============================================================
// Negate -- numeric, type error otherwise
// ============================================================

#[test]
fn negate_positive_int() {
    assert_eq!(val(vec![PushInt(5), Negate]), Value::Int(-5));
}

#[test]
fn negate_negative_int() {
    assert_eq!(val(vec![PushInt(-5), Negate]), Value::Int(5));
}

#[test]
fn negate_zero_int() {
    assert_eq!(val(vec![PushInt(0), Negate]), Value::Int(0));
}

#[test]
fn negate_int_min_wraps_to_itself() {
    // -i64::MIN overflows; the wrapping policy yields i64::MIN unchanged (matches
    // the oracle and the wrapping_* integer ops).
    assert_eq!(val(vec![PushInt(i64::MIN), Negate]), Value::Int(i64::MIN));
}

#[test]
fn negate_float() {
    assert_eq!(val(vec![float(2.5), Negate]), float_val(-2.5));
}

#[test]
fn negate_negative_float() {
    assert_eq!(val(vec![float(-1.5), Negate]), float_val(1.5));
}

#[test]
fn negate_bool_is_type_error() {
    let err = eval(vec![], vec![PushTrue, Negate]).unwrap_err();
    assert!(err.message.contains("negate"), "got: {}", err.message);
}

#[test]
fn negate_null_is_type_error() {
    let err = eval(vec![], vec![PushNull, Negate]).unwrap_err();
    assert!(err.message.contains("negate"), "got: {}", err.message);
}

#[test]
fn negate_string_is_type_error() {
    let err = eval(vec![Value::from("x")], vec![LoadConst(0), Negate]).unwrap_err();
    assert!(err.message.contains("negate"), "got: {}", err.message);
}

#[test]
fn negate_array_is_type_error() {
    let err = eval(
        vec![Value::Array(FrostArray::empty())],
        vec![LoadConst(0), Negate],
    )
    .unwrap_err();
    assert!(err.message.contains("negate"), "got: {}", err.message);
}

#[test]
fn negate_leaves_exactly_one_value() {
    // Sentinel below; after the op and a Pop, the sentinel is the tail.
    assert_eq!(
        val(vec![PushInt(42), PushInt(5), Negate, Pop]),
        Value::Int(42)
    );
}
