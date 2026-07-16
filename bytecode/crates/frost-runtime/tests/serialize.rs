//! Versioned (de)serialization of a `CompiledFunction`.
//!
//! Round-trips through both a self-describing format (JSON) and a non-self-describing
//! binary one (postcard, the case that forces the tagged `ConstValue` representation),
//! confirms the version gate rejects a mismatched image, and confirms a function-valued
//! constant cannot be serialized (an error, never a panic).

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostArray, FrostMap, MapKey, NameEntry,
    Value,
};

/// A function tree exercising every serialized shape: a child fn (recursion), and constants
/// covering a string, an int, a nested array with a null, and a map with a *non-string* key.
fn sample() -> CompiledFunction {
    let child = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "child".into(),
        code: vec![Bytecode::Pop, Bytecode::PushInt(1)],
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let int_keyed_map: FrostMap = vec![(MapKey::Int(1), Value::from("one"))]
        .into_iter()
        .collect();
    CompiledFunction {
        version: FormatVersion,
        name: "main".into(),
        code: vec![
            Bytecode::Pop,
            Bytecode::LoadConst(0),
            Bytecode::PushFloat(2.5.try_into().unwrap()),
        ],
        child_fns: vec![child],
        constants: vec![
            Value::from("hello"),
            Value::from(42i64),
            Value::from(FrostArray::from(vec![Value::from(1i64), Value::Null])),
            Value::from(int_keyed_map),
        ],
        name_table: vec![NameEntry {
            name: "x".into(),
            exported: true,
        }],
        num_captures: 0,
        arity: Arity::Between(1, 2),
    }
}

fn assert_matches(original: &CompiledFunction, restored: &CompiledFunction) {
    assert_eq!(restored.name, original.name);
    // `Value: PartialEq` covers the whole `ConstValue` round-trip, including the Int-keyed map.
    assert_eq!(restored.constants, original.constants);
    assert_eq!(restored.arity, original.arity);
    assert_eq!(restored.num_captures, original.num_captures);
    assert_eq!(restored.child_fns.len(), original.child_fns.len());
    assert_eq!(restored.child_fns[0].name, original.child_fns[0].name);
}

fn a_closure_value() -> Value {
    let f = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "f".into(),
        code: Vec::new(),
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    Value::Closure(f.assert_trusted().into_closure().unwrap())
}

#[test]
fn roundtrips_through_postcard() {
    let cf = sample();
    let bytes = postcard::to_allocvec(&cf).unwrap();
    let restored: CompiledFunction = postcard::from_bytes(&bytes).unwrap();
    assert_matches(&cf, &restored);
    // Re-serialization is byte-identical: full fidelity.
    assert_eq!(postcard::to_allocvec(&restored).unwrap(), bytes);
}

#[test]
fn roundtrips_through_json() {
    let cf = sample();
    let json = serde_json::to_string(&cf).unwrap();
    let restored: CompiledFunction = serde_json::from_str(&json).unwrap();
    assert_matches(&cf, &restored);
}

#[test]
fn a_version_mismatch_is_rejected() {
    let cf = sample();
    // Serialize, then rewrite the stamped version to one this runtime is not.
    let mut image = serde_json::to_value(&cf).unwrap();
    image["version"] = serde_json::Value::from("0.0.0-not-this-runtime");
    let err = serde_json::from_value::<CompiledFunction>(image).unwrap_err();
    assert!(err.to_string().contains("0.0.0"), "unexpected error: {err}");
}

#[test]
fn a_function_valued_constant_cannot_be_serialized() {
    // A function never legitimately reaches a constant pool, but the serializer must
    // reject one with an error rather than panicking.
    let cf = CompiledFunction {
        version: FormatVersion,
        name: "bad".into(),
        code: Vec::new(),
        child_fns: Vec::new(),
        constants: vec![a_closure_value()],
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    };
    assert!(postcard::to_allocvec(&cf).is_err());
}
