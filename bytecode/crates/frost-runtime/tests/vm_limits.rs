//! Tests for runtime resource limits (`VmRuntimeConfiguration`): the `fuel` call
//! budget and the `max_call_depth` cap.
//!
//! The load-bearing property is that **fuel exhaustion is uncatchable** -- a native
//! that swallows the error cannot resume Frost execution -- which is what makes the
//! call budget a real termination guarantee for untrusted scripts.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, NameEntry, ProgramResult,
    RunOutcome, Value, Vm, VmRuntimeConfiguration,
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

/// Build and run a top-level that captures `caps` (in slot order) and runs `code`
/// under `config`. `code` runs with the top-level's own closure value on the stack.
fn run(
    caps: Vec<(&str, Value)>,
    code: Vec<Bytecode>,
    config: VmRuntimeConfiguration,
) -> Result<ProgramResult, FrostError> {
    let name_table = caps.iter().map(|(n, _)| entry(n)).collect();
    let main = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table,
        num_captures: caps.len(),
        arity: Arity::Exact(0),
    });
    let captures: BTreeMap<String, Value> =
        caps.into_iter().map(|(n, v)| (n.to_string(), v)).collect();
    Vm::factory()
        .configuration(config)
        .build(main.assert_trusted().close(captures).unwrap())
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
}

fn unlimited() -> VmRuntimeConfiguration {
    VmRuntimeConfiguration::default()
}

fn with_fuel(budget: usize) -> VmRuntimeConfiguration {
    VmRuntimeConfiguration {
        fuel: NonZeroUsize::new(budget),
        ..Default::default()
    }
}

fn with_depth(max: usize) -> VmRuntimeConfiguration {
    VmRuntimeConfiguration {
        max_call_depth: NonZeroUsize::new(max),
        ..Default::default()
    }
}

/// A nullary native that returns `null`.
fn noop() -> Value {
    Value::native("noop", Arity::Exact(0), |_, _: &mut [Value]| {
        Ok(Value::Null)
    })
}

/// A nullary native that fails with an ordinary (catchable) error.
fn boom() -> Value {
    Value::native("boom", Arity::Exact(0), |_, _: &mut [Value]| {
        Err(FrostError::from_static("boom"))
    })
}

/// Invokes its single argument with no args, swallowing any error into a sentinel.
/// Stands in for `try_call`: it *tries* to catch whatever the invoked function raises.
fn catcher() -> Value {
    Value::native("catcher", Arity::Exact(1), |mut ctx, args| {
        match ctx.invoke(&args[0], std::iter::empty()) {
            Ok(value) => Ok(value),
            Err(_) => Ok(Value::from("swallowed")),
        }
    })
}

/// `recurse(self, n)`: invokes `self(self, n - 1)` until `n <= 0`, growing one native
/// frame per level -- an unbounded (non-tail) recursion knob for the depth cap.
fn recurse() -> Value {
    Value::native("recurse", Arity::Exact(2), |mut ctx, args| {
        let n = args[1].as_int().expect("recurse depth must be an Int");
        if n <= 0 {
            return Ok(Value::Int(0));
        }
        let f = args[0].clone();
        ctx.invoke(&f, [f.clone(), Value::Int(n - 1)])
    })
}

/// Top-level code that calls capture-slot-0 `n` times, discarding each result.
fn call_n_times(n: usize) -> Vec<Bytecode> {
    let mut code = vec![Pop]; // drop the top-level's own closure value
    for _ in 0..n {
        code.extend([LoadLocal(0), Call(0), Pop]);
    }
    code
}

// ============================================================
// Fuel: counting and the budget
// ============================================================

#[test]
fn fuel_consumed_reports_the_number_of_calls() {
    // Reported even when unmetered.
    let result = run(vec![("f", noop())], call_n_times(5), unlimited()).unwrap();
    assert_eq!(result.fuel_consumed(), 5);
}

#[test]
fn a_budget_admits_exactly_that_many_calls() {
    assert!(run(vec![("f", noop())], call_n_times(3), with_fuel(3)).is_ok());
}

#[test]
fn the_call_past_the_budget_fails() {
    assert!(run(vec![("f", noop())], call_n_times(4), with_fuel(3)).is_err());
}

// ============================================================
// Fuel exhaustion is uncatchable (the sandbox guarantee)
// ============================================================

// `catcher(target)` -- call the catcher (capture 0) with the target (capture 1).
fn catch_call() -> Vec<Bytecode> {
    vec![Pop, LoadLocal(0), LoadLocal(1), Call(1)]
}

#[test]
fn an_ordinary_error_can_be_caught() {
    // Control: `boom` fails recoverably, and the catcher swallows it into the sentinel.
    let result = run(
        vec![("c", catcher()), ("f", boom())],
        catch_call(),
        unlimited(),
    )
    .unwrap();
    assert_eq!(result.tail(), &Value::from("swallowed"));
}

#[test]
fn fuel_exhaustion_cannot_be_caught() {
    // Budget 1: the outer `Call` to the catcher is call #1 (admitted); the catcher's
    // inner `invoke` is call #2, which exhausts. The catcher swallows the error, but
    // the abort latch overrides its `Ok`, so the whole run fails instead of returning
    // the sentinel.
    assert!(
        run(
            vec![("c", catcher()), ("f", noop())],
            catch_call(),
            with_fuel(1)
        )
        .is_err()
    );
}

#[test]
fn fuel_exhaustion_carries_the_full_backtrace() {
    // Even though `catcher` swallows the error, the host must still see the trace to
    // wherever fuel ran out: the swallowing native (`catcher`) and the top level (`main`).
    let err = run(
        vec![("c", catcher()), ("f", noop())],
        catch_call(),
        with_fuel(1),
    )
    .unwrap_err();
    assert!(err.message().contains("fuel"), "message: {}", err.message());
    assert!(
        err.backtrace().contains(&"catcher".to_string()),
        "backtrace must reach the swallowing native: {:?}",
        err.backtrace()
    );
    assert!(
        err.backtrace().contains(&"main".to_string()),
        "backtrace must reach the top level: {:?}",
        err.backtrace()
    );
}

// ============================================================
// Call depth
// ============================================================

// `recurse(recurse, n)` -- recurse (capture 0) is both the callee and its own first arg.
fn recurse_to(n: i64) -> Vec<Bytecode> {
    vec![Pop, LoadLocal(0), LoadLocal(0), PushInt(n), Call(2)]
}

#[test]
fn recursion_within_the_depth_limit_succeeds() {
    let result = run(vec![("r", recurse())], recurse_to(4), with_depth(20)).unwrap();
    assert_eq!(result.tail(), &Value::Int(0));
}

#[test]
fn recursion_past_the_depth_limit_halts() {
    assert!(run(vec![("r", recurse())], recurse_to(100), with_depth(8)).is_err());
}
