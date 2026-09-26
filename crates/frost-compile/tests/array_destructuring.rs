//! Array destructuring, end to end: `def [a, b, ...rest] = value`.
//!
//! The value must be an Array of exactly as many elements as the pattern names,
//! or at least as many when there is a rest, which then binds an Array of the
//! remainder. Parts bind in source order and may nest, as Array or Map
//! patterns, each destructuring completely before the next; `_` discards a part.
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

/// `source` after a prelude defining `note(x)`, which appends `x` to the Array
/// in the cell `log` and returns it, so a script can observe the order in which
/// its expressions run.
fn noting(source: &str) -> String {
    format!(
        "def log = mutable_cell([]); defn note(x) -> {{ log.exchange(log.get() + [x]); x }}; {source}"
    )
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
fn discards_never_collide() {
    assert_values(&[
        ("def [_, _, _] = [1, 2, 3]; 0", "0"),
        ("def [_, [_, _], ..._] = [1, [2, 3], 4, 5]; 0", "0"),
        ("def [_, a, _, b, ..._] = [1, 2, 3, 4]; [a, b]", "[2, 4]"),
        ("def [_] = [1]; def [_] = [2]; 0", "0"),
    ]);
}

#[test]
fn a_discarded_rest_may_appear_at_every_level() {
    assert_values(&[
        ("def [..._] = [1, 2]; 0", "0"),
        ("def [..._] = []; 0", "0"),
        ("def [[a, ..._], ..._] = [[1, 2], 3]; a", "1"),
        ("def [a, [..._]] = [1, [2, 3]]; a", "1"),
    ]);
    // A discarded rest still requires the elements before it.
    assert_raises(&[("def [[a, ..._]] = [[]]; a", "at least 1 element")]);
}

#[test]
fn a_wide_pattern_binds_every_element() {
    assert_values(&[
        (
            "def [a, b, c, d, e, f, g, h, i, j, k, l] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]; [l, k, j, i, h, g, f, e, d, c, b, a]",
            "[12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]",
        ),
        (
            "def [a, b, c, d, e, f, g, h, ...rest] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]; [h, g, f, e, d, c, b, a, rest]",
            "[8, 7, 6, 5, 4, 3, 2, 1, [9, 10]]",
        ),
    ]);
}

#[test]
fn a_trailing_comma_is_allowed() {
    assert_values(&[
        ("def [a, b,] = [1, 2]; [b, a]", "[2, 1]"),
        ("def [a,] = [1]; a", "1"),
        ("def [[a, b,], c,] = [[1, 2], 3]; [a, b, c]", "[1, 2, 3]"),
    ]);
    // A trailing comma adds no element: the pattern still needs exactly two.
    assert_raises(&[("def [a, b,] = [1, 2, 3]; a", "exactly 2 elements")]);
}

#[test]
fn a_pattern_may_span_lines() {
    assert_values(&[
        (
            r"def [
                a, b] = [1, 2]
            [a, b]",
            "[1, 2]",
        ),
        (
            r"def [a,
                b] = [1, 2]
            [a, b]",
            "[1, 2]",
        ),
        (
            r"def [a, b
            ] = [1, 2]
            [a, b]",
            "[1, 2]",
        ),
        (
            r"def [
                a,
                b,
            ] = [1, 2]
            [a, b]",
            "[1, 2]",
        ),
        (
            r"def [
                a,

                b
            ] = [1, 2]
            [a, b]",
            "[1, 2]",
        ),
        (
            r"def [a,
                ...rest
            ] = [1, 2, 3]
            [a, rest]",
            "[1, [2, 3]]",
        ),
        (
            r"def [
            ] = []
            0",
            "0",
        ),
        (
            r"def [
                {x},
                [a,
                 b]
            ] = [{x: 1}, [2, 3]]
            [x, a, b]",
            "[1, 2, 3]",
        ),
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
fn map_patterns_nest_in_array_patterns_at_any_depth() {
    assert_values(&[
        (
            "def [{a: [b, {c: [d, ...e]}]}, ...f] = [{a: [1, {c: [2, 3, 4]}]}, 5]; [b, d, e, f]",
            "[1, 2, [3, 4], [5]]",
        ),
        (
            r#"def [{name} as person, ...rest] = [{name: "n", age: 1}, 2]; [name, person, rest]"#,
            r#"["n", {name: "n", age: 1}, [2]]"#,
        ),
        (
            "def [[{x} as p], [{y}]] = [[{x: 1}], [{y: 2}]]; [x, p, y]",
            "[1, {x: 1}, 2]",
        ),
        ("def [{}, {} as m] = [{a: 1}, {b: 2}]; m", "{b: 2}"),
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
        (r#"def [a, [b, ...c]] = [1, "s"]; a"#, "at least 1 element"),
        ("def [a, {b}] = [1, [2]]; a", "expected a Map"),
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
        // A nested Map pattern takes its turn among the Array ones.
        ("def [{a}, [b]] = [{}, 5]; b", "no value at key 'a'"),
        ("def [[b], {a}] = [5, {}]; b", "exactly 1 element"),
        ("def [{a}, {b}] = [{a: 1}, 5]; b", "expected a Map"),
    ]);
}

#[test]
fn each_part_destructures_completely_before_the_next() {
    // `note` records each computed key as it is evaluated.
    assert_values(&[
        (
            &noting(
                r#"def [{[note("a")]: x}, [{[note("b")]: y}]] = [{a: 1}, [{b: 2}]]; [x, y, log.get()]"#,
            ),
            r#"[1, 2, ["a", "b"]]"#,
        ),
        // The second part's shape fails after the first part's key ran.
        (
            &noting(
                r#"def r = try_call(fn -> { def [{[note("a")]: x}, [y]] = [{a: 1}, 5]; 0 }); [r.ok, log.get()]"#,
            ),
            r#"[false, ["a"]]"#,
        ),
        // The outer shape fails before any part is destructured.
        (
            &noting(
                r#"def r = try_call(fn -> { def [{[note("a")]: x}] = [{a: 1}, 2]; 0 }); [r.ok, log.get()]"#,
            ),
            "[false, []]",
        ),
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

#[test]
fn elements_may_be_closures_and_natives() {
    assert_values(&[
        (
            "def [inc, dec] = do { def n = 1; [fn x -> x + n, fn x -> x - n] }; [inc(5), dec(5)]",
            "[6, 4]",
        ),
        // A named lambda still recurses by its own name once destructured.
        (
            "def [fact] = [fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)]; fact(5)",
            "120",
        ),
        (
            "def [f, ...fs] = [plus, minus, times]; [f(6, 2), fs[0](6, 2), fs[1](6, 2)]",
            "[8, 4, 12]",
        ),
    ]);
}

#[test]
fn the_value_may_be_any_expression() {
    assert_values(&[
        ("def [a, b] = (fn -> [1, 2])(); [b, a]", "[2, 1]"),
        ("def [a] = if true: [1] else: [2]; a", "1"),
        ("def [a, ...r] = [[1, 2, 3], [4]][0]; [a, r]", "[1, [2, 3]]"),
        ("def [h, ...t] = [1, 2] + [3]; [h, t]", "[1, [2, 3]]"),
        ("def [a] = [1, 2] @ select(fn x -> x > 1); a", "2"),
        ("def [a, b] = null or [1, 2]; [a, b]", "[1, 2]"),
    ]);
}

#[test]
fn destructuring_leaves_the_value_intact() {
    // The destructured Array is still whole for every other use of it.
    assert_values(&[
        (
            "def xs = [1, [2, 3]]; def [a, [b, ...c]] = xs; def [d, ...e] = xs; [xs, a, b, c, d, e]",
            "[[1, [2, 3]], 1, 2, [3], 1, [[2, 3]]]",
        ),
        (
            "def xs = [1, 2]; def [a, b] = xs; def [c, d] = xs; [a, b, c, d, xs]",
            "[1, 2, 1, 2, [1, 2]]",
        ),
    ]);
    let tail = Script::new("def [a, ...r] = pair; [pair, a, r]")
        .capture("pair", Value::from_iter([Value::Int(1), Value::Int(2)]))
        .run();
    assert_eq!(tail, run("[[1, 2], 1, [2]]"), "the capture is intact");
}

// --- Scope ---

#[test]
fn the_value_is_evaluated_before_the_names_are_bound() {
    // The rhs `x` is the outer one; the pattern's `x` shadows it only after.
    assert_values(&[("def x = 9; do { def [x, y] = [x, 1]; [x, y] }", "[9, 1]")]);
}

#[test]
fn a_pattern_may_shadow_a_capture_its_value_reads() {
    let tail = Script::new("def [x, y] = x; [y, x]")
        .capture("x", Value::from_iter([Value::Int(1), Value::Int(2)]))
        .run();
    assert_eq!(tail, run("[2, 1]"));
}

#[test]
fn a_pattern_may_shadow_a_global() {
    assert_values(&[
        ("def [type, ...plus] = [1, 2]; [type, plus]", "[1, [2]]"),
        ("def [[to_string]] = [[5]]; to_string + 1", "6"),
    ]);
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
fn discarded_parts_are_not_exported() {
    let finished = Script::new("def [_, a, [_, b], ..._] = [1, 2, [3, 4], 5]; 0")
        .implicit_export()
        .finish();
    assert_eq!(
        finished.exports,
        [("a", run("2")), ("b", run("4"))]
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
        "{finished:?}"
    );
}

#[test]
fn an_exported_pattern_exports_every_named_part() {
    // No implicit export: only the `export def` parts are exported.
    let finished =
        Script::new("export def [a, _, [b, ...c], ..._] = [1, 2, [3, 4], 5]; def [d] = [6]; 0")
            .finish();
    assert_eq!(
        finished.exports,
        [("a", run("1")), ("b", run("3")), ("c", run("[4]"))]
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
        "{finished:?}"
    );
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

#[test]
fn a_name_bound_twice_across_array_and_map_parts_is_a_compile_error() {
    assert_compile_errors(&[
        ("def [{a}, a] = [{a: 1}, 2]", "`a` is already bound"),
        ("def [a, {b} as a] = [1, {b: 2}]", "`a` is already bound"),
        ("def [{b: a}, ...a] = [{b: 1}, 2]", "`a` is already bound"),
        (
            "def [{a} as m, {b} as m] = [{a: 1}, {b: 2}]",
            "`m` is already bound",
        ),
        ("def [a] = [1]; def {a} = {a: 2}", "`a` is already bound"),
    ]);
}

#[test]
fn a_part_may_not_be_read_before_the_pattern_binds_it() {
    // The value is evaluated before any part is bound.
    assert_compile_errors(&[
        ("def [a, b] = [1, a]", "`a` is not defined"),
        ("def [a, ...r] = r", "`r` is not defined"),
    ]);
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
fn a_pattern_with_no_elements_still_requires_an_array() {
    for pattern in ["[]", "[..._]", "[...r]", "[[]]"] {
        for value in ["5", "null", r#""""#, "{}"] {
            let message = raises(&format!("def {pattern} = {value}; 0"));
            assert!(message.contains("Array"), "{pattern} = {value}: {message}");
        }
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
fn a_block_whose_nested_destructuring_fails_is_left_for_runtime() {
    let source = "do { def [a, [b]] = [1, [2, 3]]; 5 }";
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(
        emitted.count(&Bytecode::ProduceError),
        2,
        "both shape checks are kept: {emitted:?}"
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
