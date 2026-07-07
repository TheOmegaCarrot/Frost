//! Tests for the `TypeTest(FrostTypeCategory)` opcode: `( v -- b )` -- consume a value, push a `Bool` of whether it fits the category.
//!
//! The category *logic* is exhaustively covered at the value level in `value_type_category.rs`;
//! here we only test the opcode wiring: that it delegates to `fits_category` with the right category,
//! has the right stack effect, and yields a `Bool`.
//! One true/false case per category suffices.
//!
//! Operands without a `Push*` opcode (String/Array/Map) come from the constant table via `LoadConst`.

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostArray, FrostError, FrostFloat, FrostMap, FrostType,
    FrostTypeCategory, Value, Vm,
};

type Ft = FrostType;
type Ftc = FrostTypeCategory;

fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        name: "<typetest>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants,
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.into_closure().unwrap();
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
        .map(|r| r.tail().clone())
}

fn val(code: Vec<Bytecode>) -> Value {
    eval(vec![], code).unwrap()
}

fn float(x: f64) -> Bytecode {
    Bytecode::PushFloat(FrostFloat::new(x).unwrap())
}

use Bytecode::{LoadConst, Pop, PushInt, PushNull, TypeTest};

// ============================================================
// Exact
// ============================================================

#[test]
fn exact_true_on_matching_type() {
    assert_eq!(
        val(vec![PushInt(1), TypeTest(Ftc::Exact(Ft::Int))]),
        Value::Bool(true)
    );
}

#[test]
fn exact_false_on_other_type() {
    let out = eval(
        vec![Value::from("x")],
        vec![LoadConst(0), TypeTest(Ftc::Exact(Ft::Int))],
    )
    .unwrap();
    assert_eq!(out, Value::Bool(false));
}

// ============================================================
// Numeric / Primitive / Structured / NonNull
// ============================================================

#[test]
fn numeric_true_on_float_false_on_string() {
    assert_eq!(
        val(vec![float(1.5), TypeTest(Ftc::Numeric)]),
        Value::Bool(true)
    );
    let out = eval(
        vec![Value::from("x")],
        vec![LoadConst(0), TypeTest(Ftc::Numeric)],
    )
    .unwrap();
    assert_eq!(out, Value::Bool(false));
}

#[test]
fn primitive_true_on_null_false_on_array() {
    // null is primitive...
    assert_eq!(
        val(vec![PushNull, TypeTest(Ftc::Primitive)]),
        Value::Bool(true)
    );
    // ...an array is not.
    let out = eval(
        vec![Value::Array(FrostArray::empty())],
        vec![LoadConst(0), TypeTest(Ftc::Primitive)],
    )
    .unwrap();
    assert_eq!(out, Value::Bool(false));
}

#[test]
fn structured_true_on_map_false_on_int() {
    let out = eval(
        vec![Value::Map(FrostMap::empty())],
        vec![LoadConst(0), TypeTest(Ftc::Structured)],
    )
    .unwrap();
    assert_eq!(out, Value::Bool(true));
    assert_eq!(
        val(vec![PushInt(1), TypeTest(Ftc::Structured)]),
        Value::Bool(false)
    );
}

#[test]
fn nonnull_false_on_null_true_on_int() {
    assert_eq!(
        val(vec![PushNull, TypeTest(Ftc::NonNull)]),
        Value::Bool(false)
    );
    assert_eq!(
        val(vec![PushInt(1), TypeTest(Ftc::NonNull)]),
        Value::Bool(true)
    );
}

// ============================================================
// Stack effect
// ============================================================

#[test]
fn consumes_operand_and_pushes_one_bool() {
    // Sentinel below; TypeTest consumes the Int and pushes a single Bool, then Pop
    // drops the Bool, revealing the sentinel -- proving `( v -- b )`.
    assert_eq!(
        val(vec![
            PushInt(99),
            PushInt(1),
            TypeTest(Ftc::Exact(Ft::Int)),
            Pop
        ]),
        Value::Int(99)
    );
}
