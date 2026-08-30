//! Tests for the `TestKey` opcode.
//!
//! `TestKey` `( m k -- m k b )` is a non-consuming query: it pushes whether Map m
//! holds key k, leaving m and k in place beneath the bool. It pushes false when the
//! key is absent or m is not a Map, and raises when k is not a valid key type
//! (Null / Array / Map / Function) -- an error, not a match failure.

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, FrostFloat, Value, Vm,
};

/// Run a constants-carrying, local-less top-level program and return its tail value.
fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<test-key>".to_string(),
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

/// Load `map` and `key` as the two constants, run `TestKey`, then drop both operands
/// from beneath the result so only the pushed bool remains as the tail value.
fn test_key(map: Value, key: Value) -> Result<Value, FrostError> {
    eval(
        vec![map, key],
        vec![
            Bytecode::LoadConst(0), // m
            Bytecode::LoadConst(1), // k
            Bytecode::TestKey,
            Bytecode::DropBelow(1), // drop k, leaving [m, b]
            Bytecode::DropBelow(1), // drop m, leaving [b]
        ],
    )
}

// ---- Present / absent, across key types ----

#[test]
fn present_string_key_is_true() {
    let map = Value::map([("a", Value::from(1i64))]);
    assert_eq!(test_key(map, Value::from("a")).unwrap(), Value::from(true));
}

#[test]
fn absent_string_key_is_false() {
    let map = Value::map([("a", Value::from(1i64))]);
    assert_eq!(test_key(map, Value::from("z")).unwrap(), Value::from(false));
}

#[test]
fn absent_in_empty_map_is_false() {
    let map = Value::Map(Default::default());
    assert_eq!(test_key(map, Value::from("a")).unwrap(), Value::from(false));
}

#[test]
fn present_int_key_is_true() {
    let map = Value::map([(1i64, Value::from("x"))]);
    assert_eq!(test_key(map, Value::from(1i64)).unwrap(), Value::from(true));
}

#[test]
fn present_bool_key_is_true() {
    let map = Value::map([(true, Value::from(1i64))]);
    assert_eq!(test_key(map, Value::from(true)).unwrap(), Value::from(true));
}

#[test]
fn present_float_key_is_true() {
    let map = Value::map([(FrostFloat::new(1.5).unwrap(), Value::from(1i64))]);
    let key: Value = 1.5.try_into().unwrap();
    assert_eq!(test_key(map, key).unwrap(), Value::from(true));
}

#[test]
fn present_bytes_key_is_true() {
    let map = Value::map([(b"hi".to_vec(), Value::from(1i64))]);
    assert_eq!(
        test_key(map, Value::from(b"hi".to_vec())).unwrap(),
        Value::from(true)
    );
}

#[test]
fn valid_key_of_mismatched_type_is_false() {
    // The map is string-keyed; an Int is a valid key type but not present, so this
    // is a plain false, not a raise. (Int 1 and Float 1.0 are also distinct keys.)
    let map = Value::map([("1", Value::from("x"))]);
    assert_eq!(
        test_key(map, Value::from(1i64)).unwrap(),
        Value::from(false)
    );
}

// ---- Non-Map operand: false, never a raise ----

#[test]
fn non_map_is_false() {
    // Regression: the non-Map branch must fall through and advance pc, not loop.
    assert_eq!(
        test_key(Value::from(5i64), Value::from("a")).unwrap(),
        Value::from(false)
    );
}

#[test]
fn non_map_with_invalid_key_type_is_false() {
    // Non-Map short-circuits to false before the key type is validated, so an
    // otherwise-invalid key does not raise here.
    let bad_key = Value::array([Value::from(1i64)]);
    assert_eq!(
        test_key(Value::from(5i64), bad_key).unwrap(),
        Value::from(false)
    );
}

// ---- Invalid key type against a Map: raises ----

#[test]
fn null_key_raises() {
    let map = Value::map([("a", Value::from(1i64))]);
    assert!(test_key(map, Value::Null).is_err());
}

#[test]
fn array_key_raises() {
    let map = Value::map([("a", Value::from(1i64))]);
    assert!(test_key(map, Value::array([Value::from(1i64)])).is_err());
}

#[test]
fn map_key_raises() {
    let map = Value::map([("a", Value::from(1i64))]);
    let bad_key = Value::map([("x", Value::from(1i64))]);
    assert!(test_key(map, bad_key).is_err());
}

// ---- Non-consuming contract ( m k -- m k b ) ----

#[test]
fn leaves_map_and_key_beneath_bool() {
    // Fold the post-test stack into an array so all three cells are observable:
    // m and k must remain, in order, beneath the pushed bool.
    let map = Value::map([("a", Value::from(1i64))]);
    let r = eval(
        vec![map.clone(), Value::from("a")],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::LoadConst(1),
            Bytecode::TestKey,
            Bytecode::MakeArray(3),
        ],
    );
    assert_eq!(
        r.unwrap(),
        Value::array([map, Value::from("a"), Value::from(true)])
    );
}
