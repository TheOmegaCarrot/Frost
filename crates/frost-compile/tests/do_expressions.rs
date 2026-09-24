//! `do` block lowering, end to end: compile full source, run it on the VM, and
//! check the result.
//!
//! A `do` block runs its statements in order, then yields its final expression.
//! It is its own scope: its bindings are visible only inside it, and may shadow
//! enclosing ones. The harness runs every behavioral case under every
//! optimization permutation.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, compile_errors, raises, run};
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

/// The code of `source`, with `x` a runtime-only capture, under exactly
/// `optimization`.
fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    Script::new(source)
        .capture("x", Value::Int(10))
        .code(optimization)
}

/// How many local definitions (`DefLocal`s) remain in the code.
fn definitions(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::DefLocal(_)))
        .count()
}

// --- Evaluation ---

#[test]
fn a_block_yields_its_final_expression() {
    assert_eq!(run("do { 5 }"), Value::Int(5));
    assert_eq!(run("do { 1; 2; 3 }"), Value::Int(3));
    assert_eq!(run("do { def y = 2; y * 3 }"), Value::Int(6));
    assert_eq!(
        run("do { def a = 2; def b = a + 1; a * b }"),
        Value::Int(6),
        "a binding may use an earlier one"
    );
}

#[test]
fn statements_run_in_order_before_the_final_expression() {
    // Each raising statement shows it ran, and ran before anything after it.
    let message = raises("do { 1 / 0; 1 % 0; 5 }");
    assert!(
        message.contains("Division by zero"),
        "the first statement raises first: {message}"
    );
    let message = raises("do { def y = 1 % 0; 1 / 0 }");
    assert!(
        message.contains("Modulus by zero"),
        "a def runs before the tail: {message}"
    );
}

#[test]
fn a_block_reads_the_enclosing_scope() {
    let tail = Script::new("do { def y = x + 1; y * 2 }")
        .capture("x", Value::Int(4))
        .run();
    assert_eq!(tail, Value::Int(10));
    assert_eq!(run("def a = 3; do { a + 1 }"), Value::Int(4));
}

#[test]
fn a_block_is_an_expression() {
    assert_eq!(run("do { 2 } + do { 3 }"), Value::Int(5));
    assert_eq!(
        run("def y = do { def z = 4; z + 1 }; y * 2"),
        Value::Int(10)
    );
    assert_eq!(
        run("if do { true }: do { 1 } else: do { 2 }"),
        Value::Int(1)
    );
}

#[test]
fn blocks_nest() {
    assert_eq!(
        run("do { def a = 1; do { def b = a + 1; b * 10 } }"),
        Value::Int(20)
    );
}

#[test]
fn a_block_statement_leaves_the_stack_balanced() {
    assert_eq!(run("do { 1 }; do { def y = 2; y }; 5"), Value::Int(5));
    let tail = Script::new("do { def y = x; y }; do { x; x }; 5")
        .capture("x", Value::Int(1))
        .run();
    assert_eq!(tail, Value::Int(5));
}

// --- Scope ---

#[test]
fn a_block_binding_is_not_visible_after_the_block() {
    let rendered = compile_errors("do { def z = 1; z }; z").render_plain();
    assert!(rendered.contains("`z` is not defined"), "{rendered}");
}

#[test]
fn a_block_binding_shadows_an_enclosing_one() {
    assert_eq!(run("def y = 1; do { def y = 2; y }"), Value::Int(2));
    assert_eq!(
        run("def y = 1; do { def y = 2; y } + y"),
        Value::Int(3),
        "the enclosing `y` is untouched once the block ends"
    );
    let tail = Script::new("do { def x = x + 1; x }")
        .capture("x", Value::Int(1))
        .run();
    assert_eq!(tail, Value::Int(2), "a block binding may shadow a capture");
}

#[test]
fn sibling_blocks_may_bind_the_same_name() {
    assert_eq!(
        run("do { def y = 1; y } + do { def y = 2; y }"),
        Value::Int(3)
    );
}

#[test]
fn a_block_rejects_a_duplicate_binding_within_itself() {
    let rendered = compile_errors("do { def y = 1; def y = 2; y }").render_plain();
    assert!(rendered.contains("`y` is already bound"), "{rendered}");
}

#[test]
fn a_block_binding_is_not_implicitly_exported() {
    let finished = Script::new("def top = do { def inner = 1; inner }; top")
        .implicit_export()
        .finish();
    assert_eq!(finished.tail, Value::Int(1));
    assert_eq!(
        finished.exports.keys().collect::<Vec<_>>(),
        vec!["top"],
        "only the top-level binding is exported"
    );
}

// --- Constant folding ---

#[test]
fn a_constant_block_folds_whole() {
    // Every statement is constant, so the block folds, bindings and all.
    for source in ["do { 5 }", "do { def y = 2; 5 }", "do { 1; 2; 5 }"] {
        let emitted = code(source, FOLD);
        assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
        assert_eq!(definitions(&emitted), 0, "no binding survives: {emitted:?}");
    }
}

#[test]
fn a_propagated_block_folds_whole() {
    // With propagation, a lookup of a constant binding is itself constant.
    let emitted = code("do { def y = 2; def z = y * 3; z + 1 }", FOLD_AND_PROPAGATE);
    assert_eq!(emitted.count(&Bytecode::PushInt(7)), 1, "{emitted:?}");
    assert_eq!(definitions(&emitted), 0, "no binding survives: {emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
}

#[test]
fn a_block_reading_its_own_binding_needs_propagation_to_fold() {
    // Without propagation, `y` is a local load, which cannot fold.
    let emitted = code("do { def y = 2; y }", FOLD);
    assert_eq!(definitions(&emitted), 1, "{emitted:?}");
    assert!(
        emitted
            .code
            .iter()
            .any(|op| matches!(op, Bytecode::LoadLocal(_))),
        "{emitted:?}"
    );
}

#[test]
fn a_block_folds_even_in_a_function_with_captures() {
    // The fold evaluates the block alone, without the function's captures; it
    // must not need them.
    let emitted = code("do { def y = 2; 5 } + x", FOLD);
    assert_eq!(definitions(&emitted), 0, "the block folded: {emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::Add),
        1,
        "the runtime Add stays: {emitted:?}"
    );
}

#[test]
fn a_runtime_body_leaves_the_tail_to_fold_alone() {
    // `def y = x` cannot fold, so the block cannot; its constant tail still does.
    let emitted = code("do { def y = x; 1 + 2 }", FOLD);
    assert_eq!(definitions(&emitted), 1, "{emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::Add),
        0,
        "the tail folds: {emitted:?}"
    );
    assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
}

#[test]
fn a_constant_statement_in_a_runtime_block_folds() {
    let emitted = code("do { def y = 2 * 3; x }", FOLD);
    assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(6)), 1, "{emitted:?}");
}

#[test]
fn a_folded_away_binding_takes_no_slot() {
    // The block's `y` disappears with the fold, so `w` gets slot 0.
    let emitted = code("def w = do { def y = 2; 5 }; w", FOLD);
    assert_eq!(emitted.count(&Bytecode::DefLocal(0)), 1, "{emitted:?}");
    assert_eq!(definitions(&emitted), 1, "only `w` is defined: {emitted:?}");
}

#[test]
fn a_raising_block_is_left_for_runtime() {
    let emitted = code("do { def y = 1 / 0; 5 }", FOLD);
    assert_eq!(emitted.count(&Bytecode::Divide), 1, "{emitted:?}");
    assert_eq!(definitions(&emitted), 1, "{emitted:?}");
}

#[test]
fn without_folding_a_constant_block_is_kept() {
    let emitted = code("do { def y = 2; 5 }", UNOPTIMIZED);
    assert_eq!(definitions(&emitted), 1, "{emitted:?}");
}
