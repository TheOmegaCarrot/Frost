//! Call lowering, end to end: compile full source, run it on the VM, and check
//! the result.
//!
//! The callee is evaluated first, then the arguments left to right, then the
//! call is made. The harness runs every behavioral case under every
//! optimization permutation.
//!
//! A [`Probe`] is a host function captured into the script: it records every
//! call it receives, making the calls themselves observable. As a capture it is
//! known only at runtime, so a call to it never folds.

mod common;

use std::sync::{Arc, Mutex};

use common::{Emitted, Script, UNOPTIMIZED, every_optimization, raises, run};
use frost_compile::OptimizationOptions;
use frost_runtime::{Arity, Bytecode, Value};

const FOLD: OptimizationOptions = OptimizationOptions {
    constant_fold: true,
    ..UNOPTIMIZED
};

/// A host function that records the arguments of every call and returns them
/// as an Array.
struct Probe {
    calls: Arc<Mutex<Vec<Vec<Value>>>>,
}

impl Probe {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// The probe as a Frost function value.
    fn function(&self) -> Value {
        let calls = Arc::clone(&self.calls);
        Value::native("probe", Arity::AtLeast(0), move |_, args| {
            calls.lock().unwrap().push(args.to_vec());
            Ok(Value::from(args.to_vec()))
        })
    }

    /// The calls one run made. The harness runs a script once per optimization
    /// permutation, so every run must have made exactly the same calls.
    fn calls_per_run(&self) -> Vec<Vec<Value>> {
        let calls = self.calls.lock().unwrap();
        let runs = every_optimization().count();
        assert_eq!(
            calls.len() % runs,
            0,
            "the runs made different numbers of calls: {calls:?}"
        );
        let per_run = calls.len() / runs;
        let first = calls[..per_run].to_vec();
        for (run, chunk) in calls.chunks(per_run.max(1)).enumerate() {
            assert_eq!(chunk, first.as_slice(), "run {run} made different calls");
        }
        first
    }
}

/// Run `source` with a probe captured as `f`, returning the tail value and the
/// calls one run made to the probe.
fn run_probed(source: &str) -> (Value, Vec<Vec<Value>>) {
    let probe = Probe::new();
    let tail = Script::new(source).capture("f", probe.function()).run();
    (tail, probe.calls_per_run())
}

fn ints(values: &[i64]) -> Vec<Value> {
    values.iter().copied().map(Value::Int).collect()
}

/// The code of `source` under exactly `optimization`, with the probe captured
/// as `f` and `x` a runtime-only Int.
fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    Script::new(source)
        .capture("f", Probe::new().function())
        .capture("x", Value::Int(1))
        .code(optimization)
}

/// How many calls remain in the code.
fn calls(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::Call(_)))
        .count()
}

// --- Calling ---

#[test]
fn a_global_is_called_with_its_arguments() {
    assert_eq!(run("plus(2, 3)"), Value::Int(5));
    assert_eq!(run("minus(2, 3)"), Value::Int(-1), "argument order kept");
    assert_eq!(run("id(7)"), Value::Int(7));
    assert_eq!(run("type(1.5)"), Value::from("Float"));
    assert_eq!(run("to_string(12)"), Value::from("12"));
}

#[test]
fn a_call_receives_every_argument_in_order() {
    let (tail, calls) = run_probed("f(1, 2, 3)");
    assert_eq!(calls, vec![ints(&[1, 2, 3])]);
    assert_eq!(
        tail,
        Value::from(ints(&[1, 2, 3])),
        "the call's result is its value"
    );
}

#[test]
fn a_call_may_take_no_arguments() {
    let (tail, calls) = run_probed("f()");
    assert_eq!(calls, vec![Vec::<Value>::new()]);
    assert_eq!(tail, Value::from(Vec::<Value>::new()));
}

#[test]
fn arguments_are_expressions() {
    let (_, calls) = run_probed("f(1 + 2, if true: 4 else: 5, do { def y = 6; y })");
    assert_eq!(calls, vec![ints(&[3, 4, 6])]);
}

#[test]
fn calls_nest() {
    assert_eq!(run("plus(plus(1, 2), times(3, 4))"), Value::Int(15));
    let (_, calls) = run_probed("f(f(1), f(2, 3))");
    assert_eq!(
        calls,
        vec![
            ints(&[1]),
            ints(&[2, 3]),
            vec![Value::from(ints(&[1])), Value::from(ints(&[2, 3]))],
        ],
        "the inner calls run first, left to right"
    );
}

#[test]
fn the_callee_is_an_expression() {
    for (x, expected) in [(true, 8), (false, 2)] {
        let tail = Script::new("(if x: plus else: minus)(5, 3)")
            .capture("x", Value::Bool(x))
            .run();
        assert_eq!(tail, Value::Int(expected), "x = {x}");
    }
    assert_eq!(
        run("id(plus)(2, 3)"),
        Value::Int(5),
        "a call's result is called"
    );
    assert_eq!(run("do { def g = times; g }(4, 5)"), Value::Int(20));
}

#[test]
fn a_call_is_an_operand() {
    assert_eq!(run("plus(1, 2) * 10"), Value::Int(30));
    assert_eq!(run("not is_int(1.5)"), Value::Bool(true));
    assert_eq!(run("def y = plus(1, 2); y + y"), Value::Int(6));
}

// --- Evaluation ---

#[test]
fn each_evaluated_call_runs_exactly_once() {
    let (tail, calls) = run_probed("f(1); f(2); 3");
    assert_eq!(
        calls,
        vec![ints(&[1]), ints(&[2])],
        "statement calls run for effect"
    );
    assert_eq!(tail, Value::Int(3));

    let (_, calls) = run_probed("do { f(1); def y = f(2); f(3) }");
    assert_eq!(calls, vec![ints(&[1]), ints(&[2]), ints(&[3])]);
}

#[test]
fn an_unevaluated_call_does_not_run() {
    for (source, expected) in [
        ("false and f(1)", vec![]),
        ("true or f(1)", vec![]),
        ("if true: f(1) else: f(2)", vec![ints(&[1])]),
        ("if false: f(1) else: f(2)", vec![ints(&[2])]),
        ("if false: f(1)", vec![]),
    ] {
        let (_, calls) = run_probed(source);
        assert_eq!(calls, expected, "{source:?}");
    }
}

#[test]
fn the_callee_is_evaluated_before_the_arguments() {
    let message = raises("(1 / 0)(1 % 0)");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn arguments_are_evaluated_left_to_right() {
    let message = raises("plus(1 / 0, 1 % 0)");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn arguments_are_evaluated_before_arity_is_checked() {
    let message = raises("plus(1 / 0)");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn an_argument_error_prevents_the_call() {
    let probe = Probe::new();
    let message = Script::new("f(1, 1 / 0)")
        .capture("f", probe.function())
        .raises();
    assert!(message.contains("Division by zero"), "{message}");
    assert!(
        probe.calls_per_run().is_empty(),
        "the probe was never called"
    );
}

// --- Call errors ---

#[test]
fn calling_a_non_function_raises() {
    for (source, type_name) in [("5(1)", "Int"), (r#""f"()"#, "String"), ("null()", "Null")] {
        let message = raises(source);
        assert!(
            message.contains("non-function") && message.contains(type_name),
            "{source:?}: {message}"
        );
    }
}

#[test]
fn a_wrong_argument_count_raises() {
    for source in ["plus(1)", "plus(1, 2, 3)", "id()"] {
        let message = raises(source);
        assert!(message.contains("expects"), "{source:?}: {message}");
    }
}

#[test]
fn an_error_raised_by_the_callee_propagates() {
    let message = raises(r#"error("boom")"#);
    assert!(message.contains("boom"), "{message}");
    let message = raises(r#"plus(1, "a")"#);
    assert!(message.contains("Int + String"), "{message}");
}

#[test]
fn a_call_statement_leaves_the_stack_balanced() {
    assert_eq!(run("plus(1, 2); id(3); 5"), Value::Int(5));
    let (tail, _) = run_probed("f(); f(1, 2); 5");
    assert_eq!(tail, Value::Int(5));
}

// --- Constant folding ---

#[test]
fn a_pure_global_call_over_constants_folds() {
    let emitted = code("plus(2, 3)", FOLD);
    assert_eq!(calls(&emitted), 0, "the call is folded away: {emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");

    let emitted = code("plus(2, 3)", UNOPTIMIZED);
    assert_eq!(calls(&emitted), 1, "unfolded, the call stays: {emitted:?}");
}

#[test]
fn nested_pure_calls_fold_whole() {
    let emitted = code("plus(plus(1, 2), times(3, 4))", FOLD);
    assert_eq!(calls(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(15)), 1, "{emitted:?}");
}

#[test]
fn a_pure_call_with_a_runtime_argument_folds_its_constant_arguments() {
    let emitted = code("plus(x, 2 * 3)", FOLD);
    assert_eq!(calls(&emitted), 1, "the call stays: {emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(6)), 1, "{emitted:?}");
}

#[test]
fn a_call_to_a_runtime_callee_does_not_fold() {
    // The probe is a capture: the callee is known only at runtime.
    let emitted = code("f(1 + 2)", FOLD);
    assert_eq!(calls(&emitted), 1, "{emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::Add),
        0,
        "its constant argument folds: {emitted:?}"
    );
    assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
}

#[test]
fn a_call_to_an_impure_global_does_not_fold() {
    let emitted = code("mutable_cell(1 + 2)", FOLD);
    assert_eq!(calls(&emitted), 1, "{emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::Add),
        0,
        "its constant argument folds: {emitted:?}"
    );
}

#[test]
fn a_raising_pure_call_is_left_for_runtime() {
    for source in [r#"error("boom")"#, "plus(1)", r#"plus(1, "a")"#, "5(1)"] {
        let emitted = code(source, FOLD);
        assert_eq!(
            calls(&emitted),
            1,
            "{source:?}: the call is kept: {emitted:?}"
        );
    }
}

#[test]
fn a_function_valued_call_is_left_for_runtime() {
    // The result is a function, which cannot be a constant.
    let emitted = code("id(plus)", FOLD);
    assert_eq!(calls(&emitted), 1, "{emitted:?}");
}

#[test]
fn a_folded_call_is_a_constant_for_its_parent() {
    // `plus(1, 2)` folds as a sibling of the runtime `x`.
    let emitted = code("plus(1, 2) * x", FOLD);
    assert_eq!(calls(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Multiply), 1, "{emitted:?}");
}
