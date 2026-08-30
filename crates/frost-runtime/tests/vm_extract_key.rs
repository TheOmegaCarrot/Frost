//! Tests for the `ExtractKey` opcode.
//!
//! `ExtractKey` `( m k -- m v )` consumes the key k and replaces it with m[k],
//! leaving the Map m in place beneath the value for a following entry. It raises
//! when the key is absent, when m is not a Map, or when k is not a valid key type
//! (the key type is validated first, so an invalid key raises even against a non-Map).

use std::sync::Arc;

use frost_runtime::{Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, Value, Vm};

/// Run a constants-carrying, local-less top-level program and return its tail value.
fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<extract-key>".to_string(),
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

/// Load `map` and `key` as the two constants, run `ExtractKey`, then drop the map
/// from beneath the result so only the extracted value remains as the tail value.
fn extract_key(map: Value, key: Value) -> Result<Value, FrostError> {
    eval(
        vec![map, key],
        vec![
            Bytecode::LoadConst(0), // m
            Bytecode::LoadConst(1), // k
            Bytecode::ExtractKey,
            Bytecode::DropBelow(1), // drop m, leaving [v]
        ],
    )
}

// ---- Present keys, across key types ----

#[test]
fn present_string_key_returns_value() {
    let map = Value::map([("a", Value::from(42i64))]);
    assert_eq!(
        extract_key(map, Value::from("a")).unwrap(),
        Value::from(42i64)
    );
}

#[test]
fn present_int_key_returns_value() {
    let map = Value::map([(1i64, Value::from("x"))]);
    assert_eq!(
        extract_key(map, Value::from(1i64)).unwrap(),
        Value::from("x")
    );
}

#[test]
fn present_bytes_key_returns_value() {
    let map = Value::map([(b"hi".to_vec(), Value::from(7i64))]);
    assert_eq!(
        extract_key(map, Value::from(b"hi".to_vec())).unwrap(),
        Value::from(7i64)
    );
}

// ---- Absent / non-Map / invalid key: all raise ----

#[test]
fn absent_key_raises() {
    let map = Value::map([("a", Value::from(1i64))]);
    assert!(extract_key(map, Value::from("z")).is_err());
}

#[test]
fn absent_in_empty_map_raises() {
    let map = Value::Map(Default::default());
    assert!(extract_key(map, Value::from("a")).is_err());
}

#[test]
fn non_map_raises() {
    assert!(extract_key(Value::from(5i64), Value::from("a")).is_err());
}

#[test]
fn null_key_raises() {
    let map = Value::map([("a", Value::from(1i64))]);
    assert!(extract_key(map, Value::Null).is_err());
}

#[test]
fn array_key_raises() {
    let map = Value::map([("a", Value::from(1i64))]);
    assert!(extract_key(map, Value::array([Value::from(1i64)])).is_err());
}

#[test]
fn invalid_key_type_raises_even_for_non_map() {
    // The key type is validated before m's type, so an invalid key raises rather
    // than the non-Map error path being reached.
    let bad_key = Value::array([Value::from(1i64)]);
    assert!(extract_key(Value::from(5i64), bad_key).is_err());
}

// ---- Stack effect ( m k -- m v ): m kept, k consumed ----

#[test]
fn leaves_map_beneath_extracted_value() {
    // Fold the post-extract stack into an array so both cells are observable:
    // m must remain beneath the extracted value, with k consumed.
    let map = Value::map([("a", Value::from(42i64))]);
    let r = eval(
        vec![map.clone(), Value::from("a")],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::LoadConst(1),
            Bytecode::ExtractKey,
            Bytecode::MakeArray(2),
        ],
    );
    assert_eq!(r.unwrap(), Value::array([map, Value::from(42i64)]));
}
