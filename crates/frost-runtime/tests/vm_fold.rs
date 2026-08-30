//! Tests for the `fold` global: the lowered form of `reduce foo with f`.
//!
//! `fold(structure, f)` / `fold(structure, f, init)` reduces `structure` to a
//! single value by threading an accumulator through `f`:
//!
//!   * Array: `f(acc, elem)`, left to right. With `init`, that is the starting
//!     accumulator. Without, the first element seeds it and the fold runs over
//!     the rest. An empty Array with no `init` folds to `null`.
//!   * Map: `f(acc, key, value)` over entries in `MapKey` order. A Map fold
//!     *requires* `init` (there is no natural first-element seed for a ternary
//!     step); omitting it is an error.
//!
//! The array probe is subtraction: non-commutative and non-associative, so a
//! single result value pins down both the accumulator-first argument order and
//! the left-to-right direction. `boom` (raises if called) proves the paths that
//! must not invoke `f`.

use std::sync::Arc;

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

/// `fn (acc, x) -> acc - x` over Ints. Non-commutative, so the result reveals both
/// the argument order (accumulator first) and the fold direction (left to right).
fn subtract() -> Value {
    native("subtract", Arity::Exact(2), |_, args| {
        Ok(Value::Int(
            args[0].as_int().expect("subtract wants Ints")
                - args[1].as_int().expect("subtract wants Ints"),
        ))
    })
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
        .map_err(frost_runtime::RunError::into_error)
        .map(|r| r.tail().clone())
}

/// `fold(structure, f)` or `fold(structure, f, init)` through the full
/// `LoadGlobal` + `Call` path, depending on whether `init` is supplied.
fn fold(structure: Value, f: Value, init: Option<Value>) -> Result<Value, FrostError> {
    match init {
        Some(init) => run_main(
            vec![("s", structure), ("f", f), ("i", init)],
            vec![
                LoadGlobal(global_slot("fold")),
                LoadLocal(0),
                LoadLocal(1),
                LoadLocal(2),
                Call(3),
            ],
        ),
        None => run_main(
            vec![("s", structure), ("f", f)],
            vec![
                LoadGlobal(global_slot("fold")),
                LoadLocal(0),
                LoadLocal(1),
                Call(2),
            ],
        ),
    }
}

// ============================================================
// Array
// ============================================================

#[test]
fn array_with_init_folds_left_accumulator_first() {
    // ((10 - 1) - 2) - 3 == 4. Any other order or direction gives a different value.
    let result = fold(
        arr(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        subtract(),
        Some(Value::Int(10)),
    );
    assert_eq!(result.unwrap(), Value::Int(4));
}

#[test]
fn array_without_init_seeds_with_the_first_element() {
    // (1 - 2) - 3 == -4: the first element is the accumulator, the fold runs over the rest.
    let result = fold(
        arr(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        subtract(),
        None,
    );
    assert_eq!(result.unwrap(), Value::Int(-4));
}

#[test]
fn array_single_element_no_init_returns_it_without_calling_f() {
    // The lone element seeds the accumulator; the fold over the empty rest never calls f.
    let result = fold(arr(vec![Value::Int(42)]), boom(), None);
    assert_eq!(result.unwrap(), Value::Int(42));
}

#[test]
fn array_single_element_with_init_calls_f_once() {
    // 20 - 5 == 15: init seeds, so f runs even on the single element.
    let result = fold(arr(vec![Value::Int(5)]), subtract(), Some(Value::Int(20)));
    assert_eq!(result.unwrap(), Value::Int(15));
}

#[test]
fn empty_array_no_init_returns_null() {
    let result = fold(arr(vec![]), boom(), None);
    assert_eq!(result.unwrap(), Value::Null);
}

#[test]
fn empty_array_with_init_returns_init_without_calling_f() {
    let result = fold(arr(vec![]), boom(), Some(Value::Int(7)));
    assert_eq!(result.unwrap(), Value::Int(7));
}

#[test]
fn array_propagates_an_error_from_f() {
    let err = fold(
        arr(vec![Value::Int(1), Value::Int(2)]),
        boom(),
        Some(Value::Int(0)),
    )
    .unwrap_err();
    assert_eq!(err.message(), "boom!");
}

// ============================================================
// Map
// ============================================================

#[test]
fn map_folds_over_entries_with_accumulator_key_value() {
    // sum of values: 0 + 1 + 2 + 3 == 6. Uses acc (arg 0) and value (arg 2),
    // proving f receives the (accumulator, key, value) triple.
    let sum_values = native("sum_values", Arity::Exact(3), |_, args| {
        Ok(Value::Int(
            args[0].as_int().expect("acc is an Int") + args[2].as_int().expect("value is an Int"),
        ))
    });
    let input = map_of(vec![
        (MapKey::from("a"), Value::Int(1)),
        (MapKey::from("b"), Value::Int(2)),
        (MapKey::from("c"), Value::Int(3)),
    ]);
    let result = fold(input, sum_values, Some(Value::Int(0)));
    assert_eq!(result.unwrap(), Value::Int(6));
}

#[test]
fn map_visits_entries_in_mapkey_order() {
    // Collect each key into an array accumulator. Int keys inserted out of order
    // must come out numerically ascending, proving both the key argument and the
    // MapKey iteration order.
    let collect_keys = native("collect_keys", Arity::Exact(3), |_, args| {
        let mut acc = args[0]
            .take()
            .try_into_array()
            .expect("acc is an Array")
            .into_vec();
        acc.push(args[1].take());
        Ok(Value::from(acc))
    });
    let input = map_of(vec![
        (MapKey::from(3i64), Value::Null),
        (MapKey::from(1i64), Value::Null),
        (MapKey::from(2i64), Value::Null),
    ]);
    let result = fold(input, collect_keys, Some(arr(vec![])));
    assert_eq!(
        result.unwrap(),
        arr(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn map_without_init_is_an_error() {
    // A ternary step has no natural first-element seed, so a Map fold requires init.
    // boom is never reached: the error is raised before any element is visited.
    let input = map_of(vec![(MapKey::from("a"), Value::Int(1))]);
    let err = fold(input, boom(), None).unwrap_err();
    assert_eq!(err.message(), "Fold over a Map requires an initializer");
}

#[test]
fn empty_map_with_init_returns_init_without_calling_f() {
    let result = fold(map_of(vec![]), boom(), Some(Value::Int(99)));
    assert_eq!(result.unwrap(), Value::Int(99));
}

#[test]
fn map_propagates_an_error_from_f() {
    let input = map_of(vec![(MapKey::from("a"), Value::Int(1))]);
    let err = fold(input, boom(), Some(Value::Int(0))).unwrap_err();
    assert_eq!(err.message(), "boom!");
}

// ============================================================
// Argument validation
// ============================================================

#[test]
fn first_argument_must_be_structured() {
    let err = fold(Value::Int(5), subtract(), Some(Value::Int(0))).unwrap_err();
    assert_eq!(
        err.message(),
        "Function fold requires Structured as argument 1, got Int"
    );
}

#[test]
fn second_argument_must_be_a_function() {
    let err = fold(arr(vec![Value::Int(1)]), Value::Int(2), Some(Value::Int(0))).unwrap_err();
    assert_eq!(
        err.message(),
        "Function fold requires Function as argument 2, got Int"
    );
}

#[test]
fn fold_enforces_its_own_arity() {
    // fold accepts 2 or 3 arguments.
    // Zero args.
    assert!(run_main(vec![], vec![LoadGlobal(global_slot("fold")), Call(0)]).is_err());
    // One arg.
    assert!(
        run_main(
            vec![("s", arr(vec![]))],
            vec![LoadGlobal(global_slot("fold")), LoadLocal(0), Call(1)],
        )
        .is_err()
    );
    // Four args.
    assert!(
        run_main(
            vec![
                ("s", arr(vec![])),
                ("f", subtract()),
                ("i", Value::Int(0)),
                ("x", Value::Int(1)),
            ],
            vec![
                LoadGlobal(global_slot("fold")),
                LoadLocal(0),
                LoadLocal(1),
                LoadLocal(2),
                LoadLocal(3),
                Call(4),
            ],
        )
        .is_err()
    );
}
