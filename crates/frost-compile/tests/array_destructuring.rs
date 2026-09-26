//! Array destructuring, end to end: `def [a, b, ...rest] = value`.
//!
//! The value must be an Array of exactly as many elements as the pattern names,
//! or at least as many when there is a rest, which then binds an Array of the
//! remainder. Parts bind in source order and may nest; `_` discards a part.
//! Cases were checked against the C++ implementation; error messages are the
//! bytecode compiler's own.
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

fn definitions(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::DefLocal(_)))
        .count()
}

// --- Binding the elements ---

#[test]
fn each_element_binds_in_order() {
    assert_values(&[
        ("def [a, b] = [1, 2]; [b, a]", "[2, 1]"),
        ("def [a] = [1]; a", "1"),
        (
            "def [a, b, c, d] = [1, 2, 3, 4]; [d, c, b, a]",
            "[4, 3, 2, 1]",
        ),
        (r#"def [s, n] = ["x", null]; [n, s]"#, r#"[null, "x"]"#),
        ("def [] = []; 1", "1"),
    ]);
}

#[test]
fn a_rest_binds_the_remaining_elements() {
    assert_values(&[
        ("def [a, ...rest] = [1, 2, 3]; [a, rest]", "[1, [2, 3]]"),
        ("def [a, ...rest] = [1]; rest", "[]"),
        (
            "def [a, b, ...rest] = [1, 2, 3, 4]; [rest, b, a]",
            "[[3, 4], 2, 1]",
        ),
        ("def [...rest] = [1, 2]; rest", "[1, 2]"),
        ("def [...rest] = []; rest", "[]"),
    ]);
}

#[test]
fn a_discard_skips_a_part() {
    assert_values(&[
        ("def [a, _] = [1, 2]; a", "1"),
        ("def [_, b, _] = [1, 2, 3]; b", "2"),
        ("def [a, ..._] = [1, 2, 3]; a", "1"),
        ("def [_, ..._] = [1, 2, 3]; 0", "0"),
    ]);
}

#[test]
fn patterns_nest() {
    assert_values(&[
        ("def [[a, b], c] = [[1, 2], 3]; [a, b, c]", "[1, 2, 3]"),
        (
            "def [a, [b, ...c]] = [1, [2, 3, 4]]; [a, b, c]",
            "[1, 2, [3, 4]]",
        ),
        ("def [[[x]]] = [[[7]]]; x", "7"),
        ("def [[a, _], [_, b]] = [[1, 2], [3, 4]]; [a, b]", "[1, 4]"),
        ("def [a, ...rest] = [[1], [2]]; rest[0][0]", "2"),
    ]);
}

// --- Nesting ---
//
// A part of a pattern may itself be a pattern, to any depth. Each nested pattern
// checks and lays out its own Array, then its parts bind in source order, before
// the next part of the pattern around it.

#[test]
fn sibling_nested_patterns_bind_in_order() {
    assert_values(&[
        (
            "def [[a, b], [c, d]] = [[1, 2], [3, 4]]; [d, c, b, a]",
            "[4, 3, 2, 1]",
        ),
        (
            "def [a, [b, c], d] = [1, [2, 3], 4]; [a, b, c, d]",
            "[1, 2, 3, 4]",
        ),
        ("def [[a], b, [c]] = [[1], 2, [3]]; [a, b, c]", "[1, 2, 3]"),
    ]);
}

#[test]
fn a_rest_may_appear_at_every_level() {
    assert_values(&[
        (
            "def [a, [b, [c, ...d], ...e], ...f] = [1, [2, [3, 4, 5], 6], 7, 8]; [a, b, c, d, e, f]",
            "[1, 2, 3, [4, 5], [6], [7, 8]]",
        ),
        (
            "def [[x, ...xs], ...rest] = [[1, 2], [3], [4]]; [x, xs, rest]",
            "[1, [2], [[3], [4]]]",
        ),
        (
            "def [[...inner], ...outer] = [[], []]; [inner, outer]",
            "[[], [[]]]",
        ),
    ]);
}

#[test]
fn a_nested_pattern_may_be_empty() {
    assert_values(&[
        ("def [[], a] = [[], 1]; a", "1"),
        ("def [[[]]] = [[[]]]; 0", "0"),
    ]);
}

#[test]
fn a_discard_may_skip_a_whole_nested_array() {
    assert_values(&[
        ("def [_, [a]] = [[9, 9], [1]]; a", "1"),
        // The discarded part is not checked for shape.
        ("def [_, [a]] = [5, [1]]; a", "1"),
    ]);
}

#[test]
fn nested_destructuring_repeats_cleanly() {
    // Each call destructures afresh, whatever the lengths.
    assert_values(&[(
        r"defn f(p) -> { def [a, [b, ...c]] = p; [a, b, c] }
        [f([1, [2]]), f([3, [4, 5, 6]]), f([7, [8]])]",
        "[[1, 2, []], [3, 4, [5, 6]], [7, 8, []]]",
    )]);
}

#[test]
fn a_nested_mismatch_raises_at_any_depth() {
    assert_raises(&[
        ("def [a, [b, c]] = [1, 5]; a", "Array"),
        ("def [a, [b, c]] = [1, [2]]; a", "exactly 2 elements"),
        ("def [a, [b, ...c]] = [1, []]; a", "at least 1 element"),
        (
            "def [a, [b, [c]]] = [1, [2, [3, 4]]]; a",
            "exactly 1 element",
        ),
        ("def [a, [b, [c]]] = [1, [2, null]]; a", "Array"),
    ]);
}

#[test]
fn the_outer_shape_is_checked_before_the_inner() {
    // Both levels mismatch; the outer `[_]` needs exactly one element.
    assert_raises(&[("def [[a, b]] = [[1], 2]; a", "exactly 1 element")]);
}

#[test]
fn nested_patterns_are_checked_left_to_right() {
    // Both nested parts mismatch; the first, needing one element, fails first.
    assert_raises(&[
        ("def [[a], [b, c]] = [[1, 2], [3]]; a", "exactly 1 element"),
        ("def [[a, b], [c]] = [[1], [2, 3]]; a", "exactly 2 elements"),
    ]);
}

#[test]
fn a_name_bound_twice_across_nesting_is_a_compile_error() {
    for source in [
        "def [a, [b, [a]]] = [1, [2, [3]]]",
        "def [[a], [a]] = [[1], [2]]",
        "def [[a, ...r], ...r] = [[1], 2]",
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains("is already bound"),
            "{source:?}:\n{rendered}"
        );
    }
}

#[test]
fn elements_may_be_any_value() {
    assert_values(&[
        ("def [f, x] = [fn v -> v * 2, 21]; f(x)", "42"),
        ("def [m, [a]] = [{k: 1}, [2]]; m.k + a", "3"),
    ]);
}

// --- Scope ---

#[test]
fn the_value_is_evaluated_before_the_names_are_bound() {
    // The rhs `x` is the outer one; the pattern's `x` shadows it only after.
    assert_values(&[("def x = 9; do { def [x, y] = [x, 1]; [x, y] }", "[9, 1]")]);
}

#[test]
fn destructuring_works_in_any_scope() {
    assert_values(&[
        ("def f = fn -> { def [a, b] = [1, 2]; a + b }; f()", "3"),
        (
            "defn swap(pair) -> { def [a, b] = pair; [b, a] }; swap([1, 2])",
            "[2, 1]",
        ),
        ("do { def [a, ...r] = [1, 2]; [a, r] }", "[1, [2]]"),
        ("def f = fn xs -> { def [h, ...t] = xs; t }; f([0])", "[]"),
    ]);
}

#[test]
fn every_part_is_implicitly_exported() {
    let finished = Script::new("def [a, [b, ...c]] = [1, [2, 3]]; 0")
        .implicit_export()
        .finish();
    assert_eq!(
        finished.exports.keys().collect::<Vec<_>>(),
        vec!["a", "b", "c"],
        "{finished:?}"
    );
    assert_eq!(finished.exports["c"], run("[3]"));
}

#[test]
fn a_name_bound_twice_in_a_pattern_is_a_compile_error() {
    for source in [
        "def [a, a] = [1, 2]",
        "def [a, ...a] = [1, 2]",
        "def [[a], a] = [[1], 2]",
        "def a = 1; def [a] = [1]",
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains("`a` is already bound"),
            "{source:?}:\n{rendered}"
        );
    }
}

// --- Mismatches ---

#[test]
fn a_wrong_length_raises() {
    assert_raises(&[
        ("def [a, b] = [1]; a", "exactly 2 elements"),
        ("def [a] = [1, 2]; a", "exactly 1 element"),
        ("def [] = [1]; 0", "exactly 0 elements"),
        ("def [a, ...rest] = []; a", "at least 1 element"),
        ("def [a, b, ...rest] = [1]; a", "at least 2 elements"),
        ("def [[a, b], c] = [[1], 3]; a", "exactly 2 elements"),
    ]);
}

#[test]
fn a_non_array_raises() {
    for value in ["5", "null", r#""ab""#, "x'0102'", "{a: 1, b: 2}", "plus"] {
        let message = raises(&format!("def [a, b] = {value}; a"));
        assert!(message.contains("Array"), "{value}: {message}");
        let message = raises(&format!("def [a, ...r] = {value}; a"));
        assert!(message.contains("Array"), "{value}: {message}");
    }
}

#[test]
fn an_error_in_the_value_raises_before_destructuring() {
    assert_raises(&[("def [a, b] = (1 / 0); a", "Division by zero")]);
}

#[test]
fn a_mismatch_stops_the_remaining_bindings() {
    // The failed statement ends the program; nothing after it runs.
    assert_raises(&[(
        "def c = mutable_cell(0); def [a] = [1, 2]; c.exchange(1); c.get()",
        "exactly 1 element",
    )]);
}

// --- What the compiler emits ---

#[test]
fn a_block_destructuring_a_constant_folds_whole() {
    let emitted = Script::new("do { def [a, b] = [1, 2]; 5 }").code(FOLD);
    assert_eq!(definitions(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
}

#[test]
fn a_block_whose_destructuring_fails_is_left_for_runtime() {
    let source = "do { def [a] = [1, 2]; 5 }";
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(
        emitted.count(&Bytecode::ProduceError),
        1,
        "the shape check is kept: {emitted:?}"
    );
    assert_raises(&[(source, "exactly 1 element")]);
}

#[test]
fn only_a_rest_splits_the_array() {
    let split = |emitted: &Emitted| {
        emitted
            .code
            .iter()
            .any(|op| matches!(op, Bytecode::SplitArray(_)))
    };
    let with_rest = Script::new("def [a, ...r] = [1, 2]; 0").code(UNOPTIMIZED);
    assert!(split(&with_rest), "{with_rest:?}");
    let without = Script::new("def [a, b] = [1, 2]; 0").code(UNOPTIMIZED);
    assert!(!split(&without), "{without:?}");
}

#[test]
fn destructured_values_are_the_values_themselves() {
    // Destructuring an Array binds its own elements, whatever their type.
    let tail = Script::new("def [a, b] = pair; [b, a]")
        .capture(
            "pair",
            Value::from_iter([Value::Int(1), Value::from("two")]),
        )
        .run();
    assert_eq!(tail, Value::from_iter([Value::from("two"), Value::Int(1)]));
}
