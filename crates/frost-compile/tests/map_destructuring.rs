//! Map destructuring, end to end: `def {key, key: name, [expr]: name} as whole = value`.
//!
//! The value must be a Map, and must hold every key the pattern names; other keys
//! are allowed. A key is a name (shorthand for the String key and a binding of
//! it), a name mapped to a part, or a computed expression; `as` binds the whole
//! Map. Parts bind in source order and may nest, and a computed key may read the
//! bindings before it. Cases were checked against the C++ implementation; error
//! messages are the bytecode compiler's own.
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

// --- Binding the entries ---

#[test]
fn a_shorthand_key_binds_its_own_name() {
    assert_values(&[
        ("def {a, b} = {a: 1, b: 2}; [b, a]", "[2, 1]"),
        ("def {a} = {a: [1, 2]}; a", "[1, 2]"),
    ]);
}

#[test]
fn a_key_may_bind_another_name() {
    assert_values(&[
        ("def {a: x, b: y} = {a: 1, b: 2}; [y, x]", "[2, 1]"),
        ("def {a: x, b} = {a: 1, b: 2}; [x, b]", "[1, 2]"),
    ]);
}

#[test]
fn a_computed_key_may_be_any_valid_key() {
    assert_values(&[
        (
            r#"def {[1]: one, [true]: t, [x'00']: b, [1.5]: f} = {[1]: "i", [true]: "t", [x'00']: "b", [1.5]: "f"}; [one, t, b, f]"#,
            r#"["i", "t", "b", "f"]"#,
        ),
        (r#"def {["with space"]: v} = {["with space"]: 1}; v"#, "1"),
        ("def k = \"b\"; def {[k]: v} = {b: 5}; v", "5"),
        ("def {[1 + 1]: v} = {[2]: \"two\"}; v", r#""two""#),
    ]);
}

#[test]
fn keys_may_be_left_out_or_taken_twice() {
    assert_values(&[
        ("def {a} = {a: 1, b: 2, c: 3}; a", "1"),
        ("def {a, a: b} = {a: 1}; [a, b]", "[1, 1]"),
        ("def {} = {a: 1}; 0", "0"),
        ("def {} = {}; 0", "0"),
    ]);
}

#[test]
fn a_discard_still_requires_its_key() {
    assert_values(&[("def {a: _, b} = {a: 1, b: 2}; b", "2")]);
    assert_raises(&[("def {a: _} = {}; 0", "no value at key 'a'")]);
}

#[test]
fn as_binds_the_whole_map() {
    assert_values(&[
        ("def {a} as m = {a: 1, b: 2}; [a, m]", "[1, {a: 1, b: 2}]"),
        ("def {} as m = {a: 1}; m", "{a: 1}"),
        ("def {a: _} as m = {a: 1}; m", "{a: 1}"),
    ]);
}

// --- Scope ---

#[test]
fn a_computed_key_may_read_an_earlier_binding() {
    // `[a]` is the value just bound to `a`, used as a key.
    assert_values(&[(r#"def {a, [a]: b} = {a: "c", c: 3}; b"#, "3")]);
}

#[test]
fn the_value_is_evaluated_before_the_names_are_bound() {
    assert_values(&[(
        "def x = 9; do { def {x, y} = {x: x + 1, y: x}; [x, y] }",
        "[10, 9]",
    )]);
}

#[test]
fn destructuring_works_in_any_scope() {
    assert_values(&[
        (
            "defn area(r) -> { def {w, h} = r; w * h }; area({w: 3, h: 4})",
            "12",
        ),
        ("do { def {a} as m = {a: 1}; [a, m] }", "[1, {a: 1}]"),
    ]);
}

#[test]
fn every_part_is_implicitly_exported() {
    let finished = Script::new("def {a, b: [c]} as m = {a: 1, b: [2]}; 0")
        .implicit_export()
        .finish();
    assert_eq!(
        finished.exports.keys().collect::<Vec<_>>(),
        vec!["a", "c", "m"],
        "{finished:?}"
    );
}

#[test]
fn a_name_bound_twice_in_a_pattern_is_a_compile_error() {
    for source in [
        "def {a, a} = {a: 1}",
        "def {a} as a = {a: 1}",
        "def {a, b: a} = {a: 1, b: 2}",
        "def {a, b: [a]} = {a: 1, b: [2]}",
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains("`a` is already bound"),
            "{source:?}:\n{rendered}"
        );
    }
}

#[test]
fn only_a_shorthand_key_binds_its_name() {
    // `a: [a]` names the key `a`, but binds only its part's `a`.
    assert_values(&[("def {a: [a]} = {a: [1]}; a", "1")]);
}

// --- Nesting ---

#[test]
fn map_and_array_patterns_nest_in_each_other() {
    assert_values(&[
        (
            "def {a: [x, y], b: {c}} = {a: [1, 2], b: {c: 3}}; [x, y, c]",
            "[1, 2, 3]",
        ),
        ("def [{a}, {b}] = [{a: 1}, {b: 2}]; [a, b]", "[1, 2]"),
        (
            "def {pts: [{x}, ...rest]} as all = {pts: [{x: 1}, {x: 2}]}; [x, rest, all.pts[1].x]",
            "[1, [{x: 2}], 2]",
        ),
        ("def {a: {b: {c}}} = {a: {b: {c: 7}}}; c", "7"),
    ]);
}

// --- Mismatches ---

#[test]
fn a_missing_key_raises() {
    assert_raises(&[
        ("def {a} = {b: 1}; a", "no value at key 'a'"),
        ("def {x, y} = {y: 1}; x", "no value at key 'x'"),
        ("def {a, b} = {}; a", "no value at key 'a'"),
        ("def {a: {b}} = {a: {c: 1}}; b", "no value at key 'b'"),
    ]);
}

#[test]
fn keys_do_not_cross_numeric_types() {
    assert_raises(&[
        ("def {[1]: x} = {[1.0]: 2}; x", "no value at key"),
        ("def {[1.0]: x} = {[1]: 2}; x", "no value at key"),
    ]);
}

#[test]
fn entries_are_looked_up_in_source_order() {
    // Both keys are missing; the first is reported.
    assert_raises(&[
        ("def {first, second} = {}; 0", "'first'"),
        ("def {second, first} = {}; 0", "'second'"),
    ]);
}

#[test]
fn a_non_map_raises() {
    for value in ["5", "null", r#""ab""#, "[1]", "x'00'", "plus"] {
        for pattern in ["{a}", "{}", "{} as m", "{[1]: x}"] {
            let message = raises(&format!("def {pattern} = {value}; 0"));
            assert!(message.contains("Map"), "{pattern} = {value}: {message}");
        }
    }
}

#[test]
fn an_invalid_computed_key_raises() {
    assert_raises(&[
        ("def {[null]: x} = {a: 1}; x", "not a valid Map key"),
        ("def {[[1]]: x} = {a: 1}; x", "not a valid Map key"),
    ]);
}

#[test]
fn errors_come_from_the_value_first_then_each_key_in_turn() {
    assert_raises(&[
        ("def {a} = (1 / 0); a", "Division by zero"),
        ("def {[1 / 0]: x} = {a: 1}; x", "Division by zero"),
        // The first entry's lookup fails before the second key is evaluated.
        (
            "def {missing, [1 / 0]: x} = {a: 1}; x",
            "no value at key 'missing'",
        ),
    ]);
}

// --- What the compiler emits ---

#[test]
fn a_block_destructuring_a_constant_folds_whole() {
    let emitted = Script::new("do { def {a} as m = {a: 1}; 5 }").code(FOLD);
    assert_eq!(definitions(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
}

#[test]
fn a_computed_key_folds_even_when_the_value_is_runtime() {
    let emitted = Script::new("def {[1 + 1]: v} = m; v")
        .capture(
            "m",
            Value::map([(frost_runtime::MapKey::Int(2), Value::Int(9))]),
        )
        .code(FOLD);
    assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 1, "{emitted:?}");
}

#[test]
fn a_block_whose_destructuring_fails_is_left_for_runtime() {
    let source = "do { def {a} = {b: 1}; 5 }";
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(
        emitted.count(&Bytecode::ExtractKey),
        1,
        "the lookup is kept: {emitted:?}"
    );
    assert_raises(&[(source, "no value at key 'a'")]);
}
