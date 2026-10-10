//! Unary operator lowering (`-` and `not`), end to end: compile full source, run
//! it on the VM, and check the result.
//!
//! The operators' own semantics are the runtime's; these tests pin that each
//! lowers to the right operation over its operand, including how it composes
//! with the binary and logical operators. The harness runs every behavioral case
//! under every optimization permutation.

use crate::script;

use frostlang::Value;
use frostlang::bytecode::Bytecode;
use frostlang::compile::OptimizationOptions;
use script::{Emitted, Script, raises, run};

fn float(f: f64) -> Value {
    Value::try_from(f).expect("test floats are finite")
}

/// The code of `source`, with `x` a runtime-only capture, under each
/// optimization permutation `select` accepts.
fn emitted(source: &str, select: impl Fn(&OptimizationOptions) -> bool) -> Vec<Emitted> {
    Script::new(source)
        .capture("x", Value::Int(1))
        .code_where(select)
}

fn folding(optimization: &OptimizationOptions) -> bool {
    optimization.constant_fold
}

fn not_folding(optimization: &OptimizationOptions) -> bool {
    !optimization.constant_fold
}

// --- Negation ---

#[test]
fn negate_an_int() {
    assert_eq!(run("-5"), Value::Int(-5));
    assert_eq!(run("-0"), Value::Int(0));
    assert_eq!(run("-(0 - 5)"), Value::Int(5), "a negative operand");
    assert_eq!(
        run("-9223372036854775807"),
        Value::Int(-i64::MAX),
        "the largest writable magnitude"
    );
}

#[test]
fn negate_a_float() {
    assert_eq!(run("-2.5"), float(-2.5));
    assert_eq!(run("-(0.0 - 2.5)"), float(2.5), "a negative operand");
}

#[test]
fn negation_nests() {
    assert_eq!(run("- -5"), Value::Int(5));
    assert_eq!(run("-(-5)"), Value::Int(5));
    assert_eq!(run("- - -5"), Value::Int(-5));
}

#[test]
fn negate_a_runtime_operand() {
    for (x, expected) in [
        (Value::Int(7), Value::Int(-7)),
        (Value::Int(-7), Value::Int(7)),
        (float(1.5), float(-1.5)),
    ] {
        let tail = Script::new("-x").capture("x", x.clone()).run();
        assert_eq!(tail, expected, "x = {x:?}");
    }
}

#[test]
fn negating_a_non_number_raises() {
    for (source, type_name) in [
        (r#"-"a""#, "String"),
        ("-true", "Bool"),
        ("-null", "Null"),
        ("-x'00'", "Bytes"),
        ("-len", "Function"),
        ("-[1]", "Array"),
        ("-{}", "Map"),
    ] {
        let message = raises(source);
        assert!(
            message.contains(type_name),
            "{source:?}: the error names the operand's type: {message}"
        );
    }
}

#[test]
fn an_operand_error_surfaces_before_negation() {
    let message = raises("-(1 / 0)");
    assert!(message.contains("Division by zero"), "{message}");
}

// --- Not ---

#[test]
fn not_inverts_truthiness() {
    // Only `null` and `false` are falsy.
    for falsy in ["null", "false"] {
        assert_eq!(
            run(&format!("not {falsy}")),
            Value::Bool(true),
            "not {falsy}"
        );
    }
    for truthy in ["true", "0", "0.0", r#""""#, "x''", "len", "[]", "{}"] {
        assert_eq!(
            run(&format!("not {truthy}")),
            Value::Bool(false),
            "not {truthy}"
        );
    }
}

#[test]
fn not_always_yields_a_bool() {
    // Unlike `and`/`or`, `not` coerces: `not not` is a truthiness test.
    assert_eq!(run("not not 5"), Value::Bool(true));
    assert_eq!(run("not not null"), Value::Bool(false));
    assert_eq!(run(r#"not not """#), Value::Bool(true));
}

#[test]
fn not_of_a_runtime_operand() {
    for (x, expected) in [
        (Value::Null, true),
        (Value::Bool(false), true),
        (Value::Bool(true), false),
        (Value::Int(0), false),
    ] {
        let tail = Script::new("not x").capture("x", x.clone()).run();
        assert_eq!(tail, Value::Bool(expected), "x = {x:?}");
    }
}

#[test]
fn an_operand_error_surfaces_through_not() {
    let message = raises("not (1 / 0)");
    assert!(message.contains("Division by zero"), "{message}");
}

// --- Composition with other operators ---

#[test]
fn prefix_operators_bind_tightest() {
    assert_eq!(run("-2 * 3"), Value::Int(-6));
    assert_eq!(run("-2 + 3"), Value::Int(1), "(-2) + 3, not -(2 + 3)");
    assert_eq!(run("2 - -3"), Value::Int(5));
    assert_eq!(run("-2 < 1"), Value::Bool(true));
    assert_eq!(
        run("not 1 == false"),
        Value::Bool(true),
        "(not 1) == false, not not (1 == false)"
    );
    assert_eq!(
        run("not true or true"),
        Value::Bool(true),
        "(not true) or true"
    );
    assert_eq!(run("not (true or true)"), Value::Bool(false));
}

#[test]
fn unary_operators_combine() {
    assert_eq!(run("not -1"), Value::Bool(false), "-1 is truthy");
    let message = raises("-not 1");
    assert!(
        message.contains("Bool"),
        "negating the Bool from `not` raises: {message}"
    );
}

#[test]
fn not_guards_with_and_or() {
    // Lua's "if missing, default" shape, via `not`.
    let source = r#"not x and "missing" or x"#;
    for (x, expected) in [
        (Value::Null, Value::from("missing")),
        (Value::Int(3), Value::Int(3)),
    ] {
        let tail = Script::new(source).capture("x", x.clone()).run();
        assert_eq!(tail, expected, "x = {x:?}");
    }
}

// --- Constant folding ---

#[test]
fn a_constant_negation_folds_to_its_value() {
    for emitted in emitted("-5", folding) {
        assert_eq!(
            emitted.count(&Bytecode::Negate),
            0,
            "folded away: {emitted:?}"
        );
        assert_eq!(emitted.count(&Bytecode::PushInt(-5)), 1, "{emitted:?}");
    }
    for emitted in emitted("-5", not_folding) {
        assert_eq!(emitted.count(&Bytecode::Negate), 1, "{emitted:?}");
    }
}

#[test]
fn a_constant_not_folds_to_its_value() {
    for (source, value) in [
        ("not true", Bytecode::PushFalse),
        ("not null", Bytecode::PushTrue),
        ("not (1 == 1)", Bytecode::PushFalse),
        ("not []", Bytecode::PushFalse),
        ("not {}", Bytecode::PushFalse),
    ] {
        for emitted in emitted(source, folding) {
            assert_eq!(
                emitted.count(&Bytecode::LogicalNot),
                0,
                "folded away: {emitted:?}"
            );
            assert_eq!(emitted.count(&value), 1, "{emitted:?}");
        }
    }
}

#[test]
fn a_runtime_operand_keeps_the_operation() {
    for (source, op) in [("-x", Bytecode::Negate), ("not x", Bytecode::LogicalNot)] {
        for emitted in emitted(source, |_| true) {
            assert_eq!(emitted.count(&op), 1, "{emitted:?}");
        }
    }
    for emitted in emitted("- -x", |_| true) {
        assert_eq!(emitted.count(&Bytecode::Negate), 2, "{emitted:?}");
    }
}

#[test]
fn a_negated_constant_folds_as_a_sibling() {
    // `-(2 * 3)` is a constant beside the runtime `x`, so it folds whole.
    for emitted in emitted("-(2 * 3) + x", folding) {
        assert_eq!(emitted.count(&Bytecode::Negate), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(-6)), 1, "{emitted:?}");
        assert_eq!(
            emitted.count(&Bytecode::Add),
            1,
            "the runtime Add stays: {emitted:?}"
        );
    }
}

#[test]
fn a_constant_inside_a_runtime_negation_folds() {
    for emitted in emitted("-(x + (2 * 3))", folding) {
        assert_eq!(emitted.count(&Bytecode::Negate), 1, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(6)), 1, "{emitted:?}");
    }
}

#[test]
fn a_failing_fold_is_left_for_runtime() {
    for (source, op) in [
        (r#"-"a""#, Bytecode::Negate),
        ("-true", Bytecode::Negate),
        ("-[1]", Bytecode::Negate),
        ("not (1 / 0)", Bytecode::Divide),
    ] {
        for emitted in emitted(source, folding) {
            assert_eq!(
                emitted.count(&op),
                1,
                "the failing operation is kept: {emitted:?}"
            );
        }
    }
}

#[test]
fn a_folded_not_decides_a_logical() {
    // `not true` folds as a sibling of the runtime `x`, becoming a constant that
    // branch elimination can decide on.
    let emitted = emitted("not true and x", |optimization| {
        optimization.constant_fold && optimization.branch_eliminate
    });
    for emitted in emitted {
        assert!(
            !emitted
                .code
                .iter()
                .any(|op| matches!(op, Bytecode::PeekJumpIfFalse(_) | Bytecode::LoadLocal(_))),
            "the test and `x` are both gone: {emitted:?}"
        );
        assert_eq!(emitted.count(&Bytecode::PushFalse), 1, "{emitted:?}");
    }
}
