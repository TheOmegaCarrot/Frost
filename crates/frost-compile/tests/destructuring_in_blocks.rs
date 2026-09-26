//! Destructuring inside blocks: `do` expressions and block-bodied lambdas.
//!
//! A block is its own scope, so its destructured bindings are visible only inside
//! it, may shadow enclosing names, and leave the stack as they found it. In a
//! lambda, destructuring is how a structured argument is taken apart. Array and
//! Map patterns themselves are covered by `array_destructuring.rs` and
//! `map_destructuring.rs`.
//!
//! The harness runs every behavioral case under every optimization permutation;
//! code-shape cases pin exactly the options they are about.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, compile_errors, raises, run};
use frost_compile::OptimizationOptions;
use frost_runtime::{Bytecode, Value};

const FOLD: OptimizationOptions = OptimizationOptions {
    constant_fold: true,
    ..UNOPTIMIZED
};

/// Assert each `source` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(run(source), run(expected), "{source:?} is {expected}");
    }
}

/// Assert each `source` raises an error mentioning `message`.
fn assert_raises(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        let raised = raises(source);
        assert!(
            raised.contains(message),
            "{source:?} raises about {message:?}, but raised: {raised}"
        );
    }
}

/// Assert each `source` fails to compile, with a diagnostic mentioning `message`.
fn assert_compile_errors(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains(message),
            "{source:?} is rejected for {message:?}, but the diagnostic is:\n{rendered}"
        );
    }
}

fn calls(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::Call(_) | Bytecode::TailCall(_)))
        .count()
}

// --- In a `do` expression ---

#[test]
fn a_do_block_may_destructure() {
    assert_values(&[
        ("do { def [a, b] = [1, 2]; a + b }", "3"),
        ("do { def {w, h} = {w: 3, h: 4}; w * h }", "12"),
        (
            "do { def [a, ...r] = [1, 2, 3]; def {b} as m = {b: a}; [a, r, b, m] }",
            "[1, [2, 3], 1, {b: 1}]",
        ),
        ("do { def [{a}, [b]] = [{a: 1}, [2]]; [a, b] }", "[1, 2]"),
    ]);
}

#[test]
fn a_destructure_may_use_an_earlier_one_in_the_block() {
    assert_values(&[
        ("do { def [a, b] = [1, 2]; def {c} = {c: a + b}; c }", "3"),
        (r#"do { def [k] = ["b"]; def {[k]: v} = {b: 5}; v }"#, "5"),
        (
            "do { def [xs] = [[1, 2]]; def [h, ...t] = xs; [h, t] }",
            "[1, [2]]",
        ),
    ]);
}

#[test]
fn a_do_blocks_bindings_stay_inside_it() {
    assert_compile_errors(&[
        ("do { def [a] = [1]; a }; a", "`a` is not defined"),
        ("do { def {a} as m = {a: 1}; a }; m", "`m` is not defined"),
    ]);
}

#[test]
fn a_do_blocks_destructure_may_shadow_an_enclosing_name() {
    assert_values(&[
        ("def a = 1; do { def [a] = [2]; a } + a", "3"),
        (
            "def m = 0; [do { def {} as m = {k: 1}; m }, m]",
            "[{k: 1}, 0]",
        ),
        // Nested blocks shadow in turn.
        (
            "do { def [a] = [1]; [a, do { def {a} = {a: 2}; a }, a] }",
            "[1, 2, 1]",
        ),
    ]);
}

#[test]
fn a_pattern_may_shadow_a_name_its_own_value_reads() {
    // The value is evaluated first, reading the enclosing `x`.
    assert_values(&[
        ("def x = [1, 2]; do { def [x, y] = x; [y, x] }", "[2, 1]"),
        ("def m = {m: 5}; do { def {m} = m; m }", "5"),
    ]);
}

#[test]
fn a_name_destructured_twice_in_one_block_is_a_compile_error() {
    assert_compile_errors(&[
        (
            "do { def [a] = [1]; def {a} = {a: 2}; a }",
            "`a` is already bound",
        ),
        ("do { def a = 1; def [a] = [2]; a }", "`a` is already bound"),
    ]);
}

#[test]
fn block_destructuring_leaves_the_stack_balanced() {
    assert_values(&[
        (
            "do { def [a, ...r] = [1, 2, 3]; def {b} as m = {b: 4}; 5 }",
            "5",
        ),
        (
            "do { def [a] = [1]; a }; do { def {b} = {b: 2}; b }; 7",
            "7",
        ),
        (
            "[do { def [a, b] = [1, 2]; a }, do { def {c} = {c: 3}; c }]",
            "[1, 3]",
        ),
    ]);
}

#[test]
fn a_failed_destructure_ends_the_block() {
    assert_raises(&[
        ("do { def [a] = [1, 2]; a }", "exactly 1 element"),
        ("do { def {a} = {}; a }", "no value at key 'a'"),
        ("do { def [a] = [1]; def {b} = 5; a }", "Map"),
    ]);
}

#[test]
fn a_do_blocks_bindings_are_not_implicitly_exported() {
    let finished = Script::new("def r = do { def [a] = [1]; def {b} = {b: 2}; a + b }; r")
        .implicit_export()
        .finish();
    assert_eq!(finished.exports.keys().collect::<Vec<_>>(), vec!["r"]);
}

#[test]
fn a_constant_destructuring_block_folds_whole() {
    for source in [
        "do { def [a, b] = [1, 2]; 5 }",
        "do { def {a} as m = {a: 1}; def [x, ...y] = [1, 2]; 5 }",
    ] {
        let emitted = Script::new(source).code(FOLD);
        assert!(
            !emitted
                .code
                .iter()
                .any(|op| matches!(op, Bytecode::DefLocal(_))),
            "{source:?}: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::PushInt(5)),
            1,
            "{source:?}: {emitted:?}"
        );
    }
}

// --- In a lambda's block body ---

#[test]
fn a_lambda_may_destructure_its_arguments() {
    assert_values(&[
        ("(fn pair -> { def [a, b] = pair; a * b })([3, 4])", "12"),
        (
            "(fn opts -> { def {w, h} = opts; w * h })({w: 3, h: 4, unused: 0})",
            "12",
        ),
        (
            "(fn p -> { def {name, tags: [first, ...rest]} as all = p; [name, first, rest, all.name] })({name: \"n\", tags: [1, 2]})",
            r#"["n", 1, [2], "n"]"#,
        ),
        (
            "(fn a, b -> { def [x] = a; def {y} = b; x + y })([1], {y: 2})",
            "3",
        ),
    ]);
}

#[test]
fn a_lambda_destructures_afresh_on_each_call() {
    assert_values(&[(
        "def f = fn p -> { def [a, ...r] = p; [a, r] }; [f([1]), f([2, 3, 4]), f([5, 6])]",
        "[[1, []], [2, [3, 4]], [5, [6]]]",
    )]);
    assert_raises(&[(
        "def f = fn p -> { def [a, b] = p; a }; [f([1, 2]), f([1])]",
        "exactly 2 elements",
    )]);
}

#[test]
fn a_destructured_binding_may_be_captured() {
    assert_values(&[
        (
            "def make = fn p -> { def [a, b] = p; fn -> a + b }; make([1, 2])()",
            "3",
        ),
        (
            "def make = fn m -> { def {k} = m; fn x -> x * k }; transform([1, 2], make({k: 10}))",
            "[10, 20]",
        ),
    ]);
}

#[test]
fn a_lambdas_computed_key_may_read_a_capture() {
    assert_values(&[(
        r#"def key = "x"; (fn m -> { def {[key]: v} = m; v })({x: 1})"#,
        "1",
    )]);
}

#[test]
fn a_recursive_lambda_may_destructure() {
    assert_values(&[
        (
            "defn sum(xs) -> if xs == []: 0 else: do { def [h, ...t] = xs; h + sum(t) }; sum([1, 2, 3, 4])",
            "10",
        ),
        (
            "defn pairs(m) -> { def {a, b} = m; if a == 0: b else: pairs({a: a - 1, b: b + a}) }; pairs({a: 4, b: 0})",
            "10",
        ),
    ]);
}

#[test]
fn tail_recursion_through_destructuring_runs_in_bounded_depth() {
    // Build a long Array, then walk it by destructuring, both by tail recursion.
    let source = r"defn build(n, acc) -> if n == 0: acc else: build(n - 1, acc + [n])
        defn total(xs, acc) -> if xs == []: acc else: do {
            def [h, ...t] = xs
            total(t, acc + h)
        }
        total(build(2000, []), 0)";
    let tail = Script::new(source).max_call_depth(100).run();
    assert_eq!(tail, Value::Int(2_001_000));
}

#[test]
fn an_abbreviated_lambda_may_destructure_in_a_do_block() {
    assert_values(&[(
        "transform([[1, 2], [3, 4]], $(do { def [a, b] = $; a * b }))",
        "[2, 12]",
    )]);
}

#[test]
fn a_destructure_may_not_rebind_a_parameter() {
    // A lambda's body shares its parameters' scope.
    assert_compile_errors(&[
        ("fn a -> { def [a] = [1]; a }", "`a` is already bound"),
        (
            "fn m -> { def {k} as m = {k: 1}; k }",
            "`m` is already bound",
        ),
    ]);
}

#[test]
fn a_pure_lambda_that_destructures_is_usable_in_a_fold() {
    let source = "transform([[1, 2], [3, 4]], fn p -> { def [a, b] = p; a * b })";
    assert_values(&[(source, "[2, 12]")]);
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(calls(&emitted), 0, "folded away: {emitted:?}");
}
