//! Tail position: a call is emitted as a `TailCall` exactly when nothing in its
//! function executes after it.
//!
//! Tail position is not an optimization: it is decided during lowering, under
//! every optimization permutation. These tests read the code without
//! optimization, except where they check how tail position survives one.
//!
//! Each probed call site is a distinct capture (`f`, `g`, ...) given a distinct
//! argument count, so the emitted call opcode identifies which site it is.

mod common;

use common::{Emitted, Script, UNOPTIMIZED};
use frost_compile::OptimizationOptions;
use frost_runtime::{Arity, Bytecode, Value};

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
    assert_calls("f(1); f(2, 2)", &[1], &[2]);
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
    assert_calls("do { f(1); def y = f(2, 2); f(3, 3, 3) }", &[1, 2], &[3]);
}

#[test]
fn the_right_operand_of_a_logical_in_tail_position_is() {
    assert_calls("x and f(1)", &[], &[1]);
    assert_calls("x or f(1)", &[], &[1]);
    assert_calls("f() and f(1)", &[0], &[1]);
}

#[test]
fn tail_position_propagates_through_nesting() {
    assert_calls(
        "if x: (x and f(1)) else: do { f(2, 2); (if x: f(3, 3, 3) else: f()) }",
        &[2],
        &[0, 1, 3],
    );
}

// --- Inner positions ---

#[test]
fn a_non_final_statement_is_not_in_tail_position() {
    assert_calls("f(1); 2", &[1], &[]);
    assert_calls("def y = f(1); y", &[1], &[]);
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
    assert_calls("do { f(1); f(2, 2) } + 1", &[1, 2], &[]);
    assert_calls("(x and f(1)) + 1", &[1], &[]);
    assert_calls("f(if x: f(1) else: f(2, 2))", &[1, 2], &[1]);
}

// --- Interaction with optimization ---

#[test]
fn a_tail_call_survives_branch_elimination() {
    let eliminate = OptimizationOptions {
        branch_eliminate: true,
        ..UNOPTIMIZED
    };
    for source in ["if true: f(1) else: f(2, 2)", "true and f(1)"] {
        let emitted = code_under(source, eliminate);
        assert_eq!(
            calls_by_arity(&emitted),
            (vec![], vec![1]),
            "{source:?}: the surviving branch keeps its tail call: {emitted:?}"
        );
    }
}
