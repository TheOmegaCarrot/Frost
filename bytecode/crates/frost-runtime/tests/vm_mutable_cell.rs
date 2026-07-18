//! Tests for the `mutable_cell` global: Frost's only built-in mutable state.
//!
//! `mutable_cell()` / `mutable_cell(initial)` returns a Map of two closures over a
//! shared `Arc<Mutex<Value>>`:
//!   `.get()`          -> the current value
//!   `.exchange(next)` -> sets the cell to `next`, returns the previous value
//! Functions may not be stored (directly or nested inside a structure).
//!
//! Each scenario runs real bytecode: it calls the global to build the cell, then
//! indexes the returned Map (`HardIndexMap`) and `Call`s the closures: the same
//! path compiled `cell.get()` / `cell.exchange(x)` would take.

use std::collections::BTreeMap;
use std::sync::Arc;

mod common;

use common::{Pop, global_slot as slot};
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostArray, FrostError, MapKey, NameEntry,
    NativeFunction, Value, Vm,
};

use Bytecode::*;

// ============================================================
// Harness
// ============================================================

fn entry(name: &str) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported: false,
    }
}

/// Run a "main" that pops its own value, runs `body`, with `caps` seated as captures
/// (slot `i` is the i-th entry) and `constants` available to const-indexed opcodes.
/// Returns the tail value, or the raised error.
fn run(
    caps: Vec<(&str, Value)>,
    constants: Vec<Value>,
    body: Vec<Bytecode>,
) -> Result<Value, FrostError> {
    let name_table = caps.iter().map(|(n, _)| entry(n)).collect();
    let mut code = vec![Pop];
    code.extend(body);
    let main = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        code,
        child_fns: Vec::new(),
        constants,
        name_table,
        num_captures: caps.len(),
        arity: Arity::Exact(0),
    });
    let captures: BTreeMap<String, Value> =
        caps.into_iter().map(|(n, v)| (n.to_string(), v)).collect();
    Vm::factory()
        .build(main.assert_trusted().close(captures).unwrap())
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
        .map(|r| r.tail().clone())
}

/// `mutable_cell(initial?)`: returns the cell Map, or the raised error.
fn new_cell(initial: Option<Value>) -> Result<Value, FrostError> {
    let mut body = vec![LoadGlobal(slot("mutable_cell"))];
    let (caps, argc) = match initial {
        Some(v) => {
            body.push(LoadLocal(0));
            (vec![("init", v)], 1)
        }
        None => (vec![], 0),
    };
    body.push(Call(argc));
    run(caps, vec![], body)
}

/// `mutable_cell(initial?)`, unwrapping the success value.
fn cell(initial: Option<Value>) -> Value {
    new_cell(initial).unwrap()
}

/// Invoke `cell.<method>(arg?)`: indexes the Map for the closure and calls it.
/// `get` passes `None`; `exchange` passes `Some(value)`.
fn invoke(cell: &Value, method: &'static str, arg: Option<Value>) -> Result<Value, FrostError> {
    let mut caps = vec![("cell", cell.clone())];
    let mut body = vec![LoadLocal(0), HardIndexMap(0)]; // cell -> the method closure
    let argc = match arg {
        Some(v) => {
            caps.push(("arg", v));
            body.push(LoadLocal(1));
            1
        }
        None => 0,
    };
    body.push(Call(argc));
    run(caps, vec![Value::from(method)], body)
}

/// `cell.get()`, unwrapping.
fn get(cell: &Value) -> Value {
    invoke(cell, "get", None).unwrap()
}

/// `cell.exchange(value)`, unwrapping.
fn exchange(cell: &Value, value: Value) -> Value {
    invoke(cell, "exchange", Some(value)).unwrap()
}

/// A throwaway Function value, which a cell may not hold.
fn a_function() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(
        "noop",
        Arity::Exact(0),
        |_, _: &mut [Value]| Ok(Value::Null),
    )))
}

/// An opaque host value, also forbidden since a cell can't see inside it to rule out a cycle.
fn an_opaque() -> Value {
    Value::Opaque(Arc::new(42i64))
}

fn arr(elems: Vec<Value>) -> Value {
    Value::Array(FrostArray::from(elems))
}

fn map(pairs: Vec<(&str, Value)>) -> Value {
    Value::Map(
        pairs
            .into_iter()
            .map(|(k, v)| (MapKey::String(Arc::from(k.as_bytes())), v))
            .collect(),
    )
}

// ============================================================
// get / exchange semantics
// ============================================================

#[test]
fn init_defaults_to_null() {
    assert_eq!(get(&cell(None)), Value::Null);
}

#[test]
fn get_reads_the_initial_value() {
    assert_eq!(get(&cell(Some(Value::Int(42)))), Value::Int(42));
}

#[test]
fn get_is_repeatable_without_mutating() {
    let c = cell(Some(Value::from("hi")));
    assert_eq!(get(&c), Value::from("hi"));
    assert_eq!(get(&c), Value::from("hi"));
}

#[test]
fn exchange_returns_previous_and_installs_new() {
    let c = cell(Some(Value::Int(1)));
    // exchange yields the *previous* value...
    assert_eq!(exchange(&c, Value::Int(2)), Value::Int(1));
    // ...and the new value is now current.
    assert_eq!(get(&c), Value::Int(2));
}

#[test]
fn exchange_from_the_null_default() {
    let c = cell(None);
    assert_eq!(exchange(&c, Value::Int(5)), Value::Null);
    assert_eq!(get(&c), Value::Int(5));
}

#[test]
fn successive_exchanges_chain() {
    let c = cell(Some(Value::Int(0)));
    assert_eq!(exchange(&c, Value::Int(1)), Value::Int(0));
    assert_eq!(exchange(&c, Value::Int(2)), Value::Int(1));
    assert_eq!(get(&c), Value::Int(2));
}

#[test]
fn handles_to_the_same_cell_share_state() {
    // Cloning the cell Value clones the Map (an Arc bump); both clones' closures
    // capture the *same* `Arc<Mutex<Value>>`, so a write through one is seen by the other.
    let c = cell(Some(Value::Int(1)));
    let alias = c.clone();
    exchange(&alias, Value::Int(99));
    assert_eq!(get(&c), Value::Int(99));
}

// ============================================================
// forbid_cycle: Functions and Opaques may not be stored
// ============================================================

#[test]
fn rejects_a_function_at_init() {
    let err = new_cell(Some(a_function())).unwrap_err();
    assert!(
        err.message().contains("Function"),
        "unexpected message: {}",
        err.message()
    );
}

#[test]
fn rejects_a_function_via_exchange() {
    let c = cell(Some(Value::Int(7)));
    assert!(invoke(&c, "exchange", Some(a_function())).is_err());
    // The rejected exchange must not have mutated the cell.
    assert_eq!(get(&c), Value::Int(7));
}

#[test]
fn rejects_a_function_nested_in_an_array() {
    // forbid_cycle recurses into structures: a function buried in an array is caught.
    assert!(new_cell(Some(arr(vec![Value::Int(1), a_function()]))).is_err());
}

#[test]
fn rejects_a_function_nested_in_a_map() {
    assert!(new_cell(Some(map(vec![("f", a_function())]))).is_err());
}

#[test]
fn accepts_a_function_free_structure() {
    // The same shapes without a function are fine, and round-trip intact.
    let nested = arr(vec![Value::Int(1), map(vec![("k", Value::from("v"))])]);
    assert_eq!(get(&cell(Some(nested.clone()))), nested);
}

#[test]
fn rejects_an_opaque_at_init() {
    // Opaque can smuggle a cycle we can't inspect, so it's forbidden wholesale.
    let err = new_cell(Some(an_opaque())).unwrap_err();
    assert!(
        err.message().contains("Opaque"),
        "unexpected message: {}",
        err.message()
    );
}

#[test]
fn rejects_an_opaque_nested_in_a_structure() {
    assert!(new_cell(Some(arr(vec![an_opaque()]))).is_err());
    assert!(new_cell(Some(map(vec![("o", an_opaque())]))).is_err());
}
