//! `do` block lowering, end to end: compile full source, run it on the VM, and
//! check the result.
//!
//! A `do` block runs its statements in order, then yields its final expression.
//! It is its own scope: its bindings are visible only inside it, and may shadow
//! enclosing ones. The harness runs every behavioral case under every
//! optimization permutation; code-shape cases pin exactly the options they are
//! about.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, compile_errors, raises, run};
use frost_compile::{Optimization, OptimizationOptions};
use frost_runtime::{Bytecode, Value};

const FOLD: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantFold, true);

const FOLD_AND_PROPAGATE: OptimizationOptions = FOLD.with(Optimization::ConstantPropagate, true);

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
    assert_eq!(
        run(r"
            do {
                1
                2
                3
            }
            "),
        Value::Int(3)
    );
    assert_eq!(run("do { def y = 2; y * 3 }"), Value::Int(6));
    assert_eq!(
        run(r"
            do {
                def a = 2
                def b = a + 1
                a * b
            }
            "),
        Value::Int(6),
        "a binding may use an earlier one"
    );
}

#[test]
fn statements_run_in_order_before_the_final_expression() {
    // Each raising statement shows it ran, and ran before anything after it.
    let message = raises(
        r"
        do {
            1 / 0
            1 % 0
            5
        }
        ",
    );
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
    assert_eq!(
        run(r"
            def a = 3
            do { a + 1 }
            "),
        Value::Int(4)
    );
}

#[test]
fn a_block_is_an_expression() {
    assert_eq!(run("do { 2 } + do { 3 }"), Value::Int(5));
    assert_eq!(
        run(r"
            def y = do { def z = 4; z + 1 }
            y * 2
            "),
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
        run(r"
            do {
                def a = 1
                do {
                    def b = a + 1
                    b * 10
                }
            }
            "),
        Value::Int(20)
    );
}

#[test]
fn a_block_statement_leaves_the_stack_balanced() {
    assert_eq!(
        run(r"
            do { 1 }
            do { def y = 2; y }
            5
            "),
        Value::Int(5)
    );
    let source = r"
        do { def y = x; y }
        do { x; x }
        5
        ";
    let tail = Script::new(source).capture("x", Value::Int(1)).run();
    assert_eq!(tail, Value::Int(5));
}

/// Defines `note(v)`, which appends `v` to a log and returns it, and `log()`,
/// the values noted so far, in order. A script appends its own statements.
const NOTE: &str = r"
def cell = mutable_cell([])
defn note(v) -> {
    cell.exchange(cell.get() + [v])
    v
}
defn log() -> cell.get()
";

#[test]
fn every_statement_runs_once_in_order() {
    let source = format!(
        r"
        {NOTE}
        do {{
            note(1)
            note(2)
            def a = note(3)
            note(a + 1)
        }}
        log()
        "
    );
    assert_eq!(run(&source), run("[1, 2, 3, 4]"));
    // A block bound to a name runs once, not at each use of the name.
    let source = format!(
        r"
        {NOTE}
        def y = do {{ note(1) }}
        [y, y, log()]
        "
    );
    assert_eq!(run(&source), run("[1, 1, [1]]"));
    // A nested block's statements run where the block is reached.
    let source = format!(
        r"
        {NOTE}
        do {{
            note(1)
            do {{
                note(2)
                note(3)
            }}
            note(4)
            do {{ note(5) }}
        }}
        log()
        "
    );
    assert_eq!(run(&source), run("[1, 2, 3, 4, 5]"));
}

#[test]
fn a_block_may_yield_any_value() {
    assert_eq!(run("do { null }"), Value::Null);
    assert_eq!(run("do { def y = 1; [y, {y: y}] }"), run("[1, {y: 1}]"));
    assert_eq!(
        run("(do { def y = 1; fn -> y })()"),
        Value::Int(1),
        "a Function closing over the block's binding"
    );
}

#[test]
fn a_block_in_a_lambda_runs_afresh_on_each_call() {
    assert_eq!(
        run(r"
            def f = fn n -> do {
                def m = n * 2
                m + 1
            }
            [f(1), f(2)]
            "),
        run("[3, 5]")
    );
}

#[test]
fn a_block_leaves_exactly_its_value_among_others() {
    for (source, expected) in [
        ("[1, do { def a = x; def b = a + 1; b }, 3]", "[1, 11, 3]"),
        (
            r"
            do {
                do { def a = 1; a }
                do { def b = x; b }
                5
            }
            ",
            "5",
        ),
        (
            r"
            do {
                if x: 1 else: 2
                x and 3
                x or 4
                5
            }
            ",
            "5",
        ),
        ("10 * do { def a = x; do { def b = a; b } - 9 }", "10"),
        (
            r#"{[do { def k = "a"; k }]: do { def v = x; v }}"#,
            "{a: 10}",
        ),
    ] {
        let tail = Script::new(source).capture("x", Value::Int(10)).run();
        assert_eq!(tail, run(expected), "{source:?}");
    }
}

// --- Scope ---

#[test]
fn a_nested_block_binding_is_not_visible_in_the_enclosing_block() {
    let rendered = compile_errors("do { do { def z = 1; z }; z }").render_plain();
    assert!(rendered.contains("`z` is not defined"), "{rendered}");
}

#[test]
fn bindings_resolve_through_several_enclosing_blocks() {
    // The innermost `a` shadows the top-level one only inside its own block.
    assert_eq!(
        run(r"
            def a = 1
            do {
                def b = a + 1
                do { def a = 10; a + b } + a
            }
            "),
        Value::Int(13)
    );
}

#[test]
fn a_use_before_a_shadowing_definition_reads_the_enclosing_name() {
    assert_eq!(
        run(r"
            def y = 1
            do {
                def z = y
                def y = 2
                z + y
            }
            "),
        Value::Int(3)
    );
}

#[test]
fn a_name_used_before_its_definition_in_the_block_is_unbound() {
    for (source, name) in [
        (
            r"
            do {
                def z = w
                def w = 1
                z
            }
            ",
            "w",
        ),
        ("do { def q = q; q }", "q"),
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains(&format!("`{name}` is not defined")),
            "{source:?}: {rendered}"
        );
    }
}

#[test]
fn a_block_binding_may_shadow_a_global() {
    assert_eq!(
        run("[do { def type = 5; type + 1 }, type(1)]"),
        run(r#"[6, "Int"]"#),
        "the global is back in view after the block"
    );
}

#[test]
fn a_block_may_not_export() {
    let rendered = compile_errors("do { export def y = 1; y }").render_plain();
    assert!(rendered.contains("unexpected export"), "{rendered}");
}

#[test]
fn a_block_binding_is_not_visible_after_the_block() {
    let rendered = compile_errors(
        r"
        do { def z = 1; z }
        z
        ",
    )
    .render_plain();
    assert!(rendered.contains("`z` is not defined"), "{rendered}");
}

#[test]
fn a_block_binding_shadows_an_enclosing_one() {
    assert_eq!(
        run(r"
            def y = 1
            do { def y = 2; y }
            "),
        Value::Int(2)
    );
    assert_eq!(
        run(r"
            def y = 1
            do { def y = 2; y } + y
            "),
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
    let rendered = compile_errors(
        r"
        do {
            def y = 1
            def y = 2
            y
        }
        ",
    )
    .render_plain();
    assert!(rendered.contains("`y` is already bound"), "{rendered}");
}

#[test]
fn a_block_binding_is_not_implicitly_exported() {
    let source = r"
        def top = do { def inner = 1; inner }
        top
        ";
    let finished = Script::new(source).implicit_export().finish();
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
    for source in [
        "do { 5 }",
        "do { def y = 2; 5 }",
        r"
        do {
            1
            2
            5
        }
        ",
    ] {
        let emitted = code(source, FOLD);
        assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
        assert_eq!(definitions(&emitted), 0, "no binding survives: {emitted:?}");
    }
}

#[test]
fn a_propagated_block_folds_whole() {
    // With propagation, a lookup of a constant binding is itself constant.
    let source = r"
        do {
            def y = 2
            def z = y * 3
            z + 1
        }
        ";
    let emitted = code(source, FOLD_AND_PROPAGATE);
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
    let source = r"
        def w = do { def y = 2; 5 }
        w
        ";
    let emitted = code(source, FOLD);
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
fn a_block_binding_a_lambda_folds_whole() {
    // Creating a pure lambda is safe in a fold; the block's value is constant.
    let emitted = code("do { def g = fn a -> a + 1; 5 }", FOLD);
    assert_eq!(definitions(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
}

#[test]
fn an_effectful_binding_keeps_the_block() {
    // The lambda would call `print`, so creating it keeps the block from folding,
    // even though it is never called.
    let source = r"
        do {
            def g = fn -> print(1)
            5
        }
        ";
    assert_eq!(run(source), Value::Int(5));
    let emitted = code(source, FOLD);
    assert_eq!(definitions(&emitted), 1, "{emitted:?}");
}

#[test]
fn nested_constant_blocks_fold_whole() {
    let source = r"
        do {
            def a = 1
            do { def a = 2; 5 }
        }
        ";
    let emitted = code(source, FOLD);
    assert_eq!(definitions(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
}

#[test]
fn a_constant_inner_block_folds_inside_a_runtime_one() {
    // As the tail and as an operand, the inner block folds on its own; the outer
    // block's runtime binding stays.
    for source in [
        r"
        do {
            def a = x
            do { def b = 2; 5 }
        }
        ",
        r"
        do {
            def a = x
            do { def b = 2; 5 } + a
        }
        ",
    ] {
        let emitted = code(source, FOLD);
        assert_eq!(definitions(&emitted), 1, "only `a`: {emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(2)), 0, "{emitted:?}");
    }
    let source = r"
        do {
            def a = x
            do { def b = 2; 5 } + a
        }
        ";
    assert_eq!(
        Script::new(source).capture("x", Value::Int(10)).run(),
        Value::Int(15)
    );
}

#[test]
fn a_folded_block_may_shadow_a_runtime_binding() {
    let source = r"
        def y = x
        do { def y = 2; 5 } + y
        ";
    let tail = Script::new(source).capture("x", Value::Int(10)).run();
    assert_eq!(tail, Value::Int(15));
    let emitted = code(source, FOLD);
    assert_eq!(definitions(&emitted), 1, "only the outer `y`: {emitted:?}");
}

#[test]
fn a_block_folds_inside_a_lambda() {
    let source = "fn n -> do { def y = 2; 5 } + n";
    assert_eq!(run(&format!("({source})(1)")), Value::Int(6));
    let body = code(source, FOLD).nested(0);
    assert_eq!(body.count(&Bytecode::PushInt(5)), 1, "{body:?}");
    assert_eq!(
        body.count(&Bytecode::PushInt(2)),
        0,
        "the block's binding is gone: {body:?}"
    );
}

#[test]
fn propagation_reaches_into_nested_blocks() {
    let source = r"
        do {
            def a = 2
            do { def b = a * 3; b + 1 }
        }
        ";
    let emitted = code(source, FOLD_AND_PROPAGATE);
    assert_eq!(definitions(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(7)), 1, "{emitted:?}");
}

#[test]
fn propagation_stops_at_a_runtime_shadow() {
    let source = r"
        do {
            def a = 2
            do { def a = x; a + 1 }
        }
        ";
    let tail = Script::new(source).capture("x", Value::Int(10)).run();
    assert_eq!(tail, Value::Int(11), "the inner `a` is `x`");
    let emitted = code(source, FOLD_AND_PROPAGATE);
    assert_eq!(
        emitted.count(&Bytecode::Add),
        1,
        "the inner `a` is runtime: {emitted:?}"
    );
}

#[test]
fn a_runtime_bodys_constant_if_tail_folds_alone() {
    let emitted = code("do { def y = x; if true: 1 else: 2 }", FOLD);
    assert!(
        !emitted
            .code
            .iter()
            .any(|op| matches!(op, Bytecode::Jump(_) | Bytecode::JumpIfFalse(_))),
        "the tail folds: {emitted:?}"
    );
    assert_eq!(emitted.count(&Bytecode::PushInt(1)), 1, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 0, "{emitted:?}");
}

#[test]
fn a_folded_block_may_decide_a_branch() {
    let emitted = code(
        "if do { def c = 1; c == 1 }: x else: 2",
        FOLD_AND_PROPAGATE.with(Optimization::BranchEliminate, true),
    );
    assert_eq!(definitions(&emitted), 0, "the block folded: {emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::PushInt(2)),
        0,
        "the alternate is dropped: {emitted:?}"
    );
    assert!(
        !emitted
            .code
            .iter()
            .any(|op| matches!(op, Bytecode::Jump(_) | Bytecode::JumpIfFalse(_))),
        "the test is eliminated: {emitted:?}"
    );
}

#[test]
fn a_raising_block_in_an_untaken_branch_never_raises() {
    let source = r"
        if x: do {
            def y = 1 / 0
            y
        }
        else: 5
        ";
    let tail = Script::new(source).capture("x", Value::Bool(false)).run();
    assert_eq!(tail, Value::Int(5));
    let message = Script::new(source).capture("x", Value::Bool(true)).raises();
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn without_folding_a_constant_block_is_kept() {
    let emitted = code("do { def y = 2; 5 }", UNOPTIMIZED);
    assert_eq!(definitions(&emitted), 1, "{emitted:?}");
}
