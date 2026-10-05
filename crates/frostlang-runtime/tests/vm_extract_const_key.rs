//! Tests for the `ExtractConstKey` opcode.
//!
//! `ExtractConstKey(n)` `( m -- m v )` looks up entry n of the function's key
//! constants in Map m, pushing the value and leaving m in place beneath it for a
//! following entry. It raises when the key is absent or m is not a Map.

use std::sync::Arc;

use frostlang_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, FrostFloat, MapKey, Value, Vm,
};

/// Run a local-less top-level program with `constants` and `key_constants`, and
/// return its tail value.
fn eval(
    constants: Vec<Value>,
    key_constants: Vec<MapKey>,
    code: Vec<Bytecode>,
) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<extract-const-key>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants,
        key_constants,
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.assert_trusted().into_closure().unwrap();
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .map_err(frostlang_runtime::RunError::into_error)
        .map(|r| r.tail().clone())
}

/// Load `map`, run `ExtractConstKey` for `key`, then drop the map from beneath the
/// result so only the extracted value remains as the tail value.
fn extract_const_key(map: Value, key: MapKey) -> Result<Value, FrostError> {
    eval(
        vec![map],
        vec![key],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::ExtractConstKey(0),
            Bytecode::DropBelow(1),
        ],
    )
}

// ---- Present keys, across key types ----

#[test]
fn a_present_key_of_each_type_gives_its_value() {
    let float = FrostFloat::new(1.5).unwrap();
    for (map, key) in [
        (Value::map([("a", Value::Int(42))]), MapKey::from("a")),
        (Value::map([(1i64, Value::Int(42))]), MapKey::from(1i64)),
        (Value::map([(false, Value::Int(42))]), MapKey::from(false)),
        (Value::map([(float, Value::Int(42))]), MapKey::from(float)),
        (
            Value::map([(b"hi".to_vec(), Value::Int(42))]),
            MapKey::from(b"hi".to_vec()),
        ),
    ] {
        assert_eq!(
            extract_const_key(map, key.clone()).unwrap(),
            Value::Int(42),
            "{key:?}"
        );
    }
}

#[test]
fn a_present_key_holding_null_gives_null() {
    let map = Value::map([("a", Value::Null)]);
    assert_eq!(
        extract_const_key(map, MapKey::from("a")).unwrap(),
        Value::Null
    );
}

// ---- Absent key and non-Map: raise ----

#[test]
fn an_absent_key_raises_naming_it() {
    let map = Value::map([("a", Value::Int(1))]);
    let error = extract_const_key(map, MapKey::from("z")).unwrap_err();
    assert!(
        error.message().contains("no value at key 'z'"),
        "{}",
        error.message()
    );
}

#[test]
fn an_absent_key_suggests_a_similar_string_key() {
    let map = Value::map([("name", Value::Int(1))]);
    let error = extract_const_key(map, MapKey::from("nmae")).unwrap_err();
    assert_eq!(
        error.message(),
        "Map has no value at key 'nmae'; did you mean 'name'?"
    );
}

#[test]
fn a_key_of_another_type_raises() {
    let map = Value::map([("1", Value::Int(1))]);
    assert!(extract_const_key(map, MapKey::from(1i64)).is_err());
}

#[test]
fn a_non_map_raises() {
    for value in [Value::Null, Value::Int(5), Value::array([Value::from("a")])] {
        let error = extract_const_key(value.clone(), MapKey::from("a")).unwrap_err();
        assert!(
            error.message().contains("Expected Map"),
            "{value:?}: {}",
            error.message()
        );
    }
}

// ---- Stack effect ( m -- m v ): m kept ----

#[test]
fn leaves_the_map_beneath_the_value() {
    let map = Value::map([("a", Value::Int(42))]);
    let r = eval(
        vec![map.clone()],
        vec![MapKey::from("a")],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::ExtractConstKey(0),
            Bytecode::MakeArray(2),
        ],
    );
    assert_eq!(r.unwrap(), Value::array([map, Value::Int(42)]));
}
