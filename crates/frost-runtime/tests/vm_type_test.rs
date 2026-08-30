//! Tests for the `TypeTest(EnumSet<FrostType>)` opcode, `( v -- b )`: consume a value, push a `Bool` of whether its type is in the set.
//!
//! The set/category *logic* is exhaustively covered at the value level in `value_type_category.rs`;
//! here we only test the opcode wiring: that it delegates to `fits` with the right set,
//! has the right stack effect, and yields a `Bool`.
//! One true/false case per set shape suffices.
//!
//! Operands without a `Push*` opcode (String/Array/Map) come from the constant table via `LoadConst`.

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, EnumSet, FormatVersion, FrostArray, FrostError, FrostFloat,
    FrostMap, FrostType, Value, Vm,
};

type Ft = FrostType;

fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<typetest>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants,
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.assert_trusted().into_closure().unwrap();
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .map_err(frost_runtime::RunError::into_error)
        .map(|r| r.tail().clone())
}

fn val(code: Vec<Bytecode>) -> Value {
    eval(vec![], code).unwrap()
}

fn float(x: f64) -> Bytecode {
    Bytecode::PushFloat(FrostFloat::new(x).unwrap())
}

mod common;

use Bytecode::{LoadConst, PushInt, PushNull, TypeTest};
use common::Pop;

// ============================================================
// Single-type sets
// ============================================================

#[test]
fn single_type_true_on_matching_type() {
    assert_eq!(
        val(vec![PushInt(1), TypeTest(Ft::Int.into())]),
        Value::Bool(true)
    );
}

#[test]
fn single_type_false_on_other_type() {
    let out = eval(
        vec![Value::from("x")],
        vec![LoadConst(0), TypeTest(Ft::Int.into())],
    )
    .unwrap();
    assert_eq!(out, Value::Bool(false));
}

// ============================================================
// Named category sets and |-built sets
// ============================================================

#[test]
fn numeric_true_on_float_false_on_string() {
    assert_eq!(
        val(vec![float(1.5), TypeTest(Ft::NUMERIC)]),
        Value::Bool(true)
    );
    let out = eval(
        vec![Value::from("x")],
        vec![LoadConst(0), TypeTest(Ft::NUMERIC)],
    )
    .unwrap();
    assert_eq!(out, Value::Bool(false));
}

#[test]
fn primitive_true_on_null_false_on_array() {
    // null is primitive...
    assert_eq!(
        val(vec![PushNull, TypeTest(Ft::PRIMITIVE)]),
        Value::Bool(true)
    );
    // ...an array is not.
    let out = eval(
        vec![Value::Array(FrostArray::empty())],
        vec![LoadConst(0), TypeTest(Ft::PRIMITIVE)],
    )
    .unwrap();
    assert_eq!(out, Value::Bool(false));
}

#[test]
fn structured_true_on_map_false_on_int() {
    let out = eval(
        vec![Value::Map(FrostMap::empty())],
        vec![LoadConst(0), TypeTest(Ft::STRUCTURED)],
    )
    .unwrap();
    assert_eq!(out, Value::Bool(true));
    assert_eq!(
        val(vec![PushInt(1), TypeTest(Ft::STRUCTURED)]),
        Value::Bool(false)
    );
}

#[test]
fn nonnull_false_on_null_true_on_int() {
    assert_eq!(
        val(vec![PushNull, TypeTest(Ft::NONNULL)]),
        Value::Bool(false)
    );
    assert_eq!(
        val(vec![PushInt(1), TypeTest(Ft::NONNULL)]),
        Value::Bool(true)
    );
}

#[test]
fn empty_set_is_always_false() {
    // The empty set is representable in the opcode; nothing fits it.
    assert_eq!(
        val(vec![PushInt(1), TypeTest(EnumSet::empty())]),
        Value::Bool(false)
    );
    assert_eq!(
        val(vec![PushNull, TypeTest(EnumSet::empty())]),
        Value::Bool(false)
    );
}

#[test]
fn ad_hoc_or_built_set() {
    // A set with no category name works the same: Int | String.
    assert_eq!(
        val(vec![PushInt(1), TypeTest(Ft::Int | Ft::String)]),
        Value::Bool(true)
    );
    assert_eq!(
        val(vec![PushNull, TypeTest(Ft::Int | Ft::String)]),
        Value::Bool(false)
    );
}

// ============================================================
// Stack effect
// ============================================================

#[test]
fn consumes_operand_and_pushes_one_bool() {
    // Sentinel below; TypeTest consumes the Int and pushes a single Bool, then Pop
    // drops the Bool, revealing the sentinel, proving `( v -- b )`.
    assert_eq!(
        val(vec![PushInt(99), PushInt(1), TypeTest(Ft::Int.into()), Pop]),
        Value::Int(99)
    );
}
