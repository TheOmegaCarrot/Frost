//! Tests for the `select` global: the lowered form of `filter foo with f`.
//!
//! `select(structure, f)` keeps the parts of `structure` for which `f` is truthy:
//!
//!   * Array: keeps each `elem` where `f(elem)` is truthy, in order.
//!   * Map: keeps each entry where `f(key, value)` is truthy (a binary predicate).
//!
//! "Truthy" is the Frost rule: only `null` and `false` are falsy, so a predicate
//! that returns `0` or `""` keeps the element. Kept elements/entries pass through
//! unchanged; the predicate's return value is used only for the keep/drop decision.
//!
//! Predicates are computing natives for the value assertions, plus a `record`
//! native (keeps everything, logging each call) to pin down what `f` was invoked
//! with and in what order.

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

/// The ordered record of every predicate invocation: one entry per call, each the
/// argument list that call received.
type CallLog = Arc<Mutex<Vec<Vec<Value>>>>;

/// A predicate that keeps everything (returns `true`) while logging each call's
/// arguments, at the given arity. It isolates *what `f` saw*, and in what order,
/// from the keep/drop decision.
fn keep_all_recorder(arity: Arity) -> (Value, CallLog) {
    let log: CallLog = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let f = native("record", arity, move |_, args: &mut [Value]| {
        sink.lock().unwrap().push(args.to_vec());
        Ok(Value::Bool(true))
    });
    (f, log)
}

/// The calls recorded so far, as owned rows.
fn calls(log: &CallLog) -> Vec<Vec<Value>> {
    log.lock().unwrap().clone()
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
        key_constants: Vec::new(),
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

/// `select(structure, f)` through the full `LoadGlobal` + `Call` path.
fn select(structure: Value, f: Value) -> Result<Value, FrostError> {
    run_main(
        vec![("s", structure), ("f", f)],
        vec![
            LoadGlobal(global_slot("select")),
            LoadLocal(0),
            LoadLocal(1),
            Call(2),
        ],
    )
}

/// `fn x -> x % 2 == 0` as a native. `Exact(1)` doubles as proof of a single argument.
fn is_even() -> Value {
    native("is_even", Arity::Exact(1), |_, args| {
        Ok(Value::Bool(
            args[0].as_int().expect("is_even wants an Int") % 2 == 0,
        ))
    })
}

/// `fn x -> x` as a native: returns its argument verbatim, so `select` decides by
/// the argument's own truthiness.
fn identity() -> Value {
    native("identity", Arity::Exact(1), |_, args| Ok(args[0].take()))
}

// ============================================================
// Array
// ============================================================

#[test]
fn array_keeps_elements_where_predicate_is_truthy_in_order() {
    let result = select(
        arr(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]),
        is_even(),
    );
    assert_eq!(result.unwrap(), arr(vec![Value::Int(2), Value::Int(4)]));
}

#[test]
fn array_decides_by_truthiness_not_just_bool() {
    // The predicate returns each element as-is. Only null and false are falsy, so
    // 0 and "" are kept; null and false are dropped.
    let result = select(
        arr(vec![
            Value::Int(0),
            Value::Null,
            Value::Bool(false),
            Value::Int(1),
            Value::from(""),
        ]),
        identity(),
    );
    assert_eq!(
        result.unwrap(),
        arr(vec![Value::Int(0), Value::Int(1), Value::from("")])
    );
}

#[test]
fn array_keeps_nothing_when_predicate_is_always_falsy() {
    let never = native("never", Arity::Exact(1), |_, _| Ok(Value::Bool(false)));
    let result = select(arr(vec![Value::Int(1), Value::Int(2)]), never);
    assert_eq!(result.unwrap(), arr(vec![]));
}

#[test]
fn array_keeps_the_original_elements_unchanged() {
    // The kept elements are the inputs themselves, not the predicate's return value.
    let truthy = native("truthy", Arity::Exact(1), |_, _| Ok(Value::from("kept")));
    let result = select(arr(vec![Value::Int(7), Value::Int(8)]), truthy);
    assert_eq!(result.unwrap(), arr(vec![Value::Int(7), Value::Int(8)]));
}

#[test]
fn empty_array_yields_empty_array_without_calling_the_predicate() {
    // boom raises if called, so a clean run proves zero invocations.
    let result = select(arr(vec![]), boom());
    assert_eq!(result.unwrap(), arr(vec![]));
}

#[test]
fn array_propagates_an_error_from_the_predicate() {
    let err = select(arr(vec![Value::Int(1)]), boom()).unwrap_err();
    assert_eq!(err.message(), "boom!");
}

// ============================================================
// Map
// ============================================================

#[test]
fn map_keeps_entries_where_predicate_is_truthy() {
    // `fn (k, v) -> v > 1`: a binary predicate that inspects the value.
    let value_over_one = native("value_over_one", Arity::Exact(2), |_, args| {
        Ok(Value::Bool(
            args[1].as_int().expect("value_over_one wants an Int") > 1,
        ))
    });
    let input = map_of(vec![
        (MapKey::from("a"), Value::Int(1)),
        (MapKey::from("b"), Value::Int(2)),
        (MapKey::from("c"), Value::Int(3)),
    ]);
    let result = select(input, value_over_one).unwrap();
    assert_eq!(
        result,
        map_of(vec![
            (MapKey::from("b"), Value::Int(2)),
            (MapKey::from("c"), Value::Int(3)),
        ])
    );
}

#[test]
fn map_predicate_can_decide_by_the_key() {
    let key_is_keep = native("key_is_keep", Arity::Exact(2), |_, args| {
        Ok(Value::Bool(args[0].as_str() == Some("keep")))
    });
    let input = map_of(vec![
        (MapKey::from("keep"), Value::Int(1)),
        (MapKey::from("drop"), Value::Int(2)),
    ]);
    let result = select(input, key_is_keep).unwrap();
    assert_eq!(result, map_of(vec![(MapKey::from("keep"), Value::Int(1))]));
}

#[test]
fn map_decides_by_truthiness_not_just_bool() {
    // The predicate returns the value as-is: 0 is kept, null is dropped.
    let by_value = native("by_value", Arity::Exact(2), |_, args| Ok(args[1].take()));
    let input = map_of(vec![
        (MapKey::from("zero"), Value::Int(0)),
        (MapKey::from("nil"), Value::Null),
        (MapKey::from("five"), Value::Int(5)),
    ]);
    let result = select(input, by_value).unwrap();
    assert_eq!(
        result,
        map_of(vec![
            (MapKey::from("five"), Value::Int(5)),
            (MapKey::from("zero"), Value::Int(0)),
        ])
    );
}

#[test]
fn map_invokes_predicate_with_each_pair_in_mapkey_order() {
    let (f, log) = keep_all_recorder(Arity::Exact(2));
    // Int keys inserted out of order must be visited numerically ascending.
    let input = map_of(vec![
        (MapKey::from(3i64), Value::from("c")),
        (MapKey::from(1i64), Value::from("a")),
        (MapKey::from(2i64), Value::from("b")),
    ]);
    let result = select(input.clone(), f).unwrap();
    assert_eq!(
        calls(&log),
        vec![
            vec![Value::Int(1), Value::from("a")],
            vec![Value::Int(2), Value::from("b")],
            vec![Value::Int(3), Value::from("c")],
        ]
    );
    // Keeping everything returns the map unchanged.
    assert_eq!(result, input);
}

#[test]
fn map_requires_a_binary_predicate() {
    // The Map case invokes the predicate with two arguments, so a unary predicate
    // is an arity error.
    let unary = native("unary", Arity::Exact(1), |_, _| Ok(Value::Bool(true)));
    let input = map_of(vec![(MapKey::from("a"), Value::Int(1))]);
    let err = select(input, unary).unwrap_err();
    assert!(
        err.message().contains("called with 2"),
        "got: {}",
        err.message()
    );
}

#[test]
fn empty_map_yields_empty_map_without_calling_the_predicate() {
    let result = select(map_of(vec![]), boom());
    assert_eq!(result.unwrap(), map_of(vec![]));
}

#[test]
fn map_propagates_an_error_from_the_predicate() {
    let input = map_of(vec![(MapKey::from("a"), Value::Int(1))]);
    let err = select(input, boom()).unwrap_err();
    assert_eq!(err.message(), "boom!");
}

// ============================================================
// Argument validation
// ============================================================

#[test]
fn first_argument_must_be_structured() {
    let err = select(Value::Int(5), is_even()).unwrap_err();
    assert_eq!(
        err.message(),
        "Function select requires Structured as argument 1, got Int"
    );
}

#[test]
fn second_argument_must_be_a_function() {
    let err = select(arr(vec![Value::Int(1)]), Value::Int(2)).unwrap_err();
    assert_eq!(
        err.message(),
        "Function select requires Function as argument 2, got Int"
    );
}

#[test]
fn select_enforces_its_own_arity() {
    // Zero args.
    assert!(run_main(vec![], vec![LoadGlobal(global_slot("select")), Call(0)]).is_err());
    // One arg.
    assert!(
        run_main(
            vec![("s", arr(vec![]))],
            vec![LoadGlobal(global_slot("select")), LoadLocal(0), Call(1)],
        )
        .is_err()
    );
    // Three args.
    assert!(
        run_main(
            vec![("s", arr(vec![])), ("f", is_even()), ("x", Value::Int(0))],
            vec![
                LoadGlobal(global_slot("select")),
                LoadLocal(0),
                LoadLocal(1),
                LoadLocal(2),
                Call(3),
            ],
        )
        .is_err()
    );
}
