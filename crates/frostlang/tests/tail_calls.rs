//! Tail position: a call is emitted as a `TailCall` exactly when nothing in its
//! function executes after it.
//!
//! Tail position is not an optimization: it is decided during lowering, under
//! every optimization permutation. These tests read the code without
//! optimization, except where they check how tail position survives one. The
//! bounded-depth tests run under every permutation, checking that tail calls
//! through each construct really reuse their frame.
//!
//! Each probed call site calls `f` with a distinct argument count, so the
//! emitted call opcode identifies which site it is.

use crate::script;

use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use frostlang::{Arity, Value};
use script::{Emitted, Script, UNOPTIMIZED};

/// The top-level code of `source` under exactly `optimization`, with `x` a
/// runtime-only Bool and `f` a runtime-only function.
fn code_under(source: &str, optimization: OptimizationOptions) -> Emitted {
    let f = Value::native("f", Arity::AtLeast(0), |_, _| Ok(Value::Null));
    Script::new(source)
        .capture("x", Value::Bool(true))
        .capture("f", f)
        .code(optimization)
}

fn code(source: &str) -> Emitted {
    code_under(source, UNOPTIMIZED)
}

/// The argument counts of the calls in the code, split into (ordinary calls,
/// tail calls).
fn calls_by_arity(emitted: &Emitted) -> (Vec<usize>, Vec<usize>) {
    let mut ordinary = Vec::new();
    let mut tail = Vec::new();
    for op in &emitted.code {
        match op {
            Bytecode::Call(n) => ordinary.push(*n),
            Bytecode::TailCall(n) => tail.push(*n),
            _ => {}
        }
    }
    ordinary.sort_unstable();
    tail.sort_unstable();
    (ordinary, tail)
}

/// Assert that the calls with these argument counts are ordinary and tail calls.
fn assert_calls(source: &str, ordinary: &[usize], tail: &[usize]) {
    let emitted = code(source);
    assert_eq!(
        calls_by_arity(&emitted),
        (ordinary.to_vec(), tail.to_vec()),
        "{source:?}: (ordinary, tail) call arities: {emitted:?}"
    );
}

// --- Tail positions ---

#[test]
fn the_final_statement_is_in_tail_position() {
    assert_calls("f(1)", &[], &[1]);
    let source = r"
        f(1)
        f(2, 2)
    ";
    assert_calls(source, &[1], &[2]);
}

#[test]
fn both_branches_of_an_if_in_tail_position_are() {
    assert_calls("if x: f(1) else: f(2, 2)", &[], &[1, 2]);
    assert_calls("if x: f(1)", &[], &[1]);
    assert_calls(
        "if x: f(1) elif x: f(2, 2) else: f(3, 3, 3)",
        &[],
        &[1, 2, 3],
    );
}

#[test]
fn the_final_expression_of_a_do_in_tail_position_is() {
    let source = r"
        do {
            f(1)
            def y = f(2, 2)
            f(3, 3, 3)
        }
    ";
    assert_calls(source, &[1, 2], &[3]);
}

#[test]
fn the_right_operand_of_a_logical_in_tail_position_is() {
    assert_calls("x and f(1)", &[], &[1]);
    assert_calls("x or f(1)", &[], &[1]);
    assert_calls("f() and f(1)", &[0], &[1]);
}

#[test]
fn tail_position_propagates_through_nesting() {
    let source = r"
        if x: (x and f(1))
        else: do {
            f(2, 2)
            (if x: f(3, 3, 3) else: f())
        }
    ";
    assert_calls(source, &[2], &[0, 1, 3]);
}

#[test]
fn an_iteration_form_in_tail_position_is_a_tail_call() {
    // Each form is a call to its global with the structure and operation (and
    // any `init`) as arguments, which are not in tail position.
    for form in ["map", "filter", "reduce", "foreach"] {
        assert_calls(&format!("{form} f(1) with f(3, 3, 3)"), &[1, 3], &[2]);
    }
    assert_calls(
        "reduce f(1) init: f(4, 4, 4, 4) with f(3, 3, 3)",
        &[1, 3, 4],
        &[3],
    );
}

#[test]
fn an_elif_branch_in_tail_position_is_but_its_condition_is_not() {
    assert_calls(
        "if x: f(1) elif f(): f(2, 2) else: f(3, 3, 3)",
        &[0],
        &[1, 2, 3],
    );
}

#[test]
fn a_lambda_body_has_its_own_tail_position() {
    // Wherever the lambda expression sits, its body's final call is the last
    // code its own function runs.
    for source in [
        "[fn -> f(1), 2]",
        r"
        def h = fn -> f(1)
        h
        ",
        "fn -> f(1)",
    ] {
        let emitted = code(source);
        assert_eq!(
            calls_by_arity(&emitted),
            (vec![], vec![]),
            "{source:?}: creating the lambda calls nothing: {emitted:?}"
        );
        let body = emitted.nested(0);
        assert_eq!(
            calls_by_arity(&body),
            (vec![], vec![1]),
            "{source:?}: {body:?}"
        );
    }
}

#[test]
fn tail_position_propagates_through_a_lambda_body() {
    let source = r"
        fn -> if x: do {
            f(1)
            x and f(3, 3, 3)
        }
        else: map x with f
    ";
    let body = code(source).nested(0);
    assert_eq!(calls_by_arity(&body), (vec![1], vec![2, 3]), "{body:?}");
}

// --- Inner positions ---

#[test]
fn a_non_final_statement_is_not_in_tail_position() {
    let call_then_value = r"
        f(1)
        2
    ";
    assert_calls(call_then_value, &[1], &[]);
    let bound_then_read = r"
        def y = f(1)
        y
    ";
    assert_calls(bound_then_read, &[1], &[]);
}

#[test]
fn an_if_condition_is_not_in_tail_position() {
    assert_calls("if f(): f(1) else: f(2, 2)", &[0], &[1, 2]);
}

#[test]
fn the_left_operand_of_a_logical_is_not_in_tail_position() {
    assert_calls("f() or x", &[0], &[]);
    assert_calls("f() and x", &[0], &[]);
}

#[test]
fn call_arguments_and_callees_are_not_in_tail_position() {
    assert_calls("f(f(1))", &[1], &[1]);
    assert_calls("f()(1)", &[0], &[1]);
}

#[test]
fn operands_are_not_in_tail_position() {
    assert_calls("f() + 1", &[0], &[]);
    assert_calls("1 + f()", &[0], &[]);
    assert_calls("not f()", &[0], &[]);
    assert_calls("-f()", &[0], &[]);
}

#[test]
fn a_selection_in_inner_position_passes_inner_on() {
    // The same shapes as the tail-position tests, used as an operand: code runs
    // after them, so none of their calls is a tail call.
    assert_calls("(if x: f(1) else: f(2, 2)) + 1", &[1, 2], &[]);
    let block = r"
        do {
            f(1)
            f(2, 2)
        } + 1
    ";
    assert_calls(block, &[1, 2], &[]);
    assert_calls("(x and f(1)) + 1", &[1], &[]);
    assert_calls("f(if x: f(1) else: f(2, 2))", &[1, 2], &[1]);
}

#[test]
fn a_selection_statement_that_is_not_final_passes_inner_on() {
    let selection = r"
        if x: f(1) else: f(2, 2)
        5
    ";
    assert_calls(selection, &[1, 2], &[]);
    let logical = r"
        x or f(1)
        5
    ";
    assert_calls(logical, &[1], &[]);
    let block = r"
        do { f(1) }
        5
    ";
    assert_calls(block, &[1], &[]);
    let iteration = r"
        map x with f
        5
    ";
    assert_calls(iteration, &[2], &[]);
    let nested_selection = r"
        do {
            if x: f(1) else: f(2, 2)
            f(3, 3, 3)
        }
    ";
    assert_calls(nested_selection, &[1, 2], &[3]);
}

#[test]
fn a_def_rhs_is_not_in_tail_position_even_as_the_final_statement() {
    // The binding is made after the call returns.
    assert_calls("def y = f(1)", &[1], &[]);
    assert_calls("def [a, b] = f(1)", &[1], &[]);
    assert_calls("def {a} = f(1)", &[1], &[]);
    let selection = r"
        def y = if x: f(1) else: do { f(2, 2) }
        y
    ";
    assert_calls(selection, &[1, 2], &[]);
    let logical = r"
        def y = x and f(1)
        y
    ";
    assert_calls(logical, &[1], &[]);
}

#[test]
fn a_logical_in_a_condition_is_not_in_tail_position() {
    assert_calls("if x and f(1): 2 else: 3", &[1], &[]);
    assert_calls("if f() or f(1): f(2, 2)", &[0, 1], &[2]);
}

#[test]
fn format_string_interpolations_are_not_in_tail_position() {
    assert_calls("$'${f(1)}'", &[1], &[]);
    assert_calls("$'a${f(1)}b${f(2, 2)}c'", &[1, 2], &[]);
}

#[test]
fn index_targets_and_keys_are_not_in_tail_position() {
    assert_calls("f(1)[f(2, 2)]", &[1, 2], &[]);
    assert_calls("f(1).key", &[1], &[]);
}

#[test]
fn structure_literal_elements_are_not_in_tail_position() {
    assert_calls("[f(1), f(2, 2)]", &[1, 2], &[]);
    assert_calls("{a: f(1), [f(2, 2)]: 3}", &[1, 2], &[]);
}

#[test]
fn iteration_form_arguments_are_not_in_tail_position() {
    // Only the form's own call is; each argument runs before it.
    assert_calls(
        "map (x and f(1)) with (if x: f(3, 3, 3) else: f)",
        &[1, 3],
        &[2],
    );
}

// --- Interaction with optimization ---

#[test]
fn a_tail_call_survives_branch_elimination() {
    let eliminate = UNOPTIMIZED.with(Optimization::BranchEliminate, true);
    for source in ["if true: f(1) else: f(2, 2)", "true and f(1)"] {
        let emitted = code_under(source, eliminate);
        assert_eq!(
            calls_by_arity(&emitted),
            (vec![], vec![1]),
            "{source:?}: the surviving branch keeps its tail call: {emitted:?}"
        );
    }
}

#[test]
fn a_tail_call_survives_propagated_elimination() {
    let propagate_and_eliminate = UNOPTIMIZED
        .with(Optimization::ConstantPropagate, true)
        .with(Optimization::BranchEliminate, true);
    for source in [
        r"
        def c = true
        if c: f(1) else: f(2, 2)
        ",
        r"
        def c = false
        if c: f(2, 2) elif c: f(3, 3, 3) else: f(1)
        ",
        r"
        def c = null
        c or f(1)
        ",
        r"
        do {
            def c = 0
            c and f(1)
        }
        ",
    ] {
        let emitted = code_under(source, propagate_and_eliminate);
        assert_eq!(
            calls_by_arity(&emitted),
            (vec![], vec![1]),
            "{source:?}: the surviving branch keeps its tail call: {emitted:?}"
        );
    }
}

#[test]
fn a_tail_call_survives_elimination_inside_a_lambda() {
    let eliminate = UNOPTIMIZED.with(Optimization::BranchEliminate, true);
    let body = code_under("fn -> if true: f(1) else: f(2, 2)", eliminate).nested(0);
    assert_eq!(calls_by_arity(&body), (vec![], vec![1]), "{body:?}");
}

#[test]
fn a_tail_call_survives_folding_around_it() {
    let fold = UNOPTIMIZED.with(Optimization::ConstantFold, true);
    for source in [
        "if x: f(1) else: 1 + 2",
        r"
        do {
            def y = 2 * 3
            f(1)
        }
        ",
    ] {
        let emitted = code_under(source, fold);
        assert_eq!(
            calls_by_arity(&emitted),
            (vec![], vec![1]),
            "{source:?}: {emitted:?}"
        );
    }
    let folded_argument = r"
        do {
            def y = 2 * 3
            f(y)
        }
    ";
    let emitted = code_under(
        folded_argument,
        fold.with(Optimization::ConstantPropagate, true),
    );
    assert_eq!(
        calls_by_arity(&emitted),
        (vec![], vec![1]),
        "the argument folds, the tail call stays: {emitted:?}"
    );
    assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
}

#[test]
fn a_pure_tail_call_folds_to_its_value() {
    // In tail position the call is a tail call, and the fold evaluates it as one.
    let source = r"
        do {
            def y = x
            to_string(1 + 2)
        }
    ";
    assert_calls(source, &[], &[1]);
    let emitted = code_under(source, UNOPTIMIZED.with(Optimization::ConstantFold, true));
    assert_eq!(
        calls_by_arity(&emitted),
        (vec![], vec![]),
        "the call folds away: {emitted:?}"
    );
    let tail = Script::new(source).capture("x", Value::Null).run();
    assert_eq!(tail, Value::from("3"));
}

// --- Bounded depth ---
//
// A hundred thousand calls under a depth limit of a hundred frames: only
// possible if each call in tail position reuses its frame.

/// Run `source`, which must stay far under a small call depth limit.
fn run_shallow(source: &str) -> Value {
    Script::new(source).max_call_depth(100).run()
}

#[test]
fn tail_recursion_through_a_logical_runs_in_bounded_depth() {
    let or = r"
        defn down(n) -> n == 0 or down(n - 1)
        down(100000)
    ";
    assert_eq!(run_shallow(or), Value::Bool(true));
    let and = r"
        defn down(n) -> n > 0 and down(n - 1)
        down(100000)
    ";
    assert_eq!(run_shallow(and), Value::Bool(false));
}

#[test]
fn tail_recursion_through_elif_and_do_runs_in_bounded_depth() {
    let source = r"
        defn down(n, acc) -> if n == 0: acc
            elif n % 2 == 0: do {
                def m = n - 1
                down(m, acc + 2)
            }
            else: down(n - 1, acc + 1)
        down(100000, 0)
    ";
    assert_eq!(run_shallow(source), Value::Int(150_000));
}

#[test]
fn mutual_tail_recursion_runs_in_bounded_depth() {
    // Each function tail-calls the other, passed in as an argument.
    let source = r#"
        defn ping(n, other) -> if n == 0: "ping" else: other(n - 1, ping)
        defn pong(n, other) -> if n == 0: "pong" else: other(n - 1, pong)
        ping(100001, pong)
    "#;
    assert_eq!(run_shallow(source), Value::from("pong"));
}
