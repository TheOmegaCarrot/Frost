//! What the compiler does with lambdas beyond their behavior: when a lambda is
//! usable in a constant fold, how effects stop folding, which names a lambda
//! captures, hoisting constant captures, tail calls in a lambda's body, and the
//! names compiled functions carry.
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
use frostlang_compile::{Optimization, OptimizationOptions};
use frostlang_runtime::{Bytecode, Value};

const FOLD: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantFold, true);

const FOLD_AND_PROPAGATE: OptimizationOptions = FOLD.with(Optimization::ConstantPropagate, true);

const PROPAGATE: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantPropagate, true);

/// Hoisting has effect only with propagation, which makes a capture's value known.
const PROPAGATE_AND_HOIST: OptimizationOptions = PROPAGATE.with(Optimization::CaptureHoist, true);

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
fn a_pure_lambda_folds_in_every_higher_order_global() {
    for (source, expected) in [
        ("select([1, 2, 3], fn v -> v > 1)", "[2, 3]"),
        ("fold([1, 2, 3], fn a, b -> a + b, 0)", "6"),
        ("each([1, 2], fn v -> v * 2)", "[1, 2]"),
        ("call(fn a, b -> a * b, [3, 4])", "12"),
        ("and_then(1, fn v -> v + 1)", "2"),
        ("or_else(null, fn -> 3)", "3"),
        // The lambda raises, but `try_call` turns that into a value.
        ("try_call(fn -> 1 / 0).error", r#""Division by zero""#),
        ("map [1, 2] with $($ + 1)", "[2, 3]"),
        ("filter [1, 2, 3] with fn v -> v != 2", "[1, 3]"),
        ("reduce [1, 2, 3] with fn a, b -> a * b", "6"),
    ] {
        assert_eq!(run(source), run(expected), "{source:?} is {expected}");
        assert_folds(source, FOLD);
    }
}

#[test]
fn a_pure_lambda_passed_to_or_stored_in_another_expression_folds() {
    for (source, expected) in [
        ("(fn f -> f(21))(fn v -> v * 2)", 42),
        ("[fn v -> v * 2][0](21)", 42),
        ("{f: fn v -> v + 1}.f(1)", 2),
        ("(fn g -> fn v -> g(v))(fn w -> w + 1)(1)", 2),
        ("$($1($2))(fn v -> v * 3, 2)", 6),
        ("do { def f = fn v -> v - 1; 5 }", 5),
    ] {
        assert_eq!(run(source), Value::Int(expected), "{source:?}");
        assert_folds(source, FOLD);
    }
}

#[test]
fn a_pure_lambda_that_raises_is_left_for_runtime() {
    let source = "transform([1, 0], fn v -> 1 / v)";
    let message = Script::new(source).raises();
    assert!(message.contains("Division by zero"), "{message}");
    assert_does_not_fold(source, FOLD);
    // Nor does a raising lambda in a branch never taken raise at all.
    assert_eq!(run("if false: (fn -> 1 / 0)() else: 1"), Value::Int(1));
}

#[test]
fn a_long_pure_recursion_runs_whether_or_not_it_folds() {
    // Pure, but perhaps too long for a fold to finish; if so, it runs at runtime.
    let source = r#"(fn count(n) -> if n == 0: "done" else: count(n - 1))(200000)"#;
    assert_eq!(run(source), Value::from("done"));
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
fn an_effect_anywhere_in_a_lambda_stops_it_folding() {
    for source in [
        // However deeply the effectful lambda is nested.
        "transform([1], (fn -> fn -> fn -> fn v -> if false: print(v) else: v)()()())",
        // In a lambda the body creates but never calls.
        r"
        transform([1], fn v -> {
            def unused = fn -> print(v)
            v
        })
        ",
        // In a callback the body passes on.
        "transform([1], fn v -> transform([v], fn w -> if false: print(w) else: w)[0])",
        // Creating mutable state is an effect too.
        "transform([1], fn v -> mutable_cell(v).get())",
        "transform([1], $(if false: print($) else: $))",
    ] {
        assert_eq!(run(source), ints(&[1]), "{source:?}");
        assert_does_not_fold(source, FOLD);
    }
}

#[test]
fn a_name_shadowing_an_impure_global_is_not_an_effect() {
    let source = "transform([1, 2], (fn print -> fn v -> print(v))(fn v -> v * 2))";
    assert_eq!(run(source), ints(&[2, 4]));
    assert_folds(source, FOLD);

    let source = r"
        def print = 3
        transform([1], fn v -> v + print)
        ";
    assert_eq!(run(source), ints(&[4]));
    assert_folds(source, FOLD_AND_PROPAGATE);
}

#[test]
fn an_effect_runs_exactly_once_per_call() {
    // Were a call folded, its effect would run at compile time instead, and the
    // cell would not see it.
    for (source, expected) in [
        (
            r"
            def c = mutable_cell(0)
            transform([1, 2, 3], fn v -> c.exchange(c.get() + v))
            c.get()
            ",
            6,
        ),
        (
            r"
            def c = mutable_cell(0)
            (fn -> c.exchange(5))()
            c.get()
            ",
            5,
        ),
        (
            r"
            def c = mutable_cell(0)
            def f = fn -> c.exchange(c.get() + 1)
            f()
            f()
            c.get()
            ",
            2,
        ),
        (
            r"
            def c = mutable_cell(0)
            (fn -> fn -> c.exchange(c.get() + 1))()()
            c.get()
            ",
            1,
        ),
    ] {
        assert_eq!(run(source), Value::Int(expected), "{source:?}");
    }
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
    let source = r"
        def k = 10
        transform([1, 2], fn v -> v * k)
        ";
    assert_eq!(run(source), ints(&[10, 20]));
    assert_folds(source, FOLD_AND_PROPAGATE);
    // Without propagation, `k` is read from its slot at runtime.
    assert_does_not_fold(source, FOLD);
}

#[test]
fn a_lambda_captures_only_the_outer_names_it_uses() {
    for (source, captures) in [
        ("fn -> x", 1),
        ("fn -> x + x + x", 1),
        (
            r"
            def y = x
            fn -> [x, y]
            ",
            2,
        ),
        ("fn a, b -> fn -> x", 1),
        ("$($ + x)", 1),
        // A global is not captured, but a binding shadowing one is.
        ("fn -> type(1)", 0),
        (
            r"
            def type = x
            fn -> type
            ",
            1,
        ),
        // Nor is a name the lambda binds itself before using it.
        (
            r"
            fn -> {
                def x = 1
                x
            }
            ",
            0,
        ),
        (
            r"
            fn -> do {
                def x = 1
                x
            }
            ",
            0,
        ),
        ("fn x -> x", 0),
        ("fn x(n) -> x", 0),
        ("fn -> fn x -> x", 0),
        // But a use before the binding is of the outer name.
        (
            r"
            fn -> {
                x
                def x = 1
                x
            }
            ",
            1,
        ),
        // A field name is not a use of a binding of that name.
        (
            r"
            def a = x
            fn m -> m.a
            ",
            0,
        ),
        // A nested lambda's needs are the outer one's too.
        ("fn -> fn -> x", 1),
        ("fn -> fn -> fn -> x", 1),
    ] {
        let lambda = code(source, UNOPTIMIZED).nested(0);
        assert_eq!(
            lambda.num_captures(),
            captures,
            "{source:?} captures {captures}: {lambda:?}"
        );
    }
}

#[test]
fn a_capture_passes_through_every_lambda_between_its_binding_and_its_use() {
    let outer = code("fn -> fn -> fn -> x", UNOPTIMIZED).nested(0);
    let middle = outer.nested(0);
    let inner = middle.nested(0);
    for lambda in [&outer, &middle, &inner] {
        assert_eq!(lambda.num_captures(), 1, "{lambda:?}");
    }
}

// --- Capture hoisting ---

/// The top-level code of `source` under exactly `optimization`, and its lambda.
fn lambda_of(source: &str, optimization: OptimizationOptions) -> (Emitted, Emitted) {
    let top = code(source, optimization);
    let lambda = top.nested(0);
    (top, lambda)
}

#[test]
fn a_constant_capture_is_hoisted_into_the_lambda() {
    let source = r"
        def k = 10
        fn v -> v * k
        ";
    let (top, lambda) = lambda_of(source, PROPAGATE_AND_HOIST);
    assert_eq!(lambda.num_captures(), 0, "`k` is not captured: {lambda:?}");
    assert_eq!(
        lambda.count(&Bytecode::PushInt(10)),
        1,
        "the lambda loads `k` itself: {lambda:?}"
    );
    assert_eq!(
        top.count(&Bytecode::PushInt(10)),
        1,
        "only the `def` pushes 10; none is pushed for the closure: {top:?}"
    );
}

#[test]
fn without_hoisting_a_constant_capture_is_pushed_at_creation() {
    let source = r"
        def k = 10
        fn v -> v * k
        ";
    let (top, lambda) = lambda_of(source, PROPAGATE);
    assert_eq!(lambda.num_captures(), 1, "{lambda:?}");
    assert_eq!(
        top.count(&Bytecode::PushInt(10)),
        2,
        "the `def`, then the propagated capture push: {top:?}"
    );
}

#[test]
fn without_propagation_there_is_nothing_to_hoist() {
    let hoist_only = UNOPTIMIZED.with(Optimization::CaptureHoist, true);
    let source = r"
        def k = 10
        fn v -> v * k
        ";
    let (_, lambda) = lambda_of(source, hoist_only);
    assert_eq!(lambda.num_captures(), 1, "{lambda:?}");
}

#[test]
fn only_constant_captures_are_hoisted() {
    // `x` is known only at runtime, so it is still captured.
    let source = r"
        def k = 10
        fn v -> v + k + x
        ";
    let (_, lambda) = lambda_of(source, PROPAGATE_AND_HOIST);
    assert_eq!(lambda.num_captures(), 1, "only `x` is captured: {lambda:?}");
    assert_eq!(lambda.count(&Bytecode::PushInt(10)), 1, "{lambda:?}");
}

#[test]
fn a_structured_constant_is_hoisted_into_the_lambdas_pool() {
    let source = r#"
        def s = "text"
        fn -> s
        "#;
    let (_, lambda) = lambda_of(source, PROPAGATE_AND_HOIST);
    assert_eq!(lambda.num_captures(), 0, "{lambda:?}");
    assert!(
        lambda
            .code
            .iter()
            .any(|op| matches!(op, Bytecode::LoadConst(_))),
        "{lambda:?}"
    );
}

#[test]
fn hoisting_carries_through_nested_lambdas() {
    let source = r"
        def k = 10
        fn -> fn -> k
        ";
    let (_, outer) = lambda_of(source, PROPAGATE_AND_HOIST);
    let inner = outer.nested(0);
    assert_eq!(outer.num_captures(), 0, "{outer:?}");
    assert_eq!(inner.num_captures(), 0, "{inner:?}");
    assert_eq!(inner.count(&Bytecode::PushInt(10)), 1, "{inner:?}");
}

#[test]
fn a_hoisted_capture_may_be_rebound_after_use() {
    // Like any capture, the lambda may use it, then shadow it with a binding.
    let source = r"
        def k = 1
        (fn -> {
            def a = k
            def k = 2
            a + k
        })()
        ";
    assert_eq!(run(source), Value::Int(3));
    let (_, lambda) = lambda_of(source, PROPAGATE_AND_HOIST);
    assert_eq!(lambda.num_captures(), 0, "{lambda:?}");
}

/// How many constant-pool loads the code makes.
fn pool_loads(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::LoadConst(_)))
        .count()
}

#[test]
fn hoisting_moves_a_pooled_capture_from_the_creator_into_the_lambda() {
    let source = r#"
        def s = "text"
        fn -> s
        "#;
    let (top, lambda) = lambda_of(source, PROPAGATE);
    let (hoisted_top, hoisted_lambda) = lambda_of(source, PROPAGATE_AND_HOIST);
    assert_eq!(
        (lambda.num_captures(), pool_loads(&lambda)),
        (1, 0),
        "unhoisted, the lambda reads its capture: {lambda:?}"
    );
    assert_eq!(
        (hoisted_lambda.num_captures(), pool_loads(&hoisted_lambda)),
        (0, 1),
        "hoisted, the lambda loads the constant from its own pool: {hoisted_lambda:?}"
    );
    assert_eq!(
        pool_loads(&hoisted_top) + 1,
        pool_loads(&top),
        "hoisted, the creator no longer loads the constant to push it: {hoisted_top:?} vs {top:?}"
    );
}

#[test]
fn every_kind_of_constant_is_hoisted() {
    let constants = r#"
        def n = null
        def t = true
        def fl = 1.5
        def s = "s"
        def a = [1, [2]]
        def m = {k: [3]}
        "#;
    assert_eq!(
        run(&format!(
            r"
            {constants}
            (fn -> [n, t, fl, s, a, m])()
            "
        )),
        run(r#"[null, true, 1.5, "s", [1, [2]], {k: [3]}]"#),
    );
    // The structures are constants only once folded.
    let fold_propagate_and_hoist = FOLD_AND_PROPAGATE.with(Optimization::CaptureHoist, true);
    let (_, lambda) = lambda_of(
        &format!(
            r"
            {constants}
            fn -> [n, t, fl, s, a, m]
            "
        ),
        fold_propagate_and_hoist,
    );
    assert_eq!(lambda.num_captures(), 0, "{lambda:?}");
}

#[test]
fn a_constant_bound_in_a_lambda_is_hoisted_into_its_nested_lambdas() {
    let source = r"
        fn -> {
            def k = 5
            fn -> k
        }
        ";
    let (_, outer) = lambda_of(source, PROPAGATE_AND_HOIST);
    assert_eq!(outer.nested(0).num_captures(), 0, "{outer:?}");
    let (_, outer) = lambda_of(source, PROPAGATE);
    assert_eq!(outer.nested(0).num_captures(), 1, "{outer:?}");
}

#[test]
fn only_constants_are_hoisted() {
    // A closure, a self-name, and a parameter are values only at runtime, even
    // when the parameter shadows a constant.
    let capturing = [
        // The second closure the top level creates captures the first.
        code(
            r"
            def g = fn -> 1
            fn -> g()
            ",
            PROPAGATE_AND_HOIST,
        )
        .nested(1),
        code("fn me() -> fn -> me", PROPAGATE_AND_HOIST)
            .nested(0)
            .nested(0),
        code(
            r"
            def k = 1
            fn k -> fn -> k
            ",
            PROPAGATE_AND_HOIST,
        )
        .nested(0)
        .nested(0),
    ];
    for lambda in capturing {
        assert_eq!(lambda.num_captures(), 1, "{lambda:?}");
    }
    assert_eq!(
        run(r"
            def k = 1
            (fn k -> fn -> k)(5)()
            "),
        Value::Int(5)
    );
}

#[test]
fn a_lambda_mixes_hoisted_and_captured_names() {
    let source = r"
        def k = 10
        fn -> fn -> k + x
        ";
    let (_, outer) = lambda_of(source, PROPAGATE_AND_HOIST);
    let inner = outer.nested(0);
    assert_eq!(outer.num_captures(), 1, "only `x`: {outer:?}");
    assert_eq!(inner.num_captures(), 1, "only `x`: {inner:?}");
    assert_eq!(inner.count(&Bytecode::PushInt(10)), 1, "{inner:?}");
    let source = r"
        def k = 10
        (fn -> fn -> k + x)()()
        ";
    let tail = Script::new(source).capture("x", Value::Int(1)).run();
    assert_eq!(tail, Value::Int(11));
}

#[test]
fn a_hoisted_capture_folds_within_the_lambda() {
    let every_optimization = FOLD_AND_PROPAGATE
        .with(Optimization::BranchEliminate, true)
        .with(Optimization::CaptureHoist, true);
    for source in [
        r"
        def k = 2 * 3
        fn -> k * 7
        ",
        r"
        def k = 2 * 3
        fn -> {
            def product = k * 7
            product
        }
        ",
    ] {
        let (_, lambda) = lambda_of(source, every_optimization);
        assert!(
            lambda.count(&Bytecode::PushInt(42)) > 0,
            "{source:?}: {lambda:?}"
        );
        assert_eq!(
            lambda.count(&Bytecode::Multiply),
            0,
            "{source:?}: {lambda:?}"
        );
        // Unhoisted, `k` is a capture, which the lambda cannot fold over.
        let (_, lambda) = lambda_of(source, FOLD_AND_PROPAGATE);
        assert_eq!(
            lambda.count(&Bytecode::Multiply),
            1,
            "{source:?}: {lambda:?}"
        );
    }
}

#[test]
fn a_hoisted_condition_eliminates_a_branch_in_the_lambda() {
    let eliminate = PROPAGATE.with(Optimization::BranchEliminate, true);
    let eliminate_and_hoist = eliminate.with(Optimization::CaptureHoist, true);
    let source = r"
        def c = false
        fn -> if c: 100 else: 200
        ";
    assert_eq!(
        run(r"
            def c = false
            (fn -> if c: 100 else: 200)()
            "),
        Value::Int(200)
    );

    let (_, lambda) = lambda_of(source, eliminate_and_hoist);
    assert_eq!(
        (
            lambda.count(&Bytecode::PushInt(100)),
            lambda.count(&Bytecode::PushInt(200))
        ),
        (0, 1),
        "only the taken branch is left: {lambda:?}"
    );
    // Unhoisted, the condition is a capture, unknown to the lambda.
    let (_, lambda) = lambda_of(source, eliminate);
    assert_eq!(
        (
            lambda.count(&Bytecode::PushInt(100)),
            lambda.count(&Bytecode::PushInt(200))
        ),
        (1, 1),
        "both branches are kept: {lambda:?}"
    );
}

#[test]
fn hoisting_keeps_each_closure_distinct() {
    // A lambda left with no captures is still created afresh each time its
    // expression is evaluated, so two closures from it are never equal.
    let source = r"
        def k = 1
        def make = fn -> fn -> k
        make() == make()
        ";
    assert_eq!(run(source), Value::Bool(false));
}

// --- Folding within a lambda ---

#[test]
fn a_lambdas_return_expression_folds() {
    // The return expression is a fold point, whatever form the body takes.
    for source in [
        "fn -> 6 * 7",
        "fn f() -> 6 * 7",
        "$(6 * 7)",
        "fn -> do { 6 * 7 }",
        r"
        fn -> {
            1
            6 * 7
        }
        ",
        r"
        fn -> {
            def a = 1
            6 * 7
        }
        ",
        r"
        fn ...args -> {
            args
            6 * 7
        }
        ",
        "fn -> if true: 6 * 7 else: 0",
        "fn -> plus(6, 36)",
        "fn -> (fn -> 6 * 7)()",
    ] {
        assert_eq!(
            run(&format!("({source})()")),
            Value::Int(42),
            "{source:?} returns 42"
        );
        let (_, lambda) = lambda_of(source, FOLD);
        assert_eq!(
            lambda.count(&Bytecode::PushInt(42)),
            1,
            "{source:?} returns a folded 42: {lambda:?}"
        );
        assert_eq!(
            lambda.count(&Bytecode::Multiply),
            0,
            "{source:?}: {lambda:?}"
        );
        assert_eq!(calls(&lambda), 0, "{source:?}: {lambda:?}");
    }
}

#[test]
fn without_folding_a_lambdas_return_expression_is_kept() {
    let (_, lambda) = lambda_of("fn -> 6 * 7", UNOPTIMIZED);
    assert_eq!(lambda.count(&Bytecode::Multiply), 1, "{lambda:?}");
}

#[test]
fn a_raising_return_expression_is_left_for_runtime() {
    let source = r"
        def f = fn -> 1 / 0
        f()
        ";
    let message = Script::new(source).raises();
    assert!(message.contains("Division by zero"), "{message}");
    let (_, lambda) = lambda_of("fn -> 1 / 0", FOLD);
    assert_eq!(lambda.count(&Bytecode::Divide), 1, "{lambda:?}");
}

#[test]
fn a_return_expression_holding_a_function_is_kept() {
    // Its value cannot be a constant, so its code is kept as it is.
    for (source, expected) in [
        ("(fn -> fn -> 42)()()", 42),
        ("(fn -> [fn -> 42])()[0]()", 42),
        ("(fn -> {f: fn -> 42})().f()", 42),
    ] {
        assert_eq!(run(source), Value::Int(expected), "{source:?}");
    }
    let (_, lambda) = lambda_of("fn -> [fn -> 42]", FOLD);
    assert_eq!(
        lambda.count(&Bytecode::MakeArray(1)),
        1,
        "the Array is built at runtime: {lambda:?}"
    );
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
        (
            r"
            fn n -> {
                def m = n
                x(m)
            }
            ",
            true,
        ),
        ("fn n -> x(n) + 1", false),
        (
            r"
            fn n -> {
                x(n)
                1
            }
            ",
            false,
        ),
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

/// How many (ordinary, tail) calls the code makes.
fn calls_by_kind(emitted: &Emitted) -> (usize, usize) {
    let count =
        |matching: fn(&Bytecode) -> bool| emitted.code.iter().filter(|op| matching(op)).count();
    (
        count(|op| matches!(op, Bytecode::Call(_))),
        count(|op| matches!(op, Bytecode::TailCall(_))),
    )
}

#[test]
fn tail_position_in_a_lambda_passes_through_every_construct() {
    for (source, ordinary, tail) in [
        ("fn n -> n and x(n)", 0, 1),
        ("fn n -> n or x(n)", 0, 1),
        ("fn n -> x(n) or n", 1, 0),
        ("fn n -> if n: 1 elif n: x(n) else: 2", 0, 1),
        ("fn n -> if n: x(1) elif n: x(2) else: x(3)", 0, 3),
        (
            r"
            fn n -> do {
                def m = n
                x(m)
            }
            ",
            0,
            1,
        ),
        (
            r"
            fn n -> {
                def m = n
                do { x(m) }
            }
            ",
            0,
            1,
        ),
        ("fn n -> n @ x()", 0, 1),
        ("fn n -> map n with x", 0, 1),
        ("$(x($))", 0, 1),
        // The callee is called in tail position; the call producing it is not.
        ("fn n -> x(n)(n)", 1, 1),
        (
            r"
            fn n -> do {
                def m = x(n)
                m
            }
            ",
            1,
            0,
        ),
        ("fn n -> if x(n): 1 else: 2", 1, 0),
        ("fn n -> [x(n)]", 1, 0),
        ("fn n -> {a: x(n)}", 1, 0),
        ("fn n -> not x(n)", 1, 0),
        ("fn n -> -x(n)", 1, 0),
        ("fn n -> x(n).a", 1, 0),
        ("fn n -> x(n)[0]", 1, 0),
        // A nested lambda's calls are its own.
        ("fn n -> fn -> x(n)", 0, 0),
    ] {
        let body = code(source, UNOPTIMIZED).nested(0);
        assert_eq!(
            calls_by_kind(&body),
            (ordinary, tail),
            "{source:?}: (ordinary, tail) calls: {body:?}"
        );
    }
}

#[test]
fn a_nested_lambda_has_its_own_tail_position() {
    let source = r"
        fn n -> {
            def g = fn -> x(n)
            g()
        }
        ";
    let outer = code(source, UNOPTIMIZED).nested(0);
    assert_eq!(calls_by_kind(&outer), (0, 1), "{outer:?}");
    let inner = outer.nested(0);
    assert_eq!(calls_by_kind(&inner), (0, 1), "{inner:?}");
}

#[test]
fn tail_position_in_a_lambda_holds_under_every_optimization() {
    let source = r"
        def k = 1
        fn n -> if n: x(n + k) else: x(k)
        ";
    let script = Script::new(source).capture("x", Value::Int(1));
    for emitted in script.code_where(|_| true) {
        let body = emitted.nested(0);
        assert_eq!(calls_by_kind(&body), (0, 2), "{body:?}");
    }
}

#[test]
fn tail_calls_through_every_construct_run_in_bounded_depth() {
    for source in [
        r"
        defn f(n) -> n == 0 or f(n - 1)
        f(100000)
        ",
        r"
        defn f(n) -> n != 0 and f(n - 1)
        f(100000) == false
        ",
        r#"
        defn f(n) -> if n == 0: true elif n < 0: "never" else: f(n - 1)
        f(100000)
        "#,
        r"
        defn f(n) -> do {
            def m = n - 1
            if m < 0: true else: f(m)
        }
        f(100000)
        ",
        r"
        defn f(n) -> if n == 0: true else: (n - 1) @ f()
        f(100000)
        ",
        // Through an abbreviated lambda, called in tail position.
        r"
        defn f(n) -> if n == 0: true else: $(f($))(n - 1)
        f(100000)
        ",
        // Mutually, through a nested named lambda.
        r"
        def even = fn even(n) -> if n == 0: true else: (fn odd(m) -> if m == 0: false else: even(m - 1))(n - 1)
        even(100000)
        ",
        // In a `defn` within a lambda, over a captured limit.
        r"
        (fn limit -> {
            defn go(i) -> if i == limit: true else: go(i + 1)
            go(0)
        })(100000)
        ",
    ] {
        let tail = Script::new(source).max_call_depth(100).run();
        assert_eq!(tail, Value::Bool(true), "{source:?}");
    }
}

#[test]
fn tail_recursion_runs_in_bounded_depth() {
    // A hundred thousand iterations under a depth limit of a hundred frames:
    // only possible if each recursive call reuses its frame.
    let source = r#"
        defn count(n) -> if n == 0: "done" else: count(n - 1)
        count(100000)
        "#;
    let tail = Script::new(source).max_call_depth(100).run();
    assert_eq!(tail, Value::from("done"));
}

#[test]
fn accumulating_tail_recursion_runs_in_bounded_depth() {
    let source = r"
        defn sum(n, total) -> if n == 0: total else: sum(n - 1, total + n)
        sum(10000, 0)
        ";
    let tail = Script::new(source).max_call_depth(100).run();
    assert_eq!(tail, Value::Int(50_005_000));
}

#[test]
fn non_tail_recursion_grows_the_stack() {
    // Far past the limit, so no optimization that lowers call depth can bring
    // it back under.
    let source = r"
        defn depth(n) -> if n == 0: 0 else: 1 + depth(n - 1)
        depth(100000)
        ";
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
fn each_nested_function_carries_its_own_name() {
    let outer = code("fn outer() -> fn inner() -> 1", UNOPTIMIZED).nested(0);
    assert_eq!(outer.name(), "outer");
    assert_eq!(outer.nested(0).name(), "inner");

    let source = r"
        fn -> {
            defn helper() -> 1
            helper
        }
        ";
    let outer = code(source, UNOPTIMIZED).nested(0);
    assert_eq!(outer.nested(0).name(), "helper");

    // An unnamed lambda does not take its enclosing function's name.
    let outer = code("fn named() -> fn -> 1", UNOPTIMIZED).nested(0);
    assert_ne!(outer.nested(0).name(), "named");
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
