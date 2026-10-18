//! Tests for the closing / entry-point API: `WellFormedProgram::close` and
//! `close_with`, `into_closure`, `capture_names`, the `MissingCaptures` error,
//! and `Vm::run_with_args`.
//!
//! Unlike the opcode test files, these build top-level closures by hand with a
//! real prelude (the leading fn-value `Pop`, plus `DefLocal`s for params), since
//! the point is the closing/calling contract itself.

use std::collections::BTreeMap;
use std::sync::Arc;

use frostlang::bytecode::{Bytecode, CompiledFunction, FormatVersion, NameEntry};
use frostlang::{
    Arity, FrostArray, FrostString, MissingCaptures, ProgramResult, Value, Vm, WellFormedProgram,
};

use crate::common;

use Bytecode::{DefLocal, LoadLocal, PushInt, Subtract};
use common::Pop;

/// Build a `CompiledFunction` with an explicit prelude (no auto-`Pop`).
/// `names` are the slot names; the first `num_captures` are captures.
fn compiled(
    code: Vec<Bytecode>,
    arity: Arity,
    num_captures: usize,
    names: &[&str],
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<closure-test>".to_string(),
        origin: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
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

fn run(vm: Vm) -> ProgramResult {
    vm.run().unwrap()
}

/// `fn -> <captured x>`: a program whose only capture, `x`, is its value.
fn returns_x() -> WellFormedProgram {
    compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["x"]).assert_well_formed()
}

/// The tail value of running `closure`.
fn tail_of(closure: Arc<frostlang::Closure>) -> Value {
    run(Vm::factory().build(closure)).tail().clone()
}

/// No captures at all, as `close` takes them.
fn no_captures() -> Vec<(&'static str, Value)> {
    Vec::new()
}

// ============================================================
// close: capture binding
// ============================================================

#[test]
fn close_binds_capture_value() {
    let closure = returns_x().close([("x", Value::Int(42))]).unwrap();
    assert_eq!(tail_of(closure), Value::Int(42));
}

#[test]
fn close_ignores_names_the_program_does_not_capture() {
    // The pairs may be a superset; unused ones are ignored.
    let closure = returns_x()
        .close([
            ("unused", Value::Int(2)),
            ("x", Value::Int(1)),
            ("also_unused", Value::Int(3)),
        ])
        .unwrap();
    assert_eq!(tail_of(closure), Value::Int(1));
}

#[test]
fn close_never_seats_a_local_named_like_a_pair() {
    // `local` is a slot name, but not a capture: a pair naming it is ignored,
    // and the capture it does not name is still missing.
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["x", "local"]);
    let err = f
        .assert_well_formed()
        .close([("local", Value::Int(1))])
        .unwrap_err();
    assert_eq!(err.names, ["x"], "only the capture is required");
}

#[test]
fn close_takes_the_last_value_of_a_repeated_name() {
    let closure = returns_x()
        .close([
            ("x", Value::Int(1)),
            ("x", Value::Int(2)),
            ("x", Value::Int(3)),
        ])
        .unwrap();
    assert_eq!(tail_of(closure), Value::Int(3), "the last pair wins");
}

#[test]
fn close_accepts_any_string_like_name() {
    let value = || Value::Int(7);
    let closures = [
        ("&str", returns_x().close([("x", value())])),
        ("String", returns_x().close([(String::from("x"), value())])),
        (
            "FrostString",
            returns_x().close([(FrostString::from("x"), value())]),
        ),
        (
            "Arc<str>",
            returns_x().close([(Arc::<str>::from("x"), value())]),
        ),
    ];
    for (how, closure) in closures {
        let closure = closure.unwrap_or_else(|missing| panic!("{how}: {missing}"));
        assert_eq!(tail_of(closure), value(), "{how}");
    }
}

#[test]
fn close_accepts_a_map_of_captures() {
    // A host's map of name to value is itself an iterator of pairs.
    let captures = BTreeMap::from([
        (String::from("x"), Value::Int(5)),
        (String::from("unused"), Value::Null),
    ]);
    let closure = returns_x().close(captures).unwrap();
    assert_eq!(tail_of(closure), Value::Int(5));
}

#[test]
fn close_missing_capture_is_error() {
    let err = returns_x().close(no_captures()).unwrap_err();
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
    // (["z", "y"]), which differs from sorted order (["y", "z"]), so a stray
    // sort of the missing list would be caught.
    let f = compiled(
        vec![Pop, LoadLocal(0)],
        Arity::Exact(0),
        3,
        &["z", "x", "y"],
    );
    let err = f
        .assert_well_formed()
        .close([("x", Value::Int(1))])
        .unwrap_err();
    assert_eq!(
        err,
        MissingCaptures {
            names: vec!["z".to_string(), "y".to_string()],
        }
    );
}

#[test]
fn close_seats_each_capture_in_its_own_slot() {
    // Two captures read back with a non-commutative op: slot 0 is `b` (100),
    // slot 1 is `a` (1), so the body computes 100 - 1 = 99. The pairs come in the
    // other order, so seating in pair order instead of slot order would swap
    // them and give 1 - 100 = -99.
    let f = compiled(
        vec![Pop, LoadLocal(0), LoadLocal(1), Subtract],
        Arity::Exact(0),
        2,
        &["b", "a"],
    );
    let closure = f
        .assert_well_formed()
        .close([("a", Value::Int(1)), ("b", Value::Int(100))])
        .unwrap();
    assert_eq!(tail_of(closure), Value::Int(99));
}

// ============================================================
// close_with: capture lookup
// ============================================================

#[test]
fn close_with_asks_once_per_capture_in_capture_order() {
    // Capture names deliberately not in alphabetical order; `local` is not a
    // capture, so it is never asked for.
    let f = compiled(
        vec![Pop, LoadLocal(0), LoadLocal(1), Subtract],
        Arity::Exact(0),
        3,
        &["z", "x", "y", "local"],
    );
    let mut asked = Vec::new();
    let closure = f
        .assert_well_formed()
        .close_with(|name| {
            asked.push(name.to_string());
            Some(Value::Int(if name == "z" { 10 } else { 3 }))
        })
        .unwrap();
    assert_eq!(asked, ["z", "x", "y"], "once each, in capture order");
    assert_eq!(tail_of(closure), Value::Int(7), "z - x = 10 - 3");
}

#[test]
fn close_with_looks_captures_up_in_a_larger_environment() {
    let environment = BTreeMap::from([
        ("unused", Value::Null),
        ("x", Value::Int(9)),
        ("also_unused", Value::Null),
    ]);
    let closure = returns_x()
        .close_with(|name| environment.get(name).cloned())
        .unwrap();
    assert_eq!(tail_of(closure), Value::Int(9));
    assert_eq!(environment.len(), 3, "the environment is only read");
}

#[test]
fn close_with_reports_every_capture_lookup_returned_none_for() {
    // Only `x` is found; `z` and `y` are reported, in capture order, though
    // `lookup` is asked for every capture before the failure.
    let f = compiled(
        vec![Pop, LoadLocal(0)],
        Arity::Exact(0),
        3,
        &["z", "x", "y"],
    );
    let mut asked = 0;
    let err = f
        .assert_well_formed()
        .close_with(|name| {
            asked += 1;
            (name == "x").then_some(Value::Int(1))
        })
        .unwrap_err();
    assert_eq!(asked, 3, "every capture is asked for");
    assert_eq!(
        err,
        MissingCaptures {
            names: vec!["z".to_string(), "y".to_string()],
        }
    );
}

#[test]
fn close_with_fails_when_lookup_finds_nothing() {
    let err = returns_x().close_with(|_| None).unwrap_err();
    assert_eq!(err.names, ["x"]);
}

#[test]
fn close_with_never_asks_for_a_program_without_captures() {
    let f = compiled(vec![Pop, PushInt(7)], Arity::Exact(0), 0, &[]);
    let closure = f
        .assert_well_formed()
        .close_with(|name| panic!("asked for {name:?}, but nothing is captured"))
        .unwrap();
    assert_eq!(tail_of(closure), Value::Int(7));
}

// ============================================================
// into_closure
// ============================================================

#[test]
fn into_closure_for_no_captures() {
    let f = compiled(vec![Pop, PushInt(7)], Arity::Exact(0), 0, &[]);
    let closure = f.assert_well_formed().into_closure().unwrap();
    assert_eq!(run(Vm::factory().build(closure)).tail(), &Value::Int(7));
}

#[test]
fn into_closure_errors_when_host_capture_needed() {
    // into_closure supplies no captures, so a required host capture is missing.
    let f = compiled(vec![Pop, LoadLocal(0)], Arity::Exact(0), 1, &["x"]);
    let err = f.assert_well_formed().into_closure().unwrap_err();
    assert_eq!(
        err,
        MissingCaptures {
            names: vec!["x".to_string()]
        }
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

#[test]
fn a_well_formed_program_lists_only_its_captures() {
    let program = compiled(vec![Pop], Arity::Exact(0), 2, &["a", "b", "c"]).assert_well_formed();
    assert_eq!(program.capture_names().collect::<Vec<_>>(), ["a", "b"]);
}

#[test]
fn a_well_formed_program_lists_the_names_close_requires() {
    // Closing with exactly the listed names succeeds; leaving any out fails.
    let names = ["a", "b"];
    let program = || compiled(vec![Pop], Arity::Exact(0), 2, &names).assert_well_formed();
    let listed: Vec<String> = program().capture_names().map(str::to_string).collect();
    assert_eq!(listed, names);
    assert!(
        program()
            .close(names.map(|name| (name, Value::Null)))
            .is_ok()
    );
    let missing = program()
        .close(no_captures())
        .expect_err("no capture supplied");
    assert_eq!(missing.names, names);
}

#[test]
fn a_well_formed_program_without_captures_lists_none() {
    let program = compiled(vec![Pop], Arity::Exact(0), 0, &[]).assert_well_formed();
    assert_eq!(program.capture_names().count(), 0);
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
        .build(f.assert_well_formed().into_closure().unwrap())
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
    let closure = f
        .assert_well_formed()
        .close([("k", Value::Int(100))])
        .unwrap();
    let result = Vm::factory()
        .build(closure)
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
        .build(f.assert_well_formed().into_closure().unwrap())
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
        .build(f.assert_well_formed().into_closure().unwrap())
        .run_with_args([Value::Int(10)])
        .unwrap_err()
        .into_error();
    assert!(
        err.message().contains("expects 2 arguments"),
        "got: {}",
        err.message()
    );
    assert!(
        err.message().contains("called with 1"),
        "got: {}",
        err.message()
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
        .build(f.assert_well_formed().into_closure().unwrap())
        .run_with_args([Value::Int(1), Value::Int(2)])
        .unwrap_err()
        .into_error();
    assert!(
        err.message().contains("expects 1 arguments"),
        "got: {}",
        err.message()
    );
    assert!(
        err.message().contains("called with 2"),
        "got: {}",
        err.message()
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
        .build(f.assert_well_formed().into_closure().unwrap())
        .run_with_args([Value::Int(1)])
        .unwrap_err()
        .into_error();
    assert!(
        err.message().contains("at least 2"),
        "got: {}",
        err.message()
    );
    assert!(
        err.message().contains("called with 1"),
        "got: {}",
        err.message()
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
        .build(f.assert_well_formed().into_closure().unwrap())
        .run()
        .unwrap_err()
        .into_error();
    assert!(
        err.message().contains("expects 1 arguments"),
        "got: {}",
        err.message()
    );
    assert!(
        err.message().contains("called with 0"),
        "got: {}",
        err.message()
    );
}
