#![allow(clippy::approx_constant)]

mod common;

use common::run;
use frost_runtime::{Bytecode, FrostArray, FrostFloat, Value};

// ============================================================
// Empty program
// ============================================================

#[test]
fn empty_program_returns_null_tail() {
    let result = run(vec![]);
    assert_eq!(result.tail(), &Value::Null);
}

// ============================================================
// Push constants
// ============================================================

#[test]
fn push_null() {
    let result = run(vec![Bytecode::PushNull]);
    assert_eq!(result.tail(), &Value::Null);
}

#[test]
fn push_true() {
    let result = run(vec![Bytecode::PushTrue]);
    assert_eq!(result.tail(), &Value::Bool(true));
}

#[test]
fn push_false() {
    let result = run(vec![Bytecode::PushFalse]);
    assert_eq!(result.tail(), &Value::Bool(false));
}

#[test]
fn push_int() {
    let result = run(vec![Bytecode::PushInt(42)]);
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn push_float() {
    let f = FrostFloat::new(3.14).unwrap();
    let result = run(vec![Bytecode::PushFloat(f)]);
    assert_eq!(result.tail(), &Value::Float(f));
}

#[test]
fn tail_is_top_of_stack() {
    let result = run(vec![
        Bytecode::PushInt(1),
        Bytecode::PushInt(2),
        Bytecode::PushInt(3),
    ]);
    assert_eq!(result.tail(), &Value::Int(3));
}

// ============================================================
// Stack manipulation
// ============================================================

#[test]
fn pop_discards_top() {
    let result = run(vec![
        Bytecode::PushInt(1),
        Bytecode::PushInt(2),
        Bytecode::Pop,
    ]);
    assert_eq!(result.tail(), &Value::Int(1));
}

#[test]
fn dup_copies_top() {
    let result = run(vec![
        Bytecode::PushInt(42),
        Bytecode::Dup,
        Bytecode::Pop, // pop the dup
    ]);
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn dup_leaves_two_copies() {
    let result = run(vec![Bytecode::PushInt(7), Bytecode::Dup]);
    // stack: 7, 7 -- tail is top = 7
    assert_eq!(result.tail(), &Value::Int(7));
}

#[test]
fn peek_down_1_is_dup() {
    // PeekDown(1) copies the top item (same as Dup)
    let result = run(vec![
        Bytecode::PushInt(10),
        Bytecode::PushInt(20),
        Bytecode::PeekDown(1), // copies 20 (top)
        Bytecode::Pop,         // pop the copy
    ]);
    assert_eq!(result.tail(), &Value::Int(20));
}

#[test]
fn peek_down_copies_deeper() {
    // PeekDown(2) copies the item below the top
    let result = run(vec![
        Bytecode::PushInt(10),
        Bytecode::PushInt(20),
        Bytecode::PeekDown(2), // copies 10 (below top)
    ]);
    assert_eq!(result.tail(), &Value::Int(10));
}

#[test]
fn drop_below_0_is_pop() {
    // DropBelow(0) removes the top, exactly like Pop.
    let result = run(vec![
        Bytecode::PushInt(1),
        Bytecode::PushInt(2),
        Bytecode::DropBelow(0),
    ]);
    assert_eq!(result.tail(), &Value::Int(1));
}

#[test]
fn drop_below_removes_element_under_top() {
    // DropBelow(1) removes the element one below the top, keeping the top and the
    // rest in order: [1, 2, 3] -> [1, 3].
    let result = run(vec![
        Bytecode::PushInt(1),
        Bytecode::PushInt(2),
        Bytecode::PushInt(3),
        Bytecode::DropBelow(1),
        Bytecode::MakeArray(2),
    ]);
    assert_eq!(
        result.tail(),
        &Value::Array(FrostArray::from(vec![Value::Int(1), Value::Int(3)]))
    );
}

#[test]
fn drop_below_removes_a_deeper_element() {
    // DropBelow(2) removes the element two below the top: [1, 2, 3, 4] -> [1, 3, 4].
    let result = run(vec![
        Bytecode::PushInt(1),
        Bytecode::PushInt(2),
        Bytecode::PushInt(3),
        Bytecode::PushInt(4),
        Bytecode::DropBelow(2),
        Bytecode::MakeArray(3),
    ]);
    assert_eq!(
        result.tail(),
        &Value::Array(FrostArray::from(vec![
            Value::Int(1),
            Value::Int(3),
            Value::Int(4)
        ]))
    );
}
