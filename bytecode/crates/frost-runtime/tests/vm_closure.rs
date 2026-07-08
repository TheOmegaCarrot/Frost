//! Tests for the closing / entry-point API: `CompiledFunction::close`,
//! `into_closure`, `capture_names`, the `MissingCaptures` error, and
//! `Vm::run_with_args`.
//!
//! Unlike the opcode test files, these build top-level closures by hand with a
//! real prelude (the leading fn-value `Pop`, plus `DefLocal`s for params), since
//! the point is the closing/calling contract itself.

use std::collections::BTreeMap;
use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostArray, MissingCaptures, NameEntry, ProgramResult,
    Value, Vm,
};

use Bytecode::{DefLocal, LoadLocal, Pop, PushInt, Subtract};

/// Build a `CompiledFunction` with an explicit prelude (no auto-`Pop`).
/// `names` are the slot names; the first `num_captures` are captures.
fn compiled(
    code: Vec<Bytecode>,
    arity: Arity,
    num_captures: usize,
    names: &[&str],
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: "<closure-test>".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names
            .iter()
            .map(|n| NameEntry {
                name: n.to_string(),
                exported: false,
            })
            .collect(),
        num_captures,
        arity,
    })
}

fn capmap(pairs: Vec<(&str, Value)>) -> BTreeMap<String, Value> {
    pairs.into_iter().map(|(n, v)| (n.to_string(), v)).collect()
}

fn run(vm: Vm) -> ProgramResult {
    vm.run().unwrap()
}

// ============================================================
// close: capture binding
// ============================================================

#[test]
fn close_binds_capture_value() {
    // fn -> <captured x>
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["x"]);
    let closure = f.assert_trusted().close(capmap(vec![("x", Value::Int(42))])).unwrap();
    assert_eq!(
        run(Vm::factory().build(closure).unwrap()).tail(),
        &Value::Int(42)
    );
}

#[test]
fn close_ignores_extra_map_entries() {
    // The map may be a superset; unused entries are ignored.
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["x"]);
    let closure = f
        .assert_trusted()
        .close(capmap(vec![
            ("x", Value::Int(1)),
            ("unused", Value::Int(2)),
            ("also_unused", Value::Int(3)),
        ]))
        .unwrap();
    assert_eq!(
        run(Vm::factory().build(closure).unwrap()).tail(),
        &Value::Int(1)
    );
}

#[test]
fn close_missing_capture_is_error() {
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["x"]);
    let err = f.assert_trusted().close(BTreeMap::new()).unwrap_err();
    assert_eq!(
        err,
        MissingCaptures {
            names: vec!["x".to_string()]
        }
    );
}

#[test]
fn close_reports_every_missing_capture_in_slot_order() {
    // Three captures with names deliberately NOT in alphabetical order, only the
    // middle one provided: the error must list the absent names in SLOT order
    // (["z", "y"]), which differs from sorted order (["y", "z"]) -- so a stray
    // sort of the missing list would be caught.
    let f = compiled(
        vec![Pop, LoadLocal(0)],
        Arity::Exact(0),
        3,
        &["z", "x", "y"],
    );
    let err = f.assert_trusted().close(capmap(vec![("x", Value::Int(1))])).unwrap_err();
    assert_eq!(
        err,
        MissingCaptures {
            names: vec!["z".to_string(), "y".to_string()],
        }
    );
}

#[test]
fn close_seats_each_capture_in_its_own_slot() {
    // Two captures whose names are deliberately NOT in alphabetical order, read
    // back with a non-commutative op: slot 0 is `b` (100), slot 1 is `a` (1), so
    // the body computes 100 - 1 = 99. Seating in BTreeMap (alphabetical) order
    // instead of name-table order would swap them and give 1 - 100 = -99.
    let f = compiled(
        vec![Pop, LoadLocal(0), LoadLocal(1), Subtract],
        Arity::Exact(0),
        2,
        &["b", "a"],
    );
    let closure = f
        .assert_trusted()
        .close(capmap(vec![("b", Value::Int(100)), ("a", Value::Int(1))]))
        .unwrap();
    assert_eq!(
        run(Vm::factory().build(closure).unwrap()).tail(),
        &Value::Int(99)
    );
}

// ============================================================
// close: the internal `imported` capture
// ============================================================

#[test]
fn close_injects_imported_capture() {
    // `imported` is runtime-supplied, so an empty map still closes; it reads false.
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["imported"]);
    let closure = f.assert_trusted().close(BTreeMap::new()).unwrap();
    assert_eq!(
        run(Vm::factory().build(closure).unwrap()).tail(),
        &Value::Bool(false)
    );
}

#[test]
fn close_does_not_let_host_override_imported() {
    // A host `imported` entry is ignored; the injected false wins.
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["imported"]);
    let closure = f
        .assert_trusted()
        .close(capmap(vec![("imported", Value::Bool(true))]))
        .unwrap();
    assert_eq!(
        run(Vm::factory().build(closure).unwrap()).tail(),
        &Value::Bool(false)
    );
}

#[test]
fn close_injects_imported_at_its_name_table_slot() {
    // `imported` is at slot 1, after host capture `x`. The injected false must
    // land at slot 1 (its name-table position), not a fixed slot 0.
    let read_imported = compiled(
        vec![Pop, LoadLocal(1)],
        Arity::Exact(0),
        2,
        &["x", "imported"],
    );
    let c1 = read_imported
        .assert_trusted()
        .close(capmap(vec![("x", Value::Int(7))]))
        .unwrap();
    assert_eq!(
        run(Vm::factory().build(c1).unwrap()).tail(),
        &Value::Bool(false)
    );

    // ...and the host capture `x` is seated at slot 0, undisturbed by the injection.
    let read_x = compiled(
        vec![Pop, LoadLocal(0)],
        Arity::Exact(0),
        2,
        &["x", "imported"],
    );
    let c2 = read_x.assert_trusted().close(capmap(vec![("x", Value::Int(7))])).unwrap();
    assert_eq!(run(Vm::factory().build(c2).unwrap()).tail(), &Value::Int(7));
}

// ============================================================
// into_closure
// ============================================================

#[test]
fn into_closure_for_no_captures() {
    let f = compiled(vec![Pop, PushInt(7)], Arity::Exact(0), 0, &[]);
    let closure = f.assert_trusted().into_closure().unwrap();
    assert_eq!(
        run(Vm::factory().build(closure).unwrap()).tail(),
        &Value::Int(7)
    );
}

#[test]
fn into_closure_errors_when_host_capture_needed() {
    // into_closure is `close` with an empty map, so a required host capture is missing.
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["x"]);
    let err = f.assert_trusted().into_closure().unwrap_err();
    assert_eq!(
        err,
        MissingCaptures {
            names: vec!["x".to_string()]
        }
    );
}

#[test]
fn into_closure_ok_for_internal_only_captures() {
    // Captures only `imported` (runtime-supplied), so into_closure still succeeds.
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["imported"]);
    let closure = f.assert_trusted().into_closure().unwrap();
    assert_eq!(
        run(Vm::factory().build(closure).unwrap()).tail(),
        &Value::Bool(false)
    );
}

// ============================================================
// capture_names
// ============================================================

#[test]
fn capture_names_lists_only_captures() {
    // a, b are captures; c is an ordinary local and must not be listed.
    let f = compiled(vec![Pop], Arity::Exact(0), 2, &["a", "b", "c"]);
    assert_eq!(f.capture_names().collect::<Vec<_>>(), ["a", "b"]);
}

#[test]
fn capture_names_empty_when_none() {
    let f = compiled(vec![Pop], Arity::Exact(0), 0, &[]);
    assert_eq!(f.capture_names().count(), 0);
}

// ============================================================
// run_with_args
// ============================================================

#[test]
fn run_with_args_passes_arguments() {
    // fn a, b -> a - b ; prelude seats b (slot 1) then a (slot 0), then pops the
    // fn value. A non-commutative op makes the seating order load-bearing:
    // 10 - 20 = -10, whereas a swapped seating would give +10.
    let f = compiled(
        vec![
            DefLocal(1),
            DefLocal(0),
            Pop,
            LoadLocal(0),
            LoadLocal(1),
            Subtract,
        ],
        Arity::Exact(2),
        0,
        &["a", "b"],
    );
    let result = Vm::factory()
        .build(f.assert_trusted().into_closure().unwrap())
        .unwrap()
        .run_with_args([Value::Int(10), Value::Int(20)])
        .unwrap();
    assert_eq!(result.tail(), &Value::Int(-10));
}

#[test]
fn run_with_args_alongside_captures() {
    // Captured `k` (slot 0) plus param `a` (slot 1): returns k - a = 95.
    // Exercises the [captures..., locals...] slot boundary under run_with_args;
    // the non-commutative op pins which value lands on each side of it.
    let f = compiled(
        vec![DefLocal(1), Pop, LoadLocal(0), LoadLocal(1), Subtract],
        Arity::Exact(1),
        1,
        &["k", "a"],
    );
    let closure = f.assert_trusted().close(capmap(vec![("k", Value::Int(100))])).unwrap();
    let result = Vm::factory()
        .build(closure)
        .unwrap()
        .run_with_args([Value::Int(5)])
        .unwrap();
    assert_eq!(result.tail(), &Value::Int(95));
}

#[test]
fn run_with_args_variadic_collects_rest() {
    // fn ...args -> args : the call collapses the args into one Array.
    let f = compiled(
        vec![DefLocal(0), Pop, LoadLocal(0)],
        Arity::AtLeast(0),
        0,
        &["args"],
    );
    let result = Vm::factory()
        .build(f.assert_trusted().into_closure().unwrap())
        .unwrap()
        .run_with_args([Value::Int(1), Value::Int(2), Value::Int(3)])
        .unwrap();
    assert_eq!(
        result.tail(),
        &Value::Array(FrostArray::from(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
        ]))
    );
}

#[test]
fn run_with_args_too_few_is_arity_error() {
    let f = compiled(
        vec![
            DefLocal(1),
            DefLocal(0),
            Pop,
            LoadLocal(0),
            LoadLocal(1),
            Subtract,
        ],
        Arity::Exact(2),
        0,
        &["a", "b"],
    );
    let err = Vm::factory()
        .build(f.assert_trusted().into_closure().unwrap())
        .unwrap()
        .run_with_args([Value::Int(10)])
        .unwrap_err()
        .into_error();
    assert!(
        err.message.contains("expects 2 arguments"),
        "got: {}",
        err.message
    );
    assert!(
        err.message.contains("called with 1"),
        "got: {}",
        err.message
    );
}

#[test]
fn run_with_args_too_many_is_arity_error() {
    // Exact(1) called with 2 args: a check that regressed from `argc == n` to
    // `argc >= n` would wrongly accept this.
    let f = compiled(
        vec![DefLocal(0), Pop, LoadLocal(0)],
        Arity::Exact(1),
        0,
        &["a"],
    );
    let err = Vm::factory()
        .build(f.assert_trusted().into_closure().unwrap())
        .unwrap()
        .run_with_args([Value::Int(1), Value::Int(2)])
        .unwrap_err()
        .into_error();
    assert!(
        err.message.contains("expects 1 arguments"),
        "got: {}",
        err.message
    );
    assert!(
        err.message.contains("called with 2"),
        "got: {}",
        err.message
    );
}

#[test]
fn run_with_args_variadic_too_few_is_arity_error() {
    // AtLeast(2) called with 1 arg drives the "expects at least" error branch.
    let f = compiled(
        vec![DefLocal(2), DefLocal(1), DefLocal(0), Pop, LoadLocal(0)],
        Arity::AtLeast(2),
        0,
        &["a", "b", "rest"],
    );
    let err = Vm::factory()
        .build(f.assert_trusted().into_closure().unwrap())
        .unwrap()
        .run_with_args([Value::Int(1)])
        .unwrap_err()
        .into_error();
    assert!(err.message.contains("at least 2"), "got: {}", err.message);
    assert!(
        err.message.contains("called with 1"),
        "got: {}",
        err.message
    );
}

#[test]
fn run_no_args_on_parameterized_is_arity_error() {
    // Bare `run()` on a closure that wants arguments is an arity error.
    let f = compiled(
        vec![DefLocal(0), Pop, LoadLocal(0)],
        Arity::Exact(1),
        0,
        &["a"],
    );
    let err = Vm::factory()
        .build(f.assert_trusted().into_closure().unwrap())
        .unwrap()
        .run()
        .unwrap_err()
        .into_error();
    assert!(
        err.message.contains("expects 1 arguments"),
        "got: {}",
        err.message
    );
    assert!(
        err.message.contains("called with 0"),
        "got: {}",
        err.message
    );
}
