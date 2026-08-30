//! Tests for the `SplitArray` opcode.
//!
//! `SplitArray(N)` `( [X] -- [N] [X-N] )` consumes an Array and pushes two Arrays:
//! the first N elements (beneath) and the remaining X-N (on top, possibly empty).
//! It raises when the operand is not an Array, or is shorter than N.

use std::sync::Arc;

use frost_runtime::{Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, Value, Vm};

/// Run a constants-carrying, local-less top-level program and return its tail value.
fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<split-array>".to_string(),
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

fn iarr(xs: &[i64]) -> Value {
    Value::from(xs.iter().copied().map(Value::from).collect::<Vec<_>>())
}

/// Split `operand` at `n` and fold the two resulting Arrays into one Array `[head, tail]`
/// so both outputs, and their order, are observable as a single tail value.
fn split(operand: Value, n: usize) -> Result<Value, FrostError> {
    eval(
        vec![operand],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::SplitArray(n),
            Bytecode::MakeArray(2),
        ],
    )
}

// ---- Splitting: head below, tail on top ----

#[test]
fn splits_in_the_middle() {
    assert_eq!(
        split(iarr(&[10, 20, 30, 40]), 2).unwrap(),
        Value::array([iarr(&[10, 20]), iarr(&[30, 40])])
    );
}

#[test]
fn split_of_one_leaves_the_rest_as_tail() {
    assert_eq!(
        split(iarr(&[10, 20, 30]), 1).unwrap(),
        Value::array([iarr(&[10]), iarr(&[20, 30])])
    );
}

#[test]
fn split_at_full_length_gives_empty_tail() {
    assert_eq!(
        split(iarr(&[10, 20]), 2).unwrap(),
        Value::array([iarr(&[10, 20]), iarr(&[])])
    );
}

#[test]
fn split_at_zero_gives_empty_head() {
    assert_eq!(
        split(iarr(&[10, 20]), 0).unwrap(),
        Value::array([iarr(&[]), iarr(&[10, 20])])
    );
}

#[test]
fn empty_array_split_at_zero_gives_two_empties() {
    assert_eq!(
        split(iarr(&[]), 0).unwrap(),
        Value::array([iarr(&[]), iarr(&[])])
    );
}

// ---- Too short, non-Array: raise ----

#[test]
fn shorter_than_n_raises() {
    assert!(split(iarr(&[10]), 2).is_err());
}

#[test]
fn empty_array_with_nonzero_n_raises() {
    assert!(split(iarr(&[]), 1).is_err());
}

#[test]
fn non_array_raises() {
    assert!(split(Value::from(5i64), 1).is_err());
}

#[test]
fn non_array_error_names_the_actual_type() {
    let err = split(Value::from(5i64), 1).unwrap_err();
    assert!(
        err.message().contains("Expected Array") && err.message().contains("Int"),
        "got: {}",
        err.message()
    );
}
