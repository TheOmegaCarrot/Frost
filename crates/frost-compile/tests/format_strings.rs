//! Format string lowering, end to end: compile full source, run it on the VM,
//! and check the result.
//!
//! A format string evaluates its interpolations left to right and joins them
//! with its literal text into one String. Each interpolated value is rendered as
//! `to_string` renders it, so these tests compare against `to_string` rather than
//! pin a rendering, which is the runtime's to define. The harness runs every
//! behavioral case under every optimization permutation.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, raises, run};
use frost_compile::OptimizationOptions;
use frost_runtime::{Bytecode, Value};

const FOLD: OptimizationOptions = OptimizationOptions {
    constant_fold: true,
    ..UNOPTIMIZED
};

/// The code of `source` under exactly `optimization`, with `x` a runtime-only
/// Int.
fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    Script::new(source)
        .capture("x", Value::Int(7))
        .code(optimization)
}

fn concats(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::Concat(_)))
        .count()
}

// --- Literal text ---

#[test]
fn literal_text_is_kept_verbatim() {
    assert_eq!(run("$'hello'"), Value::from("hello"));
    assert_eq!(run(r#"$"hello""#), Value::from("hello"), "double quotes");
}

#[test]
fn an_empty_format_string_is_the_empty_string() {
    assert_eq!(run("$''"), Value::from(""));
}

#[test]
fn an_escaped_dollar_is_literal() {
    assert_eq!(
        run(r"$'literal: \${name}'"),
        Value::from("literal: ${name}")
    );
}

#[test]
fn a_bare_dollar_is_literal() {
    assert_eq!(run("$'price: $5'"), Value::from("price: $5"));
}

#[test]
fn a_bare_dollar_at_the_end_of_the_string_is_literal() {
    assert_eq!(run("$'price: $'"), Value::from("price: $"));
}

#[test]
fn an_escaped_dollar_is_literal_right_next_to_a_real_interpolation() {
    let tail = Script::new(r"$'\${x}${x}'")
        .capture("x", Value::Int(7))
        .run();
    assert_eq!(tail, Value::from("${x}7"));
}

#[test]
fn non_ascii_literal_text_passes_through_unchanged() {
    // A non-ASCII character in the literal text is unaffected by interpolation
    // scanning: no escaping, and it can sit right next to a `${...}`.
    let tail = Script::new("$'café ☃${x}日本語'")
        .capture("x", Value::Int(1))
        .run();
    assert_eq!(tail, Value::from("café ☃1日本語"));
}

// --- Interpolation ---

#[test]
fn a_string_is_interpolated_unquoted() {
    let tail = Script::new("$'hello, ${name}!'")
        .capture("name", Value::from("ada"))
        .run();
    assert_eq!(tail, Value::from("hello, ada!"));
}

#[test]
fn an_interpolation_renders_as_to_string_does() {
    for value in [
        "null",
        "true",
        "42",
        "0 - 42",
        "2.5",
        r#""text""#,
        "x'00ff'",
        "[1, \"two\", [3]]",
        "{a: 1, b: \"two\"}",
        "plus",
    ] {
        assert_eq!(
            run(&format!("$'${{{value}}}' == to_string({value})")),
            Value::Bool(true),
            "{value}"
        );
    }
}

#[test]
fn segments_join_in_order() {
    assert_eq!(run("$'${1}-${2}-${3}'"), Value::from("1-2-3"));
    assert_eq!(
        run("$'${1}${2}${3}'"),
        Value::from("123"),
        "adjacent interpolations"
    );
    assert_eq!(run("$'<${1}'"), Value::from("<1"), "leading text");
    assert_eq!(run("$'${1}>'"), Value::from("1>"), "trailing text");
    assert_eq!(run("$'${1}'"), Value::from("1"), "a lone interpolation");
}

#[test]
fn an_interpolation_is_any_expression() {
    let tail =
        Script::new(r#"$'${x + 1} ${if x > 5: "big" else: "small"} ${plus(x, x)} ${[x][0]}'"#)
            .capture("x", Value::Int(7))
            .run();
    assert_eq!(tail, Value::from("8 big 14 7"));
}

#[test]
fn format_strings_nest() {
    assert_eq!(run(r#"$'a${$"b${1}c"}d'"#), Value::from("ab1cd"));
}

#[test]
fn format_strings_nest_deeply() {
    // Three levels: $"c${1}d" is "c1d", $'b${...}e' is "bc1de",
    // and the outer $'a${...}f' is "abc1def".
    assert_eq!(run(r#"$'a${$'b${$"c${1}d"}e'}f'"#), Value::from("abc1def"));
}

#[test]
fn many_segments_join_in_order() {
    let source = (1..=20)
        .map(|n| format!("${{{n}}}"))
        .collect::<Vec<_>>()
        .join("-");
    let expected = (1..=20)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join("-");
    assert_eq!(run(&format!("$'{source}'")), Value::from(expected));
}

#[test]
fn an_interpolated_hard_index_renders_its_field() {
    let tail = Script::new("$'name: ${m.name}'")
        .capture("m", Value::map([("name", Value::from("ada"))]))
        .run();
    assert_eq!(tail, Value::from("name: ada"));
}

#[test]
fn a_format_string_is_a_string() {
    assert_eq!(run("type($'${1}')"), Value::from("String"));
    assert_eq!(run(r#"$'${1}' == "1""#), Value::Bool(true));
    assert_eq!(run(r#"$'a${1}' + "b""#), Value::from("a1b"));
}

// --- Evaluation ---

#[test]
fn interpolations_are_evaluated_left_to_right() {
    let message = raises("$'${1 / 0}${1 % 0}'");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn a_format_string_statement_leaves_the_stack_balanced() {
    let tail = Script::new("$'a${x}b'; $''; $'c'; 5")
        .capture("x", Value::Int(7))
        .run();
    assert_eq!(tail, Value::Int(5));
}

// --- Constant folding ---

#[test]
fn a_constant_format_string_folds_to_one_constant() {
    for source in [
        "$'hello'",
        "$'a${1 + 2}b'",
        "$'${true} ${[1, 2]}'",
        "$'café'",
    ] {
        let emitted = code(source, FOLD);
        assert_eq!(concats(&emitted), 0, "{source:?}: {emitted:?}");
        assert!(
            emitted
                .code
                .iter()
                .any(|op| matches!(op, Bytecode::LoadConst(_))),
            "{source:?}: the String is loaded from the pool: {emitted:?}"
        );
    }
    let emitted = code("$'a${1}b'", UNOPTIMIZED);
    assert_eq!(
        concats(&emitted),
        1,
        "unfolded, the join stays: {emitted:?}"
    );
}

#[test]
fn a_runtime_interpolation_keeps_the_join_and_folds_the_rest() {
    let emitted = code("$'a${x}b${1 + 2}'", FOLD);
    assert_eq!(concats(&emitted), 1, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
}

#[test]
fn a_raising_interpolation_is_left_for_runtime() {
    let emitted = code("$'a${1 / 0}'", FOLD);
    assert_eq!(emitted.count(&Bytecode::Divide), 1, "{emitted:?}");
    assert_eq!(concats(&emitted), 1, "{emitted:?}");
}
