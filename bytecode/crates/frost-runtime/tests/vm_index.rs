//! Tests for `SoftIndexStructure` -- the `a[b]` opcode with null-on-missing semantics.
//! Stack: `( structure index -- result )` (structure deeper).
//!
//! It indexes an Array (by Int) or a Map (by primitive key).
//! Distinguishing *missing* (-> null) from a *type error* is the whole point, so the tests pin both outcomes (cross-checked against the C++ oracle):
//!   * Array + Int -> element, or null when out of bounds (negatives count from the end); Array + non-Int -> error.
//!   * Map + valid key -> value, or null when the key is absent; Map + null or structured key -> error.
//!   * Indexing a non-structure (String, Int, ...) -> error.
//!
//! `HardIndexMap` (`foo.bar`) is the Map-only counterpart. Stack: `( map -- value )` -- the key is a String constant read from the const pool, not a stack operand.
//! A missing key is an ERROR (an intentional deviation from the oracle's null-on-missing), and non-map operands error too (arrays are not dot-indexable).

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostArray, FrostError, FrostFloat, MapKey,
    Value, Vm,
};

fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<index>".to_string(),
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

fn float(x: f64) -> Bytecode {
    Bytecode::PushFloat(FrostFloat::new(x).unwrap())
}

fn ints(xs: &[i64]) -> Value {
    Value::Array(FrostArray::from(
        xs.iter().copied().map(Value::Int).collect::<Vec<_>>(),
    ))
}

fn skey(s: &str) -> MapKey {
    MapKey::String(Arc::from(s.as_bytes()))
}

fn map(pairs: Vec<(MapKey, Value)>) -> Value {
    Value::Map(pairs.into_iter().collect())
}

use Bytecode::{HardIndexMap, LoadConst, Pop, PushInt, PushNull, SoftIndexStructure};

// ============================================================
// Array indexing
// ============================================================

#[test]
fn array_index_in_bounds() {
    let out = eval(
        vec![ints(&[10, 20, 30])],
        vec![LoadConst(0), PushInt(0), SoftIndexStructure],
    )
    .unwrap();
    assert_eq!(out, Value::Int(10));
}

#[test]
fn array_index_last() {
    let out = eval(
        vec![ints(&[10, 20, 30])],
        vec![LoadConst(0), PushInt(2), SoftIndexStructure],
    )
    .unwrap();
    assert_eq!(out, Value::Int(30));
}

#[test]
fn array_index_negative_counts_from_end() {
    let out = eval(
        vec![ints(&[10, 20, 30])],
        vec![LoadConst(0), PushInt(-1), SoftIndexStructure],
    )
    .unwrap();
    assert_eq!(out, Value::Int(30));
}

#[test]
fn array_index_out_of_bounds_is_null() {
    let out = eval(
        vec![ints(&[10, 20, 30])],
        vec![LoadConst(0), PushInt(10), SoftIndexStructure],
    )
    .unwrap();
    assert_eq!(out, Value::Null);
}

#[test]
fn array_index_negative_out_of_bounds_is_null() {
    let out = eval(
        vec![ints(&[10, 20, 30])],
        vec![LoadConst(0), PushInt(-10), SoftIndexStructure],
    )
    .unwrap();
    assert_eq!(out, Value::Null);
}

#[test]
fn array_index_with_string_is_error() {
    let err = eval(
        vec![ints(&[1, 2]), Value::from("x")],
        vec![LoadConst(0), LoadConst(1), SoftIndexStructure],
    )
    .unwrap_err();
    assert!(err.message().contains("Array"), "got: {}", err.message());
}

#[test]
fn array_index_with_float_is_error() {
    let err = eval(
        vec![ints(&[1, 2])],
        vec![LoadConst(0), float(1.5), SoftIndexStructure],
    )
    .unwrap_err();
    assert!(err.message().contains("Array"), "got: {}", err.message());
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
    assert!(err.message().contains("Map key"), "got: {}", err.message());
}

#[test]
fn map_index_with_structured_key_is_error() {
    // An array can never be a key, so indexing by one is a type error.
    let err = eval(
        vec![map(vec![(skey("a"), Value::Int(1))]), ints(&[1])],
        vec![LoadConst(0), LoadConst(1), SoftIndexStructure],
    )
    .unwrap_err();
    assert!(err.message().contains("Map key"), "got: {}", err.message());
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
    assert!(err.message().contains("index"), "got: {}", err.message());
}

#[test]
fn index_into_int_is_error() {
    let err = eval(vec![], vec![PushInt(5), PushInt(0), SoftIndexStructure]).unwrap_err();
    assert!(err.message().contains("index"), "got: {}", err.message());
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
        vec![
            PushInt(99),
            LoadConst(0),
            PushInt(1),
            SoftIndexStructure,
            Pop,
        ],
    )
    .unwrap();
    assert_eq!(out, Value::Int(99));
}

// ============================================================
// HardIndexMap (`foo.bar`) -- Map-only, compile-time key, error on missing
// ============================================================

#[test]
fn hard_index_present_key() {
    // {bar: 1}.bar -> 1. Key "bar" is the constant at index 1.
    let out = eval(
        vec![map(vec![(skey("bar"), Value::Int(1))]), Value::from("bar")],
        vec![LoadConst(0), HardIndexMap(1)],
    )
    .unwrap();
    assert_eq!(out, Value::Int(1));
}

#[test]
fn hard_index_missing_key_is_error() {
    // {bar: 1}.baz -> error. The oracle returns null here; erroring is the
    // intentional deviation.
    let err = eval(
        vec![map(vec![(skey("bar"), Value::Int(1))]), Value::from("baz")],
        vec![LoadConst(0), HardIndexMap(1)],
    )
    .unwrap_err();
    assert!(!err.message().is_empty());
}

#[test]
fn hard_index_present_key_with_null_value_is_not_missing() {
    // {bar: null}.bar -> null: the key is present, so the stored null is returned
    // rather than erroring. Distinguishes "present but null" from "missing".
    let out = eval(
        vec![map(vec![(skey("bar"), Value::Null)]), Value::from("bar")],
        vec![LoadConst(0), HardIndexMap(1)],
    )
    .unwrap();
    assert_eq!(out, Value::Null);
}

#[test]
fn hard_index_non_map_is_error() {
    // 5.bar -> error: only maps are dot-indexable. (Key const at index 0.)
    let err = eval(vec![Value::from("bar")], vec![PushInt(5), HardIndexMap(0)]).unwrap_err();
    assert!(err.message().contains("index"), "got: {}", err.message());
}

#[test]
fn hard_index_array_is_error() {
    // Arrays are not dot-indexable -- the reason this opcode is Map-specific.
    let err = eval(
        vec![ints(&[1, 2]), Value::from("bar")],
        vec![LoadConst(0), HardIndexMap(1)],
    )
    .unwrap_err();
    assert!(err.message().contains("index"), "got: {}", err.message());
}

#[test]
fn hard_index_consumes_only_the_map() {
    // Sentinel below; the key is from the const pool, so only the map is
    // consumed. Pop drops the result, revealing the sentinel -- `( map -- value )`.
    let out = eval(
        vec![map(vec![(skey("bar"), Value::Int(1))]), Value::from("bar")],
        vec![PushInt(99), LoadConst(0), HardIndexMap(1), Pop],
    )
    .unwrap();
    assert_eq!(out, Value::Int(99));
}
