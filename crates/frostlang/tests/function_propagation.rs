//! Propagating a binding whose compile-time-known value cannot be a constant,
//! such as a function: a lookup of it still loads the binding, but a constant
//! fold can use the value, so a call to a known pure function over known
//! arguments folds to its result.
//!
//! The harness runs each behavioral case under every optimization permutation,
//! so this is checked never to change what a program does, including the
//! identity of a function value. The code assertions pin the options they are
//! about and confirm a call was actually folded.

mod script;

use frostlang::Value;
use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use script::{Script, UNOPTIMIZED, raises, run};

const FOLD: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantFold, true);

const PROPAGATE: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantPropagate, true);

const FOLD_AND_PROPAGATE: OptimizationOptions = UNOPTIMIZED
    .with(Optimization::ConstantFold, true)
    .with(Optimization::ConstantPropagate, true);

/// Whether `code` makes a call, tail or not.
fn calls(code: &[Bytecode]) -> bool {
    code.iter()
        .any(|op| matches!(op, Bytecode::Call(_) | Bytecode::TailCall(_)))
}

// Behavior, under every permutation.

#[test]
fn a_call_to_a_bound_function_computes_its_result() {
    let source = r"
        def calc = fn x -> x + 1
        calc(2)
    ";
    assert_eq!(run(source), Value::Int(3));
}

#[test]
fn a_call_to_a_bound_function_over_a_bound_value_computes_its_result() {
    let source = r"
        def n = 5
        def calc = fn x -> x + n
        calc(n)
    ";
    assert_eq!(run(source), Value::Int(10));
}

#[test]
fn a_recursive_bound_function_computes_its_result() {
    let source = r"
        defn fib(x) -> if x < 2: x else: fib(x - 1) + fib(x - 2)
        fib(10)
    ";
    assert_eq!(run(source), Value::Int(55));
}

#[test]
fn a_bound_function_is_one_value_however_often_it_is_loaded() {
    // A function compares by identity: every lookup of one binding is the same
    // function, and a function written twice is two functions.
    let source = r"
        def f = fn x -> x
        def g = f
        def h = fn x -> x
        [f == f, f == g, f == h, [f] == [g]]
    ";
    assert_eq!(
        run(source),
        Value::from(vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true),
        ])
    );
}

#[test]
fn a_function_calling_a_bound_function_computes_its_result() {
    let source = r"
        def inc = fn x -> x + 1
        def twice = fn x -> inc(inc(x))
        twice(1)
    ";
    assert_eq!(run(source), Value::Int(3));
}

#[test]
fn a_bound_function_called_inside_another_function_computes_its_result() {
    let source = r"
        def square = fn x -> x * x
        def f = fn y -> square(3) + y
        f(1)
    ";
    assert_eq!(run(source), Value::Int(10));
}

#[test]
fn a_destructured_function_computes_its_result() {
    let source = r"
        def [inc, n] = [fn x -> x + 1, 41]
        def {add} = {add: fn a, b -> a + b}
        [inc(n), add(1, 2)]
    ";
    assert_eq!(
        run(source),
        Value::from(vec![Value::Int(42), Value::Int(3)])
    );
}

#[test]
fn a_bound_pure_global_computes_its_result() {
    let source = r#"
        def up = to_upper
        up("frost")
    "#;
    assert_eq!(run(source), Value::from("FROST"));
}

#[test]
fn a_bound_function_with_an_effect_runs_it_each_call() {
    let script = Script::new(
        r"
        def show = fn x -> print(x)
        show(1)
        show(2)
        null
    ",
    );
    assert_eq!(script.printed(), vec!["1", "2"]);
}

#[test]
fn a_bound_function_that_raises_still_raises_when_called() {
    let message = raises(
        r"
        def divide_by_zero = fn x -> x / 0
        divide_by_zero(1)
    ",
    );
    assert!(
        message.contains("zero"),
        "the call raises at runtime, as written: {message}"
    );
}

#[test]
fn a_call_too_long_to_fold_still_computes_its_result() {
    // More calls than a fold may make, so the call is left to runtime.
    let source = r"
        defn count_down(n) -> if n == 0: 0 else: count_down(n - 1)
        count_down(150000)
    ";
    assert_eq!(run(source), Value::Int(0));
}

// Code shape, under pinned options.

#[test]
fn a_call_to_a_bound_function_folds_to_its_result() {
    let script = Script::new(
        r"
        def calc = fn x -> x + 1
        calc(2)
    ",
    );
    let folded = script.code(FOLD_AND_PROPAGATE);
    assert!(
        !calls(&folded.code),
        "the call is evaluated at compile time: {folded:?}"
    );
    assert_eq!(
        folded.code.last(),
        Some(&Bytecode::PushInt(3)),
        "the result is pushed directly: {folded:?}"
    );
}

#[test]
fn a_call_to_a_bound_function_needs_both_folding_and_propagation() {
    let script = Script::new(
        r"
        def calc = fn x -> x + 1
        calc(2)
    ",
    );
    for options in [FOLD, PROPAGATE] {
        let emitted = script.code(options);
        assert!(
            calls(&emitted.code),
            "the call is made at runtime: {emitted:?}"
        );
    }
}

#[test]
fn a_bound_function_is_loaded_not_created_again() {
    // Creating the function at each use would make each use a new function.
    let script = Script::new(
        r"
        def f = fn x -> x
        [f, f]
    ",
    );
    let emitted = script.code(FOLD_AND_PROPAGATE);
    let slot = emitted.slot_named("f");
    assert_eq!(
        emitted.count(&Bytecode::CreateClosure(0)),
        1,
        "the function is created once: {emitted:?}"
    );
    assert_eq!(
        emitted.count(&Bytecode::LoadLocal(slot)),
        2,
        "each use loads the binding: {emitted:?}"
    );
}

#[test]
fn a_recursive_bound_function_call_folds_to_its_result() {
    let script = Script::new(
        r"
        defn fib(x) -> if x < 2: x else: fib(x - 1) + fib(x - 2)
        fib(10)
    ",
    );
    let folded = script.code(FOLD_AND_PROPAGATE);
    assert!(!calls(&folded.code), "{folded:?}");
    assert_eq!(folded.code.last(), Some(&Bytecode::PushInt(55)));
}

#[test]
fn a_bound_function_called_inside_another_function_folds_there() {
    // `square` is a capture of the second function, known to its folds.
    let script = Script::new(
        r"
        def square = fn x -> x * x
        fn y -> square(3) + y
    ",
    );
    let outer = script.code(FOLD_AND_PROPAGATE).nested(1);
    assert!(
        !calls(&outer.code),
        "the call to the captured function folds: {outer:?}"
    );
    assert!(
        outer.code.contains(&Bytecode::PushInt(9)),
        "the folded result is pushed: {outer:?}"
    );
}

#[test]
fn a_call_with_an_effect_is_not_folded() {
    let script = Script::new(
        r"
        def show = fn x -> print(x)
        show(1)
    ",
    );
    let emitted = script.code(FOLD_AND_PROPAGATE);
    assert!(calls(&emitted.code), "{emitted:?}");
}

#[test]
fn a_call_too_long_to_fold_is_made_at_runtime() {
    let script = Script::new(
        r"
        defn count_down(n) -> if n == 0: 0 else: count_down(n - 1)
        count_down(150000)
    ",
    );
    let emitted = script.code(FOLD_AND_PROPAGATE);
    assert!(calls(&emitted.code), "{emitted:?}");
}
