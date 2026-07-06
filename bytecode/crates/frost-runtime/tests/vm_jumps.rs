//! Tests for the control-flow opcodes: `Jump`, `JumpIfTrue`, `JumpIfFalse`.
//!
//! Convention (forward-only): a taken jump does `pc += n`, and the dispatch loop's trailing `pc += 1` supplies the step --
//! so the effective advance is `n + 1`, i.e. `Jump(n)` skips the next `n` instructions and `Jump(0)` is a Nop.
//! `JumpIf*` peek the condition (it is NOT consumed) and fall through (advance 1) when not taken.
//! Truthiness follows Frost: only `null`/`false` are falsy.
//!
//! Tests pin behavior by making a skipped vs. executed instruction observable in the tail value:
//! a `PushInt(999)` that runs only if a jump was *not* taken, or a distinctive landing instruction.

use std::sync::Arc;

use frost_runtime::{Arity, Bytecode, CompiledFunction, Value, Vm};

/// Run a local-less, constant-less program and return its tail value.
fn tail(code: Vec<Bytecode>) -> Value {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        name: "<jumps>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.into_closure().unwrap();
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .unwrap()
        .tail()
        .clone()
}

use Bytecode::{Jump, JumpIfFalse, JumpIfTrue, Pop, PushFalse, PushInt, PushNull, PushTrue};

// ============================================================
// Jump (unconditional)
// ============================================================

#[test]
fn jump_zero_is_nop() {
    // Jump(0) must step to the very next instruction (not self-loop, not skip).
    // If it self-looped this test would hang; if it skipped PushInt(2) the program
    // would leave nothing.
    assert_eq!(tail(vec![Jump(0), PushInt(2)]), Value::Int(2));
}

#[test]
fn jump_skips_one_and_lands_on_next() {
    // Jump(1) skips PushInt(999) and lands on PushInt(7): tail is 7, never 999.
    assert_eq!(tail(vec![Jump(1), PushInt(999), PushInt(7)]), Value::Int(7));
}

#[test]
fn jump_skips_n_following_instructions() {
    // Jump(2) skips both trailing pushes (they would otherwise sit on top); tail
    // remains the pre-jump value.
    assert_eq!(
        tail(vec![PushInt(1), Jump(2), PushInt(998), PushInt(999)]),
        Value::Int(1)
    );
}

// ============================================================
// JumpIfTrue (peek; taken when truthy)
// ============================================================

#[test]
fn jump_if_true_taken_when_truthy() {
    // true -> jump taken, skipping PushInt(999). The condition is left on the
    // stack (not consumed), so it is the tail.
    assert_eq!(
        tail(vec![PushTrue, JumpIfTrue(1), PushInt(999)]),
        Value::Bool(true)
    );
}

#[test]
fn jump_if_true_not_taken_when_falsy() {
    // false -> not taken, falls through; drop the un-consumed condition, then PushInt(7).
    assert_eq!(
        tail(vec![PushFalse, JumpIfTrue(1), Pop, PushInt(7)]),
        Value::Int(7)
    );
}

#[test]
fn jump_if_true_taken_on_zero() {
    // 0 is truthy in Frost, so the jump is taken (999 skipped); tail is the 0.
    assert_eq!(
        tail(vec![PushInt(0), JumpIfTrue(1), PushInt(999)]),
        Value::Int(0)
    );
}

// ============================================================
// JumpIfFalse (peek; taken when falsy)
// ============================================================

#[test]
fn jump_if_false_taken_when_falsy() {
    // false -> jump taken, skipping PushInt(999); condition left on the stack.
    assert_eq!(
        tail(vec![PushFalse, JumpIfFalse(1), PushInt(999)]),
        Value::Bool(false)
    );
}

#[test]
fn jump_if_false_not_taken_when_truthy() {
    // true -> not taken, falls through; drop the un-consumed condition, then PushInt(7).
    assert_eq!(
        tail(vec![PushTrue, JumpIfFalse(1), Pop, PushInt(7)]),
        Value::Int(7)
    );
}

#[test]
fn jump_if_false_taken_on_null() {
    // null is falsy, so the jump is taken (999 skipped); tail is the null.
    assert_eq!(
        tail(vec![PushNull, JumpIfFalse(1), PushInt(999)]),
        Value::Null
    );
}

// ============================================================
// Condition is not consumed
// ============================================================

#[test]
fn jump_if_does_not_consume_condition() {
    // 42 below the condition. JumpIfTrue is not taken (false), then an explicit
    // Pop removes the *condition*; revealing 42 proves the condition was still on
    // the stack (a consuming JumpIf would have popped 42 instead).
    assert_eq!(
        tail(vec![PushInt(42), PushFalse, JumpIfTrue(5), Pop]),
        Value::Int(42)
    );
}

// ============================================================
// Composition: a real if/else branch
// ============================================================

/// `if <cond>: 100 else: 200`, with the condition popped in each branch (since
/// `JumpIf*` does not consume it).
///
/// ```text
/// 0  <cond>
/// 1  JumpIfFalse(3)   -- taken -> index 5 (else); skips 2,3,4
/// 2  Pop              -- drop cond (then-branch)
/// 3  PushInt(100)
/// 4  Jump(2)          -- skip 5,6 -> end
/// 5  Pop              -- drop cond (else-branch)
/// 6  PushInt(200)
/// ```
fn if_else(cond: Bytecode) -> Value {
    tail(vec![
        cond,
        JumpIfFalse(3),
        Pop,
        PushInt(100),
        Jump(2),
        Pop,
        PushInt(200),
    ])
}

#[test]
fn if_else_takes_then_branch() {
    assert_eq!(if_else(PushTrue), Value::Int(100));
}

#[test]
fn if_else_takes_else_branch() {
    assert_eq!(if_else(PushFalse), Value::Int(200));
}
