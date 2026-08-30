//! Happy-path tests for `Call` into a VM closure.
//!
//! Contract under test: a function and its arguments sit on the stack, the call
//! consumes them, and exactly one value is left behind: `( f a1..an -- r )`.
//!
//! Closures can only be obtained via `CreateClosure` (their fields are private),
//! so every test builds a parent function whose `child_fns[0]` is the callee and
//! whose body creates the closure, pushes args, and calls it. Bodies stay on the
//! happy path; error flows are covered in `vm_errors.rs`.
//!
//! The canonical fixed-arity callee is the identity `fn x -> x`, whose body is a
//! minimal "-O0" prelude: move the arg into a slot, pop the function value, load
//! the arg back as the result.

mod common;

use common::{entry, func, func_with_captures, run_fn};
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
            Bytecode::CreateClosure(0),
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
    // exactly one result on top; popping it reveals the sentinel, proving
    // `( f a.. -- r )` and that the caller resumed to run the trailing Pop.
    let callee = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::PushInt(7), // sentinel
            Bytecode::CreateClosure(0),
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
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure(0),
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
    let callee = func_with_captures(
        vec![
            Bytecode::Pop,          // pop the function value
            Bytecode::LoadLocal(0), // the capture
        ],
        Arity::Exact(0),
        vec![entry("c", false)], // slot 0 holds the capture
        vec![],
        1,
    );
    let program = func(
        vec![
            Bytecode::PushInt(123), // value to capture
            Bytecode::CreateClosure(0),
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
            Bytecode::CreateClosure(0),
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
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::AtLeast(0),
        vec![entry("rest", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure(0),
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
            Bytecode::CreateClosure(0),
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
            Bytecode::CreateClosure(0),
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure(0),
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
            Bytecode::CreateClosure(0),
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
    let callee = func_with_captures(
        vec![
            Bytecode::DefLocal(1), // param -> slot 1, after the capture
            Bytecode::Pop,
            Bytecode::LoadLocal(0), // return the capture
        ],
        Arity::Exact(1),
        vec![entry("c", false), entry("x", false)], // slot 0 capture, slot 1 param
        vec![],
        1,
    );
    let program = func(
        vec![
            Bytecode::PushInt(100), // captured
            Bytecode::CreateClosure(0),
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
    let callee = func_with_captures(
        vec![Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(0),
        vec![entry("c0", false), entry("c1", false)],
        vec![],
        2,
    );
    let program = func(
        vec![
            Bytecode::PushInt(1), // capture 0
            Bytecode::PushInt(2), // capture 1
            Bytecode::CreateClosure(0),
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(1));
}

// ============================================================
// Variadic + captures (full slot math: captures, then fixed, then rest)
// ============================================================

#[test]
fn call_variadic_closure_with_capture() {
    // fn x, ...rest -> rest, with one capture.
    // Slots: 0 = capture, 1 = x (fixed), 2 = rest. The vararg collapse index
    // (base + 1 + fixed_argc) must ignore the capture (captures are not on the
    // stack and not counted in arity); otherwise `rest` comes out wrong.
    let callee = func_with_captures(
        vec![
            Bytecode::DefLocal(2),  // rest array (top) -> slot 2
            Bytecode::DefLocal(1),  // x -> slot 1
            Bytecode::Pop,          // pop the function value
            Bytecode::LoadLocal(2), // return rest
        ],
        Arity::AtLeast(1),
        vec![entry("c", false), entry("x", false), entry("rest", false)],
        vec![],
        1,
    );
    let program = func(
        vec![
            Bytecode::PushInt(99), // captured
            Bytecode::CreateClosure(0),
            Bytecode::PushInt(10), // x
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
// Reuse, composition, deeper nesting
// ============================================================

#[test]
fn call_closure_reused_via_dup() {
    // Dup a closure and call it twice. `Call` clones the function value out, so
    // the first call must not destroy the shared `Arc` the second call uses.
    let callee = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure(0),
            Bytecode::Dup,
            Bytecode::PushInt(5),
            Bytecode::Call(1), // call the dup with 5
            Bytecode::Pop,     // discard 5
            Bytecode::PushInt(6),
            Bytecode::Call(1), // call the original with 6
        ],
        Arity::Exact(0),
        vec![],
        vec![callee],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(6));
}

#[test]
fn call_closure_ignores_unused_param() {
    // fn x -> 42 : the prelude must still consume the arg even though the body
    // never reads it, or a stray value would corrupt the result slot.
    let callee = func(
        vec![
            Bytecode::DefLocal(0), // consume the arg (unused)
            Bytecode::Pop,
            Bytecode::PushInt(42),
        ],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure(0),
            Bytecode::PushInt(5),
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
fn call_result_feeds_next_call() {
    // f(g(5)) where g = (fn x -> 42) and f = identity. The 42 returned by g must
    // flow in as f's argument, so the final result is 42 (not 5).
    let f = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let g = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::PushInt(42)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure(0), // f (child 0)
            Bytecode::CreateClosure(1), // g (child 1)
            Bytecode::PushInt(5),
            Bytecode::Call(1), // g(5) -> 42
            Bytecode::Call(1), // f(42) -> 42
        ],
        Arity::Exact(0),
        vec![],
        vec![f, g],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn call_deeply_nested_closures() {
    // top -> A -> B -> C, C returns 7. Trampoline must floor correctly at depth 3.
    let c = func(
        vec![Bytecode::Pop, Bytecode::PushInt(7)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let b = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure(0),
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![c],
    );
    let a = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure(0),
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure(0),
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(7));
}
