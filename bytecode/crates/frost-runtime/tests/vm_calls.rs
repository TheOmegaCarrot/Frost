//! Happy-path tests for `Call` into a VM closure.
//!
//! Contract under test: a function and its arguments sit on the stack, the call
//! consumes them, and exactly one value is left behind -- `( f a1..an -- r )`.
//!
//! Closures can only be obtained via `CreateClosure` (their fields are private),
//! so every test builds a parent function whose `child_fns[0]` is the callee and
//! whose body creates the closure, pushes args, and calls it. Bodies use only
//! implemented opcodes; nothing here reaches a `todo!()` (no error paths, no
//! arithmetic).
//!
//! The canonical fixed-arity callee is the identity `fn x -> x`, whose body is a
//! minimal "-O0" prelude: move the arg into a slot, pop the function value, load
//! the arg back as the result.

mod common;

use common::{entry, func, run_fn};
use frost_runtime::{Arity, Bytecode, FrostArray, Value};

// ============================================================
// Fixed arity
// ============================================================

#[test]
fn call_closure_returns_result() {
    // fn x -> x
    let callee = func(
        vec![
            Bytecode::DefLocal(0), // arg -> slot 0
            Bytecode::Pop,         // pop the function value
            Bytecode::LoadLocal(0),
        ],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(42),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn call_leaves_exactly_one_value() {
    // A sentinel sits below the call. The call consumes (closure, 42) and leaves
    // exactly one result on top; popping it reveals the sentinel -- proving
    // `( f a.. -- r )` and that the caller resumed to run the trailing Pop.
    let callee = func(
        vec![
            Bytecode::DefLocal(0),
            Bytecode::Pop,
            Bytecode::LoadLocal(0),
        ],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::PushInt(7), // sentinel
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(42),
            Bytecode::Call(1),
            Bytecode::Pop, // discard the single result
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(7));
}

#[test]
fn caller_resumes_after_call() {
    // The instruction after Call only runs if control returned to the caller.
    let callee = func(
        vec![
            Bytecode::DefLocal(0),
            Bytecode::Pop,
            Bytecode::LoadLocal(0),
        ],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(42),
            Bytecode::Call(1),
            Bytecode::Pop,         // drop the call's result
            Bytecode::PushInt(99), // observable only if the caller resumed
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(99));
}

// ============================================================
// Captures
// ============================================================

#[test]
fn call_closure_returns_capture() {
    // fn () -> <captured> : zero args, one capture seated in slot 0.
    let callee = func(
        vec![
            Bytecode::Pop,          // pop the function value
            Bytecode::LoadLocal(0), // the capture
        ],
        Arity::Exact(0),
        vec![entry("c", false)], // slot 0 holds the capture
        vec![],
    );
    let program = func(
        vec![
            Bytecode::PushInt(123), // value to capture
            Bytecode::CreateClosure {
                num_captures: 1,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(123));
}

// ============================================================
// Variadics (rest collapsed into an Array by the Call dispatch)
// ============================================================

#[test]
fn call_variadic_collapses_rest() {
    // fn ...rest -> rest
    let callee = func(
        vec![
            Bytecode::DefLocal(0), // rest array -> slot 0
            Bytecode::Pop,
            Bytecode::LoadLocal(0),
        ],
        Arity::AtLeast(0),
        vec![entry("rest", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::Call(3),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    let expected = Value::Array(FrostArray::from(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ]));
    assert_eq!(result.tail(), &expected);
}

#[test]
fn call_variadic_empty_rest() {
    // fn ...rest -> rest, called with no args -> rest is the empty array.
    let callee = func(
        vec![
            Bytecode::DefLocal(0),
            Bytecode::Pop,
            Bytecode::LoadLocal(0),
        ],
        Arity::AtLeast(0),
        vec![entry("rest", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Array(FrostArray::empty()));
}

#[test]
fn call_variadic_splits_fixed_and_rest() {
    // fn a, ...rest -> rest : slot 0 = a (fixed), slot 1 = rest (collapsed).
    let callee = func(
        vec![
            Bytecode::DefLocal(1), // rest array (top) -> slot 1
            Bytecode::DefLocal(0), // a -> slot 0
            Bytecode::Pop,
            Bytecode::LoadLocal(1),
        ],
        Arity::AtLeast(1),
        vec![entry("a", false), entry("rest", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(10), // a
            Bytecode::PushInt(20), // rest[0]
            Bytecode::PushInt(30), // rest[1]
            Bytecode::Call(3),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    let expected = Value::Array(FrostArray::from(vec![Value::Int(20), Value::Int(30)]));
    assert_eq!(result.tail(), &expected);
}

// ============================================================
// Nesting (trampoline depth > 1)
// ============================================================

#[test]
fn call_nested_closures() {
    // B: fn -> 5
    let b = func(
        vec![Bytecode::Pop, Bytecode::PushInt(5)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    // A: fn -> B()   (creates B as its own child, calls it, returns its result)
    let a = func(
        vec![
            Bytecode::Pop, // pop A's own function value
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(5));
}

// ============================================================
// Slot layout & argument ordering
// ============================================================

#[test]
fn call_closure_multiple_params_in_order() {
    // fn a, b -> a : returns the FIRST param. A reversed prelude or a naive
    // "top of stack" would yield b (20) instead of a (10).
    let callee = func(
        vec![
            Bytecode::DefLocal(1), // b (top) -> slot 1
            Bytecode::DefLocal(0), // a -> slot 0
            Bytecode::Pop,
            Bytecode::LoadLocal(0), // return a
        ],
        Arity::Exact(2),
        vec![entry("a", false), entry("b", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(10), // a
            Bytecode::PushInt(20), // b
            Bytecode::Call(2),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(10));
}

#[test]
fn call_closure_param_seats_after_captures() {
    // One capture (slot 0) + one param (slot 1). Returning the capture must yield
    // the captured value; getting the arg would mean the param clobbered slot 0.
    let callee = func(
        vec![
            Bytecode::DefLocal(1), // param -> slot 1, after the capture
            Bytecode::Pop,
            Bytecode::LoadLocal(0), // return the capture
        ],
        Arity::Exact(1),
        vec![entry("c", false), entry("x", false)], // slot 0 capture, slot 1 param
        vec![],
    );
    let program = func(
        vec![
            Bytecode::PushInt(100), // captured
            Bytecode::CreateClosure {
                num_captures: 1,
                function: 0,
            },
            Bytecode::PushInt(5), // arg
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(100));
}

#[test]
fn call_closure_captures_in_order() {
    // Two captures: the first pushed must land in slot 0.
    let callee = func(
        vec![Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(0),
        vec![entry("c0", false), entry("c1", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::PushInt(1), // capture 0
            Bytecode::PushInt(2), // capture 1
            Bytecode::CreateClosure {
                num_captures: 2,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(1));
}
