//! What the compiler does with lambdas beyond their behavior: when a lambda is
//! usable in a constant fold, how effects stop folding, tail calls in a lambda's
//! body, and the names compiled functions carry.
//!
//! A lambda is usable in a fold when calling it has no effects (its body, and
//! any lambda it creates, loads no impure global) and every value it captures is
//! known at compile time. Lambda behavior itself is covered by
//! `lambda_behavior.rs`.
//!
//! Behavioral cases run under every optimization permutation; code-shape cases
//! pin exactly the options they are about.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, run};
use frost_compile::OptimizationOptions;
use frost_runtime::{Bytecode, Value};

const FOLD: OptimizationOptions = OptimizationOptions {
    constant_fold: true,
    ..UNOPTIMIZED
};

const FOLD_AND_PROPAGATE: OptimizationOptions = OptimizationOptions {
    constant_propagate: true,
    ..FOLD
};

fn ints(values: &[i64]) -> Value {
    Value::from_iter(values.iter().copied().map(Value::Int))
}

/// The top-level code of `source` under exactly `optimization`, with `x` a
/// runtime-only Int.
fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    Script::new(source)
        .capture("x", Value::Int(1))
        .code(optimization)
}

/// How many calls, tail or not, the code makes.
fn calls(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::Call(_) | Bytecode::TailCall(_)))
        .count()
}

/// Assert `source` folds away entirely: no call is left to make at runtime.
fn assert_folds(source: &str, optimization: OptimizationOptions) {
    let emitted = code(source, optimization);
    assert_eq!(calls(&emitted), 0, "{source:?} folds away: {emitted:?}");
    assert!(
        !emitted
            .code
            .iter()
            .any(|op| matches!(op, Bytecode::CreateClosure(_))),
        "{source:?}: no closure is created at runtime: {emitted:?}"
    );
}

/// Assert `source`'s call is left for runtime.
fn assert_does_not_fold(source: &str, optimization: OptimizationOptions) {
    let emitted = code(source, optimization);
    assert!(calls(&emitted) > 0, "{source:?} is not folded: {emitted:?}");
}

// --- Usable in folds ---

#[test]
fn an_immediately_called_pure_lambda_folds() {
    assert_eq!(run("(fn x -> x + 1)(2)"), Value::Int(3));
    assert_folds("(fn x -> x + 1)(2)", FOLD);
    let emitted = code("(fn x -> x + 1)(2)", FOLD);
    assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
}

#[test]
fn a_higher_order_call_over_a_pure_lambda_folds() {
    for source in [
        "transform([1, 2, 3], fn v -> v * 2)",
        "transform([1, 2, 3], $($ * 2))",
    ] {
        assert_eq!(run(source), ints(&[2, 4, 6]), "{source:?}");
        assert_folds(source, FOLD);
    }
}

#[test]
fn a_curried_pure_function_folds_through_its_returned_closure() {
    // The outer lambda returns a closure over its parameters; the result is
    // never a constant itself, but it is usable within the enclosing fold.
    let source = "transform([1, 2, 3], (fn f, a -> fn b -> f(a, b))(plus, 2))";
    assert_eq!(run(source), ints(&[3, 4, 5]));
    assert_folds(source, FOLD);
}

#[test]
fn a_recursive_pure_function_folds() {
    let source = "(fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1))(10)";
    assert_eq!(run(source), Value::Int(3_628_800));
    assert_folds(source, FOLD);
}

#[test]
fn a_lambda_is_never_itself_a_constant() {
    // A Function cannot be pooled, so the closure is always created at runtime.
    let emitted = code("fn v -> v", FOLD);
    assert_eq!(emitted.count(&Bytecode::CreateClosure(0)), 1, "{emitted:?}");
}

// --- Effects ---

#[test]
fn a_lambda_mentioning_an_impure_global_does_not_fold() {
    // `print` is never called here, but mentioning it is enough: the check is
    // conservative, and never runs an effect at compile time.
    let source = "transform([1, 2], fn v -> if false: print(v) else: v)";
    assert_eq!(run(source), ints(&[1, 2]));
    assert_does_not_fold(source, FOLD);
}

#[test]
fn an_effect_in_a_nested_lambda_stops_the_outer_one_folding() {
    // The outer lambda only creates the effectful one, but the closure it returns
    // would run the effect when called within the fold.
    let source = "transform([1, 2], (fn -> fn v -> if false: print(v) else: v)())";
    assert_eq!(run(source), ints(&[1, 2]));
    assert_does_not_fold(source, FOLD);
}

#[test]
fn a_lambda_mentioning_only_pure_globals_folds() {
    assert_folds(
        "transform([1, -2], fn v -> type(v) == \"Int\" and to_string(v))",
        FOLD,
    );
}

// --- Captures ---

#[test]
fn a_lambda_capturing_a_runtime_value_does_not_fold() {
    let source = "transform([1, 2], fn v -> v + x)";
    let tail = Script::new(source).capture("x", Value::Int(10)).run();
    assert_eq!(tail, ints(&[11, 12]));
    assert_does_not_fold(source, FOLD);
}

#[test]
fn a_lambda_capturing_a_propagated_constant_folds() {
    let source = "def k = 10; transform([1, 2], fn v -> v * k)";
    assert_eq!(run(source), ints(&[10, 20]));
    assert_folds(source, FOLD_AND_PROPAGATE);
    // Without propagation, `k` is read from its slot at runtime.
    assert_does_not_fold(source, FOLD);
}

// --- Folding limits ---

#[test]
fn a_runaway_recursion_is_left_for_runtime() {
    // The fold's own call budget stops it; the recursion then fails at runtime,
    // under the depth limit, exactly as it does unoptimized.
    let source = "(fn deep(n) -> 1 + deep(n))(0)";
    let message = Script::new(source).max_call_depth(200).raises();
    assert!(message.contains("maximum call depth"), "{message}");
    assert_does_not_fold(source, FOLD);
}

// --- Tail calls ---

#[test]
fn a_call_in_a_lambdas_tail_position_is_a_tail_call() {
    for (source, tail) in [
        ("fn n -> x(n)", true),
        ("fn n -> if n: x(n) else: x(0)", true),
        ("fn n -> { def m = n; x(m) }", true),
        ("fn n -> x(n) + 1", false),
        ("fn n -> { x(n); 1 }", false),
    ] {
        let body = code(source, UNOPTIMIZED).nested(0);
        let tail_calls = body
            .code
            .iter()
            .filter(|op| matches!(op, Bytecode::TailCall(_)))
            .count();
        assert_eq!(tail_calls > 0, tail, "{source:?}: {body:?}");
    }
}

#[test]
fn tail_recursion_runs_in_bounded_depth() {
    // A hundred thousand iterations under a depth limit of a hundred frames:
    // only possible if each recursive call reuses its frame.
    let source = r#"defn count(n) -> if n == 0: "done" else: count(n - 1); count(100000)"#;
    let tail = Script::new(source).max_call_depth(100).run();
    assert_eq!(tail, Value::from("done"));
}

#[test]
fn accumulating_tail_recursion_runs_in_bounded_depth() {
    let source =
        "defn sum(n, total) -> if n == 0: total else: sum(n - 1, total + n); sum(10000, 0)";
    let tail = Script::new(source).max_call_depth(100).run();
    assert_eq!(tail, Value::Int(50_005_000));
}

#[test]
fn non_tail_recursion_grows_the_stack() {
    // Far past the limit, so no optimization that lowers call depth can bring
    // it back under.
    let source = "defn depth(n) -> if n == 0: 0 else: 1 + depth(n - 1); depth(100000)";
    let message = Script::new(source).max_call_depth(100).raises();
    assert!(message.contains("maximum call depth"), "{message}");
}

// --- Names ---

#[test]
fn a_function_is_named_for_its_self_name() {
    assert_eq!(
        code("fn fact(n) -> n", UNOPTIMIZED).nested(0).name(),
        "fact"
    );
    assert_eq!(
        code("defn greet(n) -> n", UNOPTIMIZED).nested(0).name(),
        "greet"
    );
}

#[test]
fn an_unnamed_function_is_given_a_name() {
    // Every function has a name; one the script leaves unnamed gets one that no
    // script name can refer to.
    for source in ["fn n -> n", "$($ + 1)"] {
        let name = code(source, UNOPTIMIZED).nested(0).name().to_string();
        assert!(!name.is_empty(), "{source:?}");
        assert!(
            !name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "{source:?}: `{name}` must not be a valid identifier"
        );
    }
}
