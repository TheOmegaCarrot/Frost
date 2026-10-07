//! Tests for the `TestConstKey` opcode.
//!
//! `TestConstKey(n)` `( m -- m b )` is a non-consuming query: it pushes whether Map
//! m holds entry n of the function's key constants, leaving m in place beneath the
//! bool. It pushes false when the key is absent or m is not a Map; it never raises.

use std::sync::Arc;

use frostlang::bytecode::{Bytecode, CompiledFunction, FormatVersion};
use frostlang::{Arity, FrostError, FrostFloat, MapKey, Value, Vm};

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
        name: "<test-const-key>".to_string(),
        origin: None,
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
        .map_err(frostlang::RunError::into_error)
        .map(|r| r.tail().clone())
}

/// Load `map`, run `TestConstKey` for `key`, then drop the map from beneath the
/// result so only the pushed bool remains as the tail value.
fn test_const_key(map: Value, key: MapKey) -> Value {
    eval(
        vec![map],
        vec![key],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::TestConstKey(0),
            Bytecode::DropBelow(1),
        ],
    )
    .unwrap()
}

// ---- Present / absent, across key types ----

#[test]
fn a_present_key_of_each_type_is_true() {
    let float = FrostFloat::new(1.5).unwrap();
    for (map, key) in [
        (Value::map([("a", Value::Int(1))]), MapKey::from("a")),
        (Value::map([(1i64, Value::Int(1))]), MapKey::from(1i64)),
        (Value::map([(true, Value::Int(1))]), MapKey::from(true)),
        (Value::map([(float, Value::Int(1))]), MapKey::from(float)),
        (
            Value::map([(b"hi".to_vec(), Value::Int(1))]),
            MapKey::from(b"hi".to_vec()),
        ),
    ] {
        assert_eq!(
            test_const_key(map, key.clone()),
            Value::Bool(true),
            "{key:?}"
        );
    }
}

#[test]
fn an_absent_key_is_false() {
    let map = Value::map([("a", Value::Int(1))]);
    assert_eq!(test_const_key(map, MapKey::from("z")), Value::Bool(false));
    let empty = Value::Map(Default::default());
    assert_eq!(test_const_key(empty, MapKey::from("a")), Value::Bool(false));
}

#[test]
fn a_key_of_another_type_is_false() {
    // Int 1 is a valid key but not the String "1", nor the Float 1.0.
    let map = Value::map([("1", Value::Int(1))]);
    assert_eq!(test_const_key(map, MapKey::from(1i64)), Value::Bool(false));
    let map = Value::map([(1i64, Value::Int(1))]);
    let one = MapKey::from(FrostFloat::new(1.0).unwrap());
    assert_eq!(test_const_key(map, one), Value::Bool(false));
}

// ---- Non-Map operand: false, never a raise ----

#[test]
fn a_non_map_is_false() {
    for value in [
        Value::Null,
        Value::Int(5),
        Value::from("a"),
        Value::array([Value::from("a")]),
    ] {
        assert_eq!(
            test_const_key(value.clone(), MapKey::from("a")),
            Value::Bool(false),
            "{value:?}"
        );
    }
}

// ---- Non-consuming contract ( m -- m b ) ----

#[test]
fn leaves_the_map_beneath_the_bool() {
    let map = Value::map([("a", Value::Int(1))]);
    let r = eval(
        vec![map.clone()],
        vec![MapKey::from("a")],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::TestConstKey(0),
            Bytecode::MakeArray(2),
        ],
    );
    assert_eq!(r.unwrap(), Value::array([map, Value::Bool(true)]));
}

#[test]
fn the_index_selects_the_key_constant() {
    let map = Value::map([("b", Value::Int(1))]);
    let r = eval(
        vec![map],
        vec![MapKey::from("a"), MapKey::from("b")],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::TestConstKey(0),
            Bytecode::DropBelow(1),
            Bytecode::LoadConst(0),
            Bytecode::TestConstKey(1),
            Bytecode::DropBelow(1),
            Bytecode::MakeArray(2),
        ],
    );
    assert_eq!(
        r.unwrap(),
        Value::array([Value::Bool(false), Value::Bool(true)])
    );
}
