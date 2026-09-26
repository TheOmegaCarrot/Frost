//! The iteration forms, end to end: `filter`, `map`, `reduce`, and `foreach`.
//!
//! Each form is syntax for a call to a global: `filter xs with f` is
//! `select(xs, f)`, `map xs with f` is `transform(xs, f)`,
//! `reduce xs [init: i] with f` is `fold(xs, f[, i])`, and `foreach xs with f` is
//! `each(xs, f)`. The globals' own semantics are the runtime's; these tests pin
//! that each form calls the right one, with its operands in the right order,
//! whatever the script names its own bindings.
//!
//! The harness runs every behavioral case under every optimization permutation;
//! code-shape cases pin exactly the options they are about.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, raises, run};
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

/// The code of `source` under exactly `optimization`, with `xs` a runtime-only
/// Array.
fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    Script::new(source)
        .capture("xs", Value::from_iter([Value::Int(1), Value::Int(2)]))
        .code(optimization)
}

fn calls(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::Call(_) | Bytecode::TailCall(_)))
        .count()
}

// --- Each form ---

#[test]
fn filter_keeps_the_elements_its_operation_accepts() {
    assert_values(&[
        ("filter [1, 2, 3, 4] with fn x -> x > 2", "[3, 4]"),
        ("filter [] with fn x -> true", "[]"),
        ("filter {a: 1, b: 2} with fn k, v -> v > 1", "{b: 2}"),
        ("filter {} with fn k, v -> true", "{}"),
    ]);
}

#[test]
fn map_transforms_each_element() {
    assert_values(&[
        ("map [1, 2, 3] with fn x -> x * 2", "[2, 4, 6]"),
        ("map [] with fn x -> x", "[]"),
        (
            "map {a: 1, b: 2} with fn k, v -> {[k]: v * 10}",
            "{a: 10, b: 20}",
        ),
        ("map {} with fn k, v -> {[k]: v}", "{}"),
    ]);
}

#[test]
fn map_over_a_map_requires_the_operation_to_return_a_map() {
    // Each call's result is merged in, so it must itself be a Map entry set.
    let message = raises("map {a: 1} with fn k, v -> v");
    assert!(
        message.contains("must return a Map") && message.contains("Int"),
        "{message}"
    );
}

#[test]
fn reduce_folds_the_elements() {
    assert_values(&[
        ("reduce [1, 2, 3] with fn acc, x -> acc + x", "6"),
        ("reduce [1, 2, 3] init: 10 with fn acc, x -> acc + x", "16"),
        (
            r#"reduce ["a", "b"] init: "" with fn acc, x -> acc + x"#,
            r#""ab""#,
        ),
        (
            "reduce {a: 1, b: 2} init: 0 with fn acc, k, v -> acc + v",
            "3",
        ),
    ]);
}

#[test]
fn reduce_of_nothing_is_its_init_or_null() {
    assert_values(&[
        ("reduce [] init: 5 with fn acc, x -> acc + x", "5"),
        ("reduce [] with fn acc, x -> acc + x", "null"),
        ("reduce {} init: 5 with fn acc, k, v -> acc + v", "5"),
    ]);
}

#[test]
fn reduce_over_a_map_always_requires_an_init() {
    // Unlike an Array, a Map has no intrinsic "first element" to seed the
    // accumulator with, so folding one without `init:` is an error.
    assert_raises(&[
        (
            "reduce {a: 1} with fn acc, k, v -> acc + v",
            "requires an initializer",
        ),
        (
            "reduce {} with fn acc, k, v -> acc + v",
            "requires an initializer",
        ),
    ]);
}

#[test]
fn foreach_runs_its_operation_on_each_element() {
    assert_values(&[
        (
            "def c = mutable_cell(0); foreach [1, 2, 3] with fn x -> c.exchange(c.get() + x); c.get()",
            "6",
        ),
        (
            "def c = mutable_cell(0); foreach {a: 1, b: 2} with fn k, v -> c.exchange(c.get() + v); c.get()",
            "3",
        ),
    ]);
}

#[test]
fn foreach_yields_its_structure() {
    assert_values(&[
        ("foreach [1, 2] with fn x -> x * 100", "[1, 2]"),
        ("foreach {a: 1} with fn k, v -> null", "{a: 1}"),
        ("foreach [] with fn x -> x", "[]"),
        ("foreach {} with fn k, v -> null", "{}"),
    ]);
}

// --- The forms are calls to their globals ---

#[test]
fn each_form_equals_its_direct_call() {
    for (form, call) in [
        (
            "filter [1, 2, 3] with fn x -> x != 2",
            "select([1, 2, 3], fn x -> x != 2)",
        ),
        (
            "map [1, 2, 3] with fn x -> x + 1",
            "transform([1, 2, 3], fn x -> x + 1)",
        ),
        (
            "reduce [1, 2, 3] with fn a, x -> a * x",
            "fold([1, 2, 3], fn a, x -> a * x)",
        ),
        (
            "reduce [1, 2, 3] init: 4 with fn a, x -> a * x",
            "fold([1, 2, 3], fn a, x -> a * x, 4)",
        ),
        (
            "foreach [1, 2, 3] with fn x -> x",
            "each([1, 2, 3], fn x -> x)",
        ),
    ] {
        assert_eq!(run(form), run(call), "{form:?} is {call:?}");
    }
}

#[test]
fn a_binding_named_like_the_global_does_not_change_the_form() {
    assert_values(&[
        ("def select = 5; filter [1, 2] with fn x -> x > 1", "[2]"),
        ("def transform = 5; map [1, 2] with fn x -> x + 1", "[2, 3]"),
        ("def fold = 5; reduce [1, 2] with fn a, x -> a + x", "3"),
        ("def each = 5; foreach [1, 2] with fn x -> x", "[1, 2]"),
        (
            "(fn transform -> map [1] with fn x -> x + transform)(10)",
            "[11]",
        ),
    ]);
}

#[test]
fn the_operation_may_be_any_function() {
    assert_values(&[
        ("map [1, 2] with $($ + 1)", "[2, 3]"),
        ("map [1, 2] with to_string", r#"["1", "2"]"#),
        (
            "def double = fn x -> x * 2; map [1, 2] with double",
            "[2, 4]",
        ),
        ("def k = 3; map [1, 2] with fn x -> x * k", "[3, 6]"),
        ("reduce [1, 2, 3] with plus", "6"),
        ("filter [1, null, 2] with is_int", "[1, 2]"),
    ]);
}

#[test]
fn forms_compose() {
    assert_values(&[
        (
            "map (filter [1, 2, 3, 4] with fn x -> x > 2) with fn x -> x * 10",
            "[30, 40]",
        ),
        ("reduce (map [1, 2, 3] with fn x -> x * x) with plus", "14"),
        (
            "map [[1, 2], [3]] with fn row -> reduce row init: 0 with plus",
            "[3, 3]",
        ),
        ("def xs = map [1, 2] with fn x -> x + 1; xs[1]", "3"),
    ]);
}

// --- Evaluation and errors ---

#[test]
fn operands_are_evaluated_structure_then_operation_then_init() {
    assert_raises(&[
        ("map (1 / 0) with (1 % 0)", "Division by zero"),
        ("filter (1 / 0) with (1 % 0)", "Division by zero"),
        ("foreach (1 / 0) with (1 % 0)", "Division by zero"),
        (
            "reduce (1 / 0) init: (1 % 0) with (null + true)",
            "Division by zero",
        ),
        // The operation comes before `init`, though written after it.
        ("reduce [1] init: (1 % 0) with (null + true)", "Null + Bool"),
    ]);
}

#[test]
fn a_bad_operand_raises_the_globals_type_error() {
    assert_raises(&[
        ("map 5 with fn x -> x", "argument 1"),
        ("map [1] with 5", "argument 2"),
        ("filter null with fn x -> x", "argument 1"),
        ("reduce [1] with 5", "argument 2"),
        ("foreach 5 with fn x -> x", "argument 1"),
    ]);
}

#[test]
fn an_error_in_the_operation_propagates() {
    assert_raises(&[
        ("map [1, 0] with fn x -> 1 / x", "Division by zero"),
        (r#"foreach [1] with fn x -> error("boom")"#, "boom"),
    ]);
}

// --- What the compiler emits ---

#[test]
fn a_constant_form_folds_away() {
    for source in [
        "filter [1, 2, 3] with fn x -> x > 1",
        "map [1, 2, 3] with fn x -> x * 2",
        "reduce [1, 2, 3] init: 0 with fn a, x -> a + x",
        "foreach [1, 2] with fn x -> x",
    ] {
        let emitted = code(source, FOLD);
        assert_eq!(calls(&emitted), 0, "{source:?} folds away: {emitted:?}");
    }
}

#[test]
fn a_runtime_structure_keeps_the_call() {
    for source in [
        "map xs with fn x -> x * 2",
        "filter xs with fn x -> x > 1",
        "reduce xs init: 0 with fn a, x -> a + x",
        "foreach xs with fn x -> x",
    ] {
        let emitted = code(source, FOLD);
        assert_eq!(calls(&emitted), 1, "{source:?}: {emitted:?}");
    }
}

#[test]
fn an_effectful_operation_keeps_the_call() {
    let emitted = code("map [1, 2] with fn x -> if false: print(x) else: x", FOLD);
    assert_eq!(calls(&emitted), 1, "{emitted:?}");
}

#[test]
fn a_form_in_tail_position_is_a_tail_call() {
    let body = code("fn ys -> map ys with fn x -> x", UNOPTIMIZED).nested(0);
    assert_eq!(body.count(&Bytecode::TailCall(2)), 1, "{body:?}");
    let body = code("fn ys -> [map ys with fn x -> x]", UNOPTIMIZED).nested(0);
    assert_eq!(body.count(&Bytecode::Call(2)), 1, "{body:?}");
    assert_eq!(body.count(&Bytecode::TailCall(2)), 0, "{body:?}");
}
