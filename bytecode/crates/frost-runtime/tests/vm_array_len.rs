//! Tests for the `TestArrayLenExact` / `TestArrayLenAtLeast` opcodes.
//!
//! Both are non-consuming length queries with stack effect `( x -- x b )`: they
//! push whether the operand is an Array of the required length, leaving the
//! operand in place beneath the pushed bool. A non-Array operand pushes false.

use std::sync::Arc;

use frost_runtime::{Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, Value, Vm};

/// Run a constants-carrying, local-less top-level program and return its tail value.
fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<array-len>".to_string(),
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
        .map_err(|e| e.into_error())
        .map(|r| r.tail().clone())
}

fn arr(len: usize) -> Value {
    Value::from((0..len as i64).map(Value::from).collect::<Vec<_>>())
}

/// Load `operand` as the sole constant, run `op` against it, then drop the operand
/// from beneath the result so only the pushed bool remains as the tail value.
fn test_len(operand: Value, op: Bytecode) -> Value {
    eval(
        vec![operand],
        vec![Bytecode::LoadConst(0), op, Bytecode::DropBelow(1)],
    )
    .unwrap()
}

// ---- Exact ----

#[test]
fn exact_matching_length_is_true() {
    assert_eq!(test_len(arr(2), Bytecode::TestArrayLenExact(2)), Value::from(true));
}

#[test]
fn exact_longer_is_false() {
    assert_eq!(test_len(arr(3), Bytecode::TestArrayLenExact(2)), Value::from(false));
}

#[test]
fn exact_shorter_is_false() {
    assert_eq!(test_len(arr(1), Bytecode::TestArrayLenExact(2)), Value::from(false));
}

#[test]
fn exact_zero_matches_empty_array() {
    assert_eq!(test_len(arr(0), Bytecode::TestArrayLenExact(0)), Value::from(true));
}

#[test]
fn exact_non_array_is_false() {
    assert_eq!(test_len(Value::from(5i64), Bytecode::TestArrayLenExact(0)), Value::from(false));
}

// ---- AtLeast ----

#[test]
fn at_least_equal_is_true() {
    assert_eq!(test_len(arr(2), Bytecode::TestArrayLenAtLeast(2)), Value::from(true));
}

#[test]
fn at_least_more_is_true() {
    assert_eq!(test_len(arr(3), Bytecode::TestArrayLenAtLeast(2)), Value::from(true));
}

#[test]
fn at_least_fewer_is_false() {
    assert_eq!(test_len(arr(1), Bytecode::TestArrayLenAtLeast(2)), Value::from(false));
}

#[test]
fn at_least_zero_accepts_any_array() {
    assert_eq!(test_len(arr(3), Bytecode::TestArrayLenAtLeast(0)), Value::from(true));
}

#[test]
fn at_least_non_array_is_false() {
    assert_eq!(test_len(Value::from(5i64), Bytecode::TestArrayLenAtLeast(0)), Value::from(false));
}

// ---- Non-consuming contract ( x -- x b ) ----
// Fold the post-test stack into an array so both cells are observable: the
// operand must remain beneath the pushed bool.

#[test]
fn exact_leaves_operand_beneath_bool() {
    let r = eval(
        vec![arr(2)],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::TestArrayLenExact(2),
            Bytecode::MakeArray(2),
        ],
    );
    assert_eq!(r.unwrap(), Value::array([arr(2), Value::from(true)]));
}

#[test]
fn at_least_leaves_operand_beneath_bool() {
    let r = eval(
        vec![arr(1)],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::TestArrayLenAtLeast(2),
            Bytecode::MakeArray(2),
        ],
    );
    assert_eq!(r.unwrap(), Value::array([arr(1), Value::from(false)]));
}
