//! The Vm's debug-only invariant checks, exercised by deliberately malformed bytecode.
//!
//! These describe what the compiler must not emit. Each violation here is silent
//! without the assertion: a runaway jump falls out of the dispatch loop and returns
//! early, and an over-deep `PeekDown`/`DropBelow` reads or removes an operand
//! belonging to the calling frame. Both would surface far from their cause.
//!
//! The whole module is debug-only, since a release build compiles the assertions
//! out and none of these would panic.
#![cfg(debug_assertions)]

use std::sync::Arc;

mod common;

use common::{Pop, func, run_fn};
use frost_runtime::{Arity, Bytecode, CompiledFunction};

use Bytecode::*;

/// `CreateClosure` for the first child function, capturing nothing.
const MAKE_CHILD: Bytecode = CreateClosure {
    num_captures: 0,
    function: 0,
};

/// A program that calls `callee` with no arguments, having first left `spare`
/// values of its own on the operand stack.
///
/// Those spare values sit below the callee's frame base, so a callee that reaches
/// past its own operands lands in them.
fn calling_program(spare: usize, callee: Arc<CompiledFunction>) -> Arc<CompiledFunction> {
    let mut code: Vec<Bytecode> = (0..spare).map(|i| PushInt(i as i64)).collect();
    code.extend([MAKE_CHILD, Call(0)]);
    // Discard the spares from under the result: the top level must itself finish
    // with at most one value on the stack.
    code.extend((0..spare).map(|_| DropBelow(1)));
    func(code, Arity::Exact(0), vec![], vec![callee])
}

// ============================================================
// Jump targets
// ============================================================

#[test]
#[should_panic(expected = "leaves the function")]
fn a_jump_past_the_end_is_caught() {
    run_fn(func(vec![Jump(50)], Arity::Exact(0), vec![], vec![]));
}

#[test]
#[should_panic(expected = "leaves the function")]
fn a_conditional_jump_past_the_end_is_caught() {
    run_fn(func(
        vec![PushTrue, PeekJumpIfTrue(50)],
        Arity::Exact(0),
        vec![],
        vec![],
    ));
}

#[test]
fn a_jump_to_one_past_the_end_is_the_return_position() {
    // A function with no `Return` returns by falling off the end, so a branch
    // reaching the exit lands exactly one past the last instruction. Legal.
    let result = run_fn(func(
        vec![PushInt(7), Jump(1)],
        Arity::Exact(0),
        vec![],
        vec![],
    ));
    assert_eq!(result.tail(), &frost_runtime::Value::Int(7));
}

#[test]
fn a_not_taken_jump_is_never_checked() {
    // The target is only meaningful when the branch is taken; an untaken branch
    // past the end is unreachable code, not a violation.
    run_fn(func(
        vec![PushFalse, PeekJumpIfTrue(50), Pop],
        Arity::Exact(0),
        vec![],
        vec![],
    ));
}

// ============================================================
// Frame-base floor
// ============================================================

#[test]
#[should_panic(expected = "below the running frame's base")]
fn peeking_below_the_frame_base_is_caught() {
    // The callee pops its own function value, leaving it nothing of its own on
    // the stack, so any peek reaches into the caller's operands.
    let callee = func(vec![Pop, PeekDown(0)], Arity::Exact(0), vec![], vec![]);
    run_fn(calling_program(2, callee));
}

#[test]
#[should_panic(expected = "below the running frame's base")]
fn dropping_below_the_frame_base_is_caught() {
    let callee = func(vec![Pop, DropBelow(0)], Arity::Exact(0), vec![], vec![]);
    run_fn(calling_program(2, callee));
}

#[test]
#[should_panic(expected = "below the running frame's base")]
fn popping_below_the_frame_base_is_caught() {
    // The first pop takes the callee's own function value, leaving it at its base;
    // the second reaches into the caller. Covers every instruction that pops.
    let callee = func(vec![Pop, Pop], Arity::Exact(0), vec![], vec![]);
    run_fn(calling_program(2, callee));
}

#[test]
#[should_panic(expected = "below the running frame's base")]
fn branching_on_a_callers_operand_is_caught() {
    // A peek rather than a pop: reading the caller's top would branch on a value
    // this function never produced, and the stack would look untouched afterward.
    let callee = func(vec![Pop, PeekJumpIfTrue(0)], Arity::Exact(0), vec![], vec![]);
    run_fn(calling_program(2, callee));
}

#[test]
#[should_panic(expected = "below the running frame's base")]
fn calling_a_callers_operand_is_caught() {
    // `Call` takes its callee from `argc + 1` below the top; with nothing of its
    // own on the stack, that lands in the caller's operands.
    let callee = func(vec![Pop, Call(0)], Arity::Exact(0), vec![], vec![]);
    run_fn(calling_program(2, callee));
}

#[test]
#[should_panic(expected = "below the running frame's base")]
fn collecting_a_callers_operand_into_a_structure_is_caught() {
    let callee = func(vec![Pop, MakeArray(1)], Arity::Exact(0), vec![], vec![]);
    run_fn(calling_program(2, callee));
}

#[test]
fn reaching_within_the_frame_is_allowed() {
    // The same shape, but the callee peeks at a value it pushed itself, then
    // discards the copy so it still returns exactly one value.
    let callee = func(
        vec![Pop, PushInt(1), PeekDown(0), Pop],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    run_fn(calling_program(2, callee));
}

// ============================================================
// Stack underflow (checked in every build, not just debug)
// ============================================================

#[test]
#[should_panic(expected = "FROST STACK UNDERFLOW")]
fn peeking_past_the_whole_stack_underflows() {
    run_fn(func(vec![PeekDown(5)], Arity::Exact(0), vec![], vec![]));
}

#[test]
#[should_panic(expected = "FROST STACK UNDERFLOW")]
fn dropping_past_the_whole_stack_underflows() {
    run_fn(func(vec![DropBelow(5)], Arity::Exact(0), vec![], vec![]));
}

// ============================================================
// The calling convention's return shape
// ============================================================

#[test]
#[should_panic(expected = "must leave exactly its result at its frame base")]
fn returning_with_a_surplus_value_is_caught() {
    // Two values left where the convention allows exactly one.
    let callee = func(
        vec![Pop, PushInt(1), PushInt(2)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    run_fn(calling_program(0, callee));
}

#[test]
#[should_panic(expected = "must leave exactly its result at its frame base")]
fn returning_without_a_result_is_caught() {
    // The callee consumed its function value and produced nothing.
    let callee = func(vec![Pop], Arity::Exact(0), vec![], vec![]);
    run_fn(calling_program(0, callee));
}

// ============================================================
// Stack marks balanced at return
// ============================================================

#[test]
#[should_panic(expected = "balanced every stack mark")]
fn returning_with_an_unbalanced_mark_is_caught() {
    // The callee saves a mark and never drops or rewinds it, yet still returns a
    // single result: the leaked mark is caught only by the marks-balanced assert.
    let callee = func(
        vec![Pop, MarkStack, PushInt(0)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    run_fn(calling_program(0, callee));
}
