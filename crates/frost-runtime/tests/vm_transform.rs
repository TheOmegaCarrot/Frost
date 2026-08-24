//! Tests for the `transform` global: the lowered form of `map foo with f`.
//!
//! `transform(structure, f)` applies `f` to each element of an Array, or to each
//! `(key, value)` pair of a Map.
//!
//!   * Array: `f(elem)` element-wise, producing a new Array of the same length,
//!     in order. `f` may return any type.
//!   * Map: `f(key, value)` per entry, and `f` *must* return a Map; the returned
//!     maps are merged (in `MapKey` iteration order, so a later entry's keys win
//!     on collision). This lets `f` remap keys, expand one entry into many, or
//!     drop an entry by returning `{}`.
//!
//! Callbacks are natives here: computing ones for the value assertions, and a
//! `record` native (appending each call's arguments to a shared log) for the
//! ones that pin down what `f` was invoked with.

use std::sync::{Arc, Mutex};

mod common;

use common::{Pop, global_slot};
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostArray, FrostError, FrostMap,
    FrostResult, MapKey, NameEntry, NativeCtx, NativeFunction, Value, Vm,
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

/// A Map-case callback that appends its `(key, value)` arguments to a shared log
/// and returns `{}`, so every entry is dropped and the run's result is an empty
/// Map: it isolates *what `f` saw* from whatever the merge produces.
fn recorder() -> (Value, CallLog) {
    let log: CallLog = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let f = native("record", Arity::Exact(2), move |_, args: &mut [Value]| {
        sink.lock().unwrap().push(args.to_vec());
        Ok(Value::Map(FrostMap::empty()))
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

/// `transform(structure, f)` through the full `LoadGlobal` + `Call` path.
fn transform(structure: Value, f: Value) -> Result<Value, FrostError> {
    run_main(
        vec![("s", structure), ("f", f)],
        vec![
            LoadGlobal(global_slot("transform")),
            LoadLocal(0),
            LoadLocal(1),
            Call(2),
        ],
    )
}

/// `fn x -> x * 10` as a native. `Exact(1)` doubles as proof of a single argument.
fn times_ten() -> Value {
    native("times_ten", Arity::Exact(1), |_, args| {
        Ok(Value::Int(
            args[0].as_int().expect("times_ten wants an Int") * 10,
        ))
    })
}

// ============================================================
// Array
// ============================================================

#[test]
fn array_applies_f_to_each_element_in_order() {
    let result = transform(
        arr(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        times_ten(),
    );
    assert_eq!(
        result.unwrap(),
        arr(vec![Value::Int(10), Value::Int(20), Value::Int(30)])
    );
}

#[test]
fn array_result_has_the_same_length() {
    let result = transform(arr(vec![Value::Int(5), Value::Int(6)]), times_ten()).unwrap();
    assert_eq!(result.as_array().unwrap().len(), 2);
}

#[test]
fn array_callback_may_return_any_type() {
    // The Array case places no constraint on f's return type (unlike the Map case).
    let to_str = native("to_str", Arity::Exact(1), |_, _| Ok(Value::from("x")));
    let result = transform(arr(vec![Value::Int(1), Value::Int(2)]), to_str);
    assert_eq!(
        result.unwrap(),
        arr(vec![Value::from("x"), Value::from("x")])
    );
}

#[test]
fn empty_array_yields_empty_array_without_calling_f() {
    // boom raises if called, so a clean run proves zero invocations.
    let result = transform(arr(vec![]), boom());
    assert_eq!(result.unwrap(), arr(vec![]));
}

#[test]
fn array_propagates_an_error_from_f() {
    let err = transform(arr(vec![Value::Int(1)]), boom()).unwrap_err();
    assert_eq!(err.message(), "boom!");
}

// ============================================================
// Map
// ============================================================

#[test]
fn map_applies_f_to_each_key_value_pair() {
    // `fn (k, v) -> {[k]: v * 10}`: uses both the key (as the result key) and the
    // value, proving each entry reaches f as a (key, value) pair.
    let scale = native("scale", Arity::Exact(2), |_, args| {
        let key = MapKey::try_from(args[0].take())?;
        let value = Value::Int(args[1].as_int().expect("scale wants an Int") * 10);
        Ok(Value::map([(key, value)]))
    });
    let input = map_of(vec![
        (MapKey::from("a"), Value::Int(1)),
        (MapKey::from("b"), Value::Int(2)),
    ]);
    let result = transform(input, scale).unwrap();
    assert_eq!(
        result,
        map_of(vec![
            (MapKey::from("a"), Value::Int(10)),
            (MapKey::from("b"), Value::Int(20)),
        ])
    );
}

#[test]
fn map_invokes_f_with_each_pair_in_mapkey_order() {
    let (f, log) = recorder();
    // Int keys inserted out of order must be visited numerically ascending.
    let input = map_of(vec![
        (MapKey::from(3i64), Value::from("c")),
        (MapKey::from(1i64), Value::from("a")),
        (MapKey::from(2i64), Value::from("b")),
    ]);
    transform(input, f).unwrap();
    assert_eq!(
        calls(&log),
        vec![
            vec![Value::Int(1), Value::from("a")],
            vec![Value::Int(2), Value::from("b")],
            vec![Value::Int(3), Value::from("c")],
        ]
    );
}

#[test]
fn map_callback_can_remap_keys() {
    let rename = native("rename", Arity::Exact(2), |_, args| {
        Ok(Value::map([(MapKey::from("renamed"), args[1].take())]))
    });
    let input = map_of(vec![(MapKey::from("original"), Value::Int(7))]);
    let result = transform(input, rename).unwrap();
    assert_eq!(
        result,
        map_of(vec![(MapKey::from("renamed"), Value::Int(7))])
    );
}

#[test]
fn map_callback_can_expand_one_entry_into_many() {
    // One input entry -> a two-entry map, so the result is larger than the input.
    let split = native("split", Arity::Exact(2), |_, args| {
        let n = args[1].as_int().expect("split wants an Int");
        Ok(Value::map([
            (MapKey::from("lo"), Value::Int(n)),
            (MapKey::from("hi"), Value::Int(n + 1)),
        ]))
    });
    let input = map_of(vec![(MapKey::from("x"), Value::Int(100))]);
    let result = transform(input, split).unwrap();
    assert_eq!(
        result,
        map_of(vec![
            (MapKey::from("hi"), Value::Int(101)),
            (MapKey::from("lo"), Value::Int(100)),
        ])
    );
}

#[test]
fn map_callback_can_drop_an_entry_by_returning_empty() {
    // Every entry maps to `{}`, so the result is empty.
    let drop = native("drop", Arity::Exact(2), |_, _| {
        Ok(Value::Map(FrostMap::empty()))
    });
    let input = map_of(vec![
        (MapKey::from("a"), Value::Int(1)),
        (MapKey::from("b"), Value::Int(2)),
    ]);
    let result = transform(input, drop).unwrap();
    assert_eq!(result, map_of(vec![]));
}

#[test]
fn map_later_entry_wins_on_key_collision() {
    // Both entries map to the key "dup"; `b` (visited after `a` in MapKey order)
    // is merged last, so its value wins.
    let to_dup = native("to_dup", Arity::Exact(2), |_, args| {
        Ok(Value::map([(MapKey::from("dup"), args[1].take())]))
    });
    let input = map_of(vec![
        (MapKey::from("a"), Value::from("first")),
        (MapKey::from("b"), Value::from("second")),
    ]);
    let result = transform(input, to_dup).unwrap();
    assert_eq!(
        result,
        map_of(vec![(MapKey::from("dup"), Value::from("second"))])
    );
}

#[test]
fn map_callback_must_return_a_map() {
    let bad = native("bad", Arity::Exact(2), |_, _| Ok(Value::Int(0)));
    let input = map_of(vec![(MapKey::from("a"), Value::Int(1))]);
    let err = transform(input, bad).unwrap_err();
    assert_eq!(
        err.message(),
        "When transforming a Map, the function must return a Map, got Int"
    );
}

#[test]
fn empty_map_yields_empty_map_without_calling_f() {
    let result = transform(map_of(vec![]), boom());
    assert_eq!(result.unwrap(), map_of(vec![]));
}

#[test]
fn map_propagates_an_error_from_f() {
    let input = map_of(vec![(MapKey::from("a"), Value::Int(1))]);
    let err = transform(input, boom()).unwrap_err();
    assert_eq!(err.message(), "boom!");
}

// ============================================================
// Argument validation
// ============================================================

#[test]
fn first_argument_must_be_structured() {
    let err = transform(Value::Int(5), times_ten()).unwrap_err();
    assert_eq!(
        err.message(),
        "Function transform requires Structured as argument 1, got Int"
    );
}

#[test]
fn second_argument_must_be_a_function() {
    let err = transform(arr(vec![Value::Int(1)]), Value::Int(2)).unwrap_err();
    assert_eq!(
        err.message(),
        "Function transform requires Function as argument 2, got Int"
    );
}

#[test]
fn transform_enforces_its_own_arity() {
    // Zero args.
    assert!(run_main(vec![], vec![LoadGlobal(global_slot("transform")), Call(0)]).is_err());
    // One arg.
    assert!(
        run_main(
            vec![("s", arr(vec![]))],
            vec![LoadGlobal(global_slot("transform")), LoadLocal(0), Call(1)],
        )
        .is_err()
    );
    // Three args.
    assert!(
        run_main(
            vec![("s", arr(vec![])), ("f", times_ten()), ("x", Value::Int(0))],
            vec![
                LoadGlobal(global_slot("transform")),
                LoadLocal(0),
                LoadLocal(1),
                LoadLocal(2),
                Call(3),
            ],
        )
        .is_err()
    );
}
