//! Tests for the `each` global: a foreach loop surfaced as a function.
//!
//! `each(structure, f)` walks `structure` for side effects and returns it unchanged.
//! For an Array it calls `f` with each element, in order. For a Map it calls `f`
//! with each `(key, value)` pair, in the sequence defined by `MapKey`'s total
//! order (so the Map callback must be binary).
//!
//! The callback is a `record` native that appends its argument list to a shared
//! log, so a test can read back exactly what `each` invoked it with, and in what
//! order. `record`'s arity doubles as proof of the callback's argument count.

use std::sync::{Arc, Mutex};

mod common;

use common::{Pop, global_slot};
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostArray, FrostError, FrostResult, MapKey,
    NameEntry, NativeCtx, NativeFunction, Value, Vm,
};

use Bytecode::*;

// ============================================================
// Helpers
// ============================================================

fn entry(name: &str) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported: false,
    }
}

/// A native function `Value`.
fn native(
    name: &'static str,
    arity: Arity,
    f: impl Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static,
) -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(name, arity, f)))
}

/// A native that raises if it is ever invoked: the "must not be called" sentinel.
fn boom() -> Value {
    native("boom", Arity::AtLeast(0), |_, _| {
        Err(FrostError::from_static("boom!"))
    })
}

/// The ordered record of every callback invocation: one entry per call, each the
/// argument list that call received.
type CallLog = Arc<Mutex<Vec<Vec<Value>>>>;

/// A callback that appends its arguments to a shared log and returns null.
/// `arity` fixes the accepted argument count, so an arity mismatch surfaces as an error.
fn recorder(arity: Arity) -> (Value, CallLog) {
    let log: CallLog = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let f = native("record", arity, move |_, args: &mut [Value]| {
        sink.lock().unwrap().push(args.to_vec());
        Ok(Value::Null)
    });
    (f, log)
}

/// The calls recorded so far, as owned rows.
fn calls(log: &CallLog) -> Vec<Vec<Value>> {
    log.lock().unwrap().clone()
}

/// The first argument of each recorded call: the element (Array) or key (Map).
fn firsts(log: &CallLog) -> Vec<Value> {
    log.lock().unwrap().iter().map(|c| c[0].clone()).collect()
}

fn arr(values: Vec<Value>) -> Value {
    Value::Array(FrostArray::from(values))
}

fn map_of(pairs: Vec<(MapKey, Value)>) -> Value {
    Value::Map(pairs.into_iter().collect())
}

/// Run a top-level "main" that pops its own value then runs `body`, with `caps`
/// seated as captures (slot `i` is the i-th entry). Returns the tail value or the
/// raised error.
fn run_main(caps: Vec<(&str, Value)>, body: Vec<Bytecode>) -> Result<Value, FrostError> {
    let names: Vec<NameEntry> = caps.iter().map(|(n, _)| entry(n)).collect();
    let mut code = vec![Pop];
    code.extend(body);
    let main = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names,
        num_captures: caps.len(),
        arity: Arity::Exact(0),
    });
    let map = caps.into_iter().map(|(n, v)| (n.to_string(), v)).collect();
    Vm::factory()
        .build(main.assert_trusted().close(map).unwrap())
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
        .map(|r| r.tail().clone())
}

/// `each(structure, f)` through the full `LoadGlobal` + `Call` path.
fn each(structure: Value, f: Value) -> Result<Value, FrostError> {
    run_main(
        vec![("s", structure), ("f", f)],
        vec![
            LoadGlobal(global_slot("each")),
            LoadLocal(0),
            LoadLocal(1),
            Call(2),
        ],
    )
}

// ============================================================
// Array
// ============================================================

#[test]
fn array_calls_function_once_per_element_in_order() {
    let (f, log) = recorder(Arity::Exact(1));
    each(arr(vec![Value::Int(10), Value::Int(20), Value::Int(30)]), f).unwrap();
    assert_eq!(
        calls(&log),
        vec![
            vec![Value::Int(10)],
            vec![Value::Int(20)],
            vec![Value::Int(30)],
        ]
    );
}

#[test]
fn array_is_returned_unchanged() {
    let (f, _) = recorder(Arity::Exact(1));
    let input = arr(vec![Value::Int(1), Value::Int(2)]);
    let result = each(input.clone(), f).unwrap();
    assert_eq!(result, input);
}

#[test]
fn empty_array_never_calls_the_function() {
    // boom raises if called, so a clean run proves zero invocations.
    let result = each(arr(vec![]), boom()).unwrap();
    assert_eq!(result, arr(vec![]));
}

#[test]
fn array_callback_receives_exactly_one_argument() {
    // The callback is Exact(1): calling it with any other count would be an arity
    // error, so a clean run is itself the proof.
    let (f, log) = recorder(Arity::Exact(1));
    each(arr(vec![Value::from("a"), Value::from("b")]), f).unwrap();
    assert!(calls(&log).iter().all(|c| c.len() == 1));
}

// ============================================================
// Map
// ============================================================

#[test]
fn map_calls_function_with_each_key_value_pair() {
    let (f, log) = recorder(Arity::Exact(2));
    let m = map_of(vec![
        (MapKey::from("a"), Value::Int(1)),
        (MapKey::from("b"), Value::Int(2)),
    ]);
    each(m, f).unwrap();
    assert_eq!(
        calls(&log),
        vec![
            vec![Value::from("a"), Value::Int(1)],
            vec![Value::from("b"), Value::Int(2)],
        ]
    );
}

#[test]
fn map_visits_entries_in_mapkey_order_not_insertion_order() {
    // Int keys inserted out of order must be visited numerically ascending.
    let (f, log) = recorder(Arity::Exact(2));
    let m = map_of(vec![
        (MapKey::from(3i64), Value::Null),
        (MapKey::from(1i64), Value::Null),
        (MapKey::from(2i64), Value::Null),
    ]);
    each(m, f).unwrap();
    assert_eq!(
        firsts(&log),
        vec![Value::Int(1), Value::Int(2), Value::Int(3)]
    );
}

#[test]
fn map_visits_mixed_key_types_in_total_order() {
    // MapKey's total order is Bool < Int < Float < String, regardless of the order
    // the keys were inserted.
    let (f, log) = recorder(Arity::Exact(2));
    let m = map_of(vec![
        (MapKey::from("z"), Value::Null),
        (MapKey::from(1i64), Value::Null),
        (MapKey::from(true), Value::Null),
    ]);
    each(m, f).unwrap();
    assert_eq!(
        firsts(&log),
        vec![Value::Bool(true), Value::Int(1), Value::from("z")]
    );
}

#[test]
fn map_is_returned_unchanged() {
    let (f, _) = recorder(Arity::Exact(2));
    let input = map_of(vec![
        (MapKey::from("a"), Value::Int(1)),
        (MapKey::from("b"), Value::Int(2)),
    ]);
    let result = each(input.clone(), f).unwrap();
    assert_eq!(result, input);
}

#[test]
fn empty_map_never_calls_the_function() {
    let result = each(map_of(vec![]), boom()).unwrap();
    assert_eq!(result, map_of(vec![]));
}

#[test]
fn map_requires_a_binary_callback() {
    // The Map case invokes the callback with two arguments, so a unary callback
    // is an arity error.
    let (f, _) = recorder(Arity::Exact(1));
    let m = map_of(vec![(MapKey::from("a"), Value::Int(1))]);
    let err = each(m, f).unwrap_err();
    assert!(
        err.message().contains("called with 2"),
        "got: {}",
        err.message()
    );
}

// ============================================================
// Error propagation
// ============================================================

#[test]
fn error_from_the_callback_propagates_and_stops_iteration() {
    // A callback that records, then raises on the element `2`. Iteration must halt
    // there: `3` is never visited.
    let log: CallLog = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let f = native("fail_on_two", Arity::Exact(1), move |_, args: &mut [Value]| {
        sink.lock().unwrap().push(args.to_vec());
        if args[0] == Value::Int(2) {
            Err(FrostError::from_static("stop"))
        } else {
            Ok(Value::Null)
        }
    });
    let err = each(arr(vec![Value::Int(1), Value::Int(2), Value::Int(3)]), f).unwrap_err();
    assert_eq!(err.message(), "stop");
    assert_eq!(firsts(&log), vec![Value::Int(1), Value::Int(2)]);
}

// ============================================================
// Argument validation
// ============================================================

#[test]
fn first_argument_must_be_structured() {
    let (f, _) = recorder(Arity::Exact(1));
    let err = each(Value::Int(5), f).unwrap_err();
    assert_eq!(
        err.message(),
        "Function each requires Structured as argument 1, got Int"
    );
}

#[test]
fn second_argument_must_be_a_function() {
    let err = each(arr(vec![Value::Int(1)]), Value::Int(2)).unwrap_err();
    assert_eq!(
        err.message(),
        "Function each requires Function as argument 2, got Int"
    );
}

#[test]
fn each_enforces_its_own_arity() {
    let (f, _) = recorder(Arity::Exact(1));
    // Zero args.
    assert!(run_main(vec![], vec![LoadGlobal(global_slot("each")), Call(0)]).is_err());
    // One arg.
    assert!(
        run_main(
            vec![("s", arr(vec![]))],
            vec![LoadGlobal(global_slot("each")), LoadLocal(0), Call(1)],
        )
        .is_err()
    );
    // Three args.
    assert!(
        run_main(
            vec![("s", arr(vec![])), ("f", f), ("x", Value::Int(0))],
            vec![
                LoadGlobal(global_slot("each")),
                LoadLocal(0),
                LoadLocal(1),
                LoadLocal(2),
                Call(3),
            ],
        )
        .is_err()
    );
}
