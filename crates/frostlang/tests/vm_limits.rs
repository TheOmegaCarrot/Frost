//! Tests for runtime resource limits (`VmRuntimeConfiguration`): the `fuel` call
//! budget and the `max_call_depth` cap.
//!
//! The load-bearing property is that **fuel exhaustion is uncatchable** (a native
//! that swallows the error cannot resume Frost execution), which is what makes the
//! call budget a real termination guarantee for untrusted scripts.

use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};

use crate::common;

use common::Pop;
use frostlang::bytecode::{Bytecode, CompiledFunction, FormatVersion, NameEntry};
use frostlang::native::Function;
use frostlang::{
    AbortReason, Arity, FrostError, FrostResult, ProgramResult, RunError, RunErrorKind, Value, Vm,
    VmRuntimeConfiguration, frost_closure,
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
    run_keeping_vm(caps, code, config).map_err(RunError::into_error)
}

/// [`run`], keeping a failed run's whole [`RunError`].
#[allow(clippy::result_large_err)]
fn run_keeping_vm(
    caps: Vec<(&str, Value)>,
    code: Vec<Bytecode>,
    config: VmRuntimeConfiguration,
) -> Result<ProgramResult, RunError> {
    let name_table = caps.iter().map(|(n, _)| entry(n)).collect();
    let main = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        origin: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table,
        num_captures: caps.len(),
        arity: Arity::Exact(0),
        debug_info: None,
    });
    let captures: BTreeMap<String, Value> =
        caps.into_iter().map(|(n, v)| (n.to_string(), v)).collect();
    Vm::factory()
        .configuration(config)
        .build(main.assert_well_formed().close(captures).unwrap())
        .run()
}

fn unlimited() -> VmRuntimeConfiguration {
    VmRuntimeConfiguration::default()
}

fn with_fuel(budget: u64) -> VmRuntimeConfiguration {
    VmRuntimeConfiguration::default().with_fuel(NonZeroU64::new(budget))
}

fn with_depth(max: u64) -> VmRuntimeConfiguration {
    VmRuntimeConfiguration::default().with_max_call_depth(NonZeroU64::new(max))
}

/// A nullary native that returns `null`.
fn noop() -> Value {
    frost_closure!("noop", || Value::Null)
}

/// A nullary native that fails with an ordinary (catchable) error.
fn boom() -> Value {
    frost_closure!("boom", || -> FrostResult {
        Err(FrostError::from_static("boom"))
    })
}

/// Invokes its single argument with no args, swallowing any error into a sentinel.
/// Stands in for `try_call`: it *tries* to catch whatever the invoked function raises.
fn catcher() -> Value {
    frost_closure!("catcher", |#[frost(ctx)] ctx, function: Function| {
        ctx.invoke(&function, std::iter::empty())
            .unwrap_or_else(|_| Value::from("swallowed"))
    })
}

/// `recurse(self, n)`: invokes `self(self, n - 1)` until `n <= 0`, growing one native
/// frame per level; an unbounded (non-tail) recursion knob for the depth cap.
fn recurse() -> Value {
    frost_closure!("recurse", |#[frost(ctx)] ctx,
                               f: Function,
                               n: i64|
     -> FrostResult {
        if n <= 0 {
            return Ok(Value::Int(0));
        }
        ctx.invoke(&f, [f.clone().into_value(), Value::Int(n - 1)])
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

// `catcher(target)`: call the catcher (capture 0) with the target (capture 1).
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
        err.backtrace()
            .iter()
            .any(|frame| frame.name() == "catcher"),
        "backtrace must reach the swallowing native: {:?}",
        err.backtrace()
    );
    assert!(
        err.backtrace().iter().any(|frame| frame.name() == "main"),
        "backtrace must reach the top level: {:?}",
        err.backtrace()
    );
}

#[test]
fn a_native_sees_fuel_exhaustion_from_its_call_back_into_frost() {
    // Budget 1: the call to `checker` is call #1; its `invoke` is call #2, which
    // exhausts the fuel. The native swallows that error, then checks for an abort.
    let observed = Arc::new(Mutex::new(Vec::new()));
    let checker = {
        let observed = Arc::clone(&observed);
        frost_closure!("checker", move |#[frost(ctx)] ctx, function: Function| {
            observed.lock().unwrap().push(ctx.abort_reason());
            let _ = ctx.invoke(&function, []);
            observed.lock().unwrap().push(ctx.abort_reason());
            "finished"
        })
    };
    let failure = run_keeping_vm(
        vec![("c", checker), ("f", noop())],
        catch_call(),
        with_fuel(1),
    )
    .unwrap_err();
    assert_eq!(
        *observed.lock().unwrap(),
        [None, Some(AbortReason::FuelExhausted)],
        "before and after exhausting the fuel"
    );
    assert_eq!(
        failure.kind(),
        RunErrorKind::Aborted(AbortReason::FuelExhausted)
    );
}

// ============================================================
// Call depth
// ============================================================

// `recurse(recurse, n)`: recurse (capture 0) is both the callee and its own first arg.
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

// ============================================================
// Telling an abort from a raised error
// ============================================================

/// The kind of a run that must fail.
fn failure_kind(
    caps: Vec<(&str, Value)>,
    code: Vec<Bytecode>,
    config: VmRuntimeConfiguration,
) -> RunErrorKind {
    run_keeping_vm(caps, code, config)
        .expect_err("the run should fail")
        .kind()
}

#[test]
fn an_uncaught_error_is_raised() {
    let kind = failure_kind(
        vec![("f", boom())],
        vec![Pop, LoadLocal(0), Call(0)],
        unlimited(),
    );
    assert_eq!(kind, RunErrorKind::Raised);
}

#[test]
fn running_out_of_fuel_aborts() {
    let kind = failure_kind(vec![("f", noop())], call_n_times(4), with_fuel(3));
    assert_eq!(kind, RunErrorKind::Aborted(AbortReason::FuelExhausted));
}

#[test]
fn running_out_of_fuel_aborts_even_when_a_native_swallows_it() {
    let kind = failure_kind(
        vec![("c", catcher()), ("f", noop())],
        catch_call(),
        with_fuel(1),
    );
    assert_eq!(kind, RunErrorKind::Aborted(AbortReason::FuelExhausted));
}

#[test]
fn exceeding_the_call_depth_is_raised() {
    // Frost code can catch a call-depth error, so it is not an abort.
    let kind = failure_kind(vec![("r", recurse())], recurse_to(100), with_depth(8));
    assert_eq!(kind, RunErrorKind::Raised);
}

#[test]
fn a_recycled_vm_forgets_a_previous_abort() {
    let aborted = run_keeping_vm(vec![("f", noop())], call_n_times(4), with_fuel(3))
        .expect_err("the first run should run out of fuel");
    let raises = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        origin: None,
        code: vec![Pop, PushNull, ProduceError],
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
        debug_info: None,
    })
    .assert_well_formed()
    .into_closure()
    .unwrap();
    let kind = aborted
        .into_idle_vm()
        .build(raises)
        .run()
        .expect_err("the second run should raise")
        .kind();
    assert_eq!(kind, RunErrorKind::Raised);
}
