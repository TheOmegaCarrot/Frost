//! Tests for `SoftIndexStructure` -- the `a[b]` opcode with null-on-missing
//! semantics. Stack: `( structure index -- result )` (structure deeper).
//!
//! It indexes an Array (by Int) or a Map (by primitive key). Distinguishing
//! *missing* (-> null) from a *type error* is the whole point, so the tests pin
//! both outcomes (cross-checked against the C++ oracle):
//!   * Array + Int -> element, or null when out of bounds (negatives count from
//!     the end); Array + non-Int -> error.
//!   * Map + valid key -> value, or null when the key is absent; Map + null or
//!     structured key -> error.
//!   * Indexing a non-structure (String, Int, ...) -> error.

use std::sync::Arc;

use frost_runtime::{Arity, Bytecode, CompiledFunction, FrostArray, FrostError, FrostFloat, MapKey, Value, Vm};

fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let program = Arc::new(CompiledFunction {
        name: "<index>".to_string(),
        code,
        child_fns: Vec::new(),
        constants,
        name_table: Vec::new(),
        arity: Arity::Exact(0),
    });
    Vm::new(program).unwrap().run().map(|r| r.tail().clone())
}

fn float(x: f64) -> Bytecode {
    Bytecode::PushFloat(FrostFloat::new(x).unwrap())
}

fn ints(xs: &[i64]) -> Value {
    Value::Array(FrostArray::from(xs.iter().copied().map(Value::Int).collect::<Vec<_>>()))
}

fn skey(s: &str) -> MapKey {
    MapKey::String(Arc::from(s.as_bytes()))
}

fn map(pairs: Vec<(MapKey, Value)>) -> Value {
    Value::Map(pairs.into_iter().collect())
}

use Bytecode::{LoadConst, Pop, PushInt, PushNull, SoftIndexStructure};

// ============================================================
// Array indexing
// ============================================================

#[test]
fn array_index_in_bounds() {
    let out = eval(vec![ints(&[10, 20, 30])], vec![LoadConst(0), PushInt(0), SoftIndexStructure]).unwrap();
    assert_eq!(out, Value::Int(10));
}

#[test]
fn array_index_last() {
    let out = eval(vec![ints(&[10, 20, 30])], vec![LoadConst(0), PushInt(2), SoftIndexStructure]).unwrap();
    assert_eq!(out, Value::Int(30));
}

#[test]
fn array_index_negative_counts_from_end() {
    let out = eval(vec![ints(&[10, 20, 30])], vec![LoadConst(0), PushInt(-1), SoftIndexStructure]).unwrap();
    assert_eq!(out, Value::Int(30));
}

#[test]
fn array_index_out_of_bounds_is_null() {
    let out = eval(vec![ints(&[10, 20, 30])], vec![LoadConst(0), PushInt(10), SoftIndexStructure]).unwrap();
    assert_eq!(out, Value::Null);
}

#[test]
fn array_index_negative_out_of_bounds_is_null() {
    let out = eval(vec![ints(&[10, 20, 30])], vec![LoadConst(0), PushInt(-10), SoftIndexStructure]).unwrap();
    assert_eq!(out, Value::Null);
}

#[test]
fn array_index_with_string_is_error() {
    let err = eval(
        vec![ints(&[1, 2]), Value::from("x")],
        vec![LoadConst(0), LoadConst(1), SoftIndexStructure],
    )
    .unwrap_err();
    assert!(err.message.contains("Array"), "got: {}", err.message);
}

#[test]
fn array_index_with_float_is_error() {
    let err = eval(vec![ints(&[1, 2])], vec![LoadConst(0), float(1.5), SoftIndexStructure]).unwrap_err();
    assert!(err.message.contains("Array"), "got: {}", err.message);
}

// ============================================================
// Map indexing
// ============================================================

#[test]
fn map_index_present_key() {
    let out = eval(
        vec![map(vec![(skey("a"), Value::Int(1))]), Value::from("a")],
        vec![LoadConst(0), LoadConst(1), SoftIndexStructure],
    )
    .unwrap();
    assert_eq!(out, Value::Int(1));
}

#[test]
fn map_index_missing_key_is_null() {
    let out = eval(
        vec![map(vec![(skey("a"), Value::Int(1))]), Value::from("z")],
        vec![LoadConst(0), LoadConst(1), SoftIndexStructure],
    )
    .unwrap();
    assert_eq!(out, Value::Null);
}

#[test]
fn map_index_with_int_key() {
    // Non-string primitive keys work: {1: "x"}[1] -> "x".
    let out = eval(
        vec![map(vec![(MapKey::Int(1), Value::from("x"))])],
        vec![LoadConst(0), PushInt(1), SoftIndexStructure],
    )
    .unwrap();
    assert_eq!(out, Value::from("x"));
}

#[test]
fn map_index_with_null_key_is_error() {
    let err = eval(
        vec![map(vec![(skey("a"), Value::Int(1))])],
        vec![LoadConst(0), PushNull, SoftIndexStructure],
    )
    .unwrap_err();
    assert!(err.message.contains("Map key"), "got: {}", err.message);
}

#[test]
fn map_index_with_structured_key_is_error() {
    // An array can never be a key, so indexing by one is a type error.
    let err = eval(
        vec![map(vec![(skey("a"), Value::Int(1))]), ints(&[1])],
        vec![LoadConst(0), LoadConst(1), SoftIndexStructure],
    )
    .unwrap_err();
    assert!(err.message.contains("Map key"), "got: {}", err.message);
}

// ============================================================
// Non-structure operands
// ============================================================

#[test]
fn index_into_string_is_error() {
    // Strings are not indexable via this opcode.
    let err = eval(
        vec![Value::from("abc")],
        vec![LoadConst(0), PushInt(0), SoftIndexStructure],
    )
    .unwrap_err();
    assert!(err.message.contains("index"), "got: {}", err.message);
}

#[test]
fn index_into_int_is_error() {
    let err = eval(vec![], vec![PushInt(5), PushInt(0), SoftIndexStructure]).unwrap_err();
    assert!(err.message.contains("index"), "got: {}", err.message);
}

// ============================================================
// Stack effect
// ============================================================

#[test]
fn consumes_structure_and_index_pushes_one() {
    // Sentinel below; index op consumes both structure and index, pushes one
    // result; Pop drops it, revealing the sentinel -- `( structure index -- r )`.
    let out = eval(
        vec![ints(&[7, 8, 9])],
        vec![PushInt(99), LoadConst(0), PushInt(1), SoftIndexStructure, Pop],
    )
    .unwrap();
    assert_eq!(out, Value::Int(99));
}
