//! Happy-path tests for `Call` into a native function.
//!
//! Scope is LEAF natives only -- ones that read their args and return a value
//! without calling back into the VM. Re-entrancy (`NativeCtx::invoke`) does not
//! exist yet, so higher-order natives are out of scope, as are the error paths
//! (arity mismatch, non-callable) which still hit `todo!()`.
//!
//! A native is seated the representative way: built with `NativeFunction::new`,
//! wrapped in a `Value`, and injected into a top-level slot via `set_binding`.
//! Native bodies are plain Rust, so they need no arithmetic opcodes.

mod common;

use std::sync::Arc;

use common::{entry, fn_with_locals};
use frost_runtime::{Arity, Bytecode, FrostResult, NativeCtx, NativeFunction, ProgramResult, Value};

/// Build a native function `Value`.
fn native(
    name: &str,
    arity: Arity,
    f: impl Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static,
) -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(f, name, arity)))
}

/// Seat each `(name, value)` into a top-level slot (in order, so name `i` is slot
/// `i`), then run `code` to completion.
fn run_with_bindings(bindings: Vec<(&str, Value)>, code: Vec<Bytecode>) -> ProgramResult {
    let program = fn_with_locals(
        code,
        bindings.iter().map(|(name, _)| entry(name, false)).collect(),
    );
    let mut vm = frost_runtime::Vm::new(program).unwrap();
    for (name, value) in bindings {
        assert!(vm.set_binding(name, value));
    }
    vm.run().unwrap()
}

#[test]
fn native_returns_result() {
    // add(2, 3) -> 5, computed in Rust.
    let add = native("add", Arity::Exact(2), |_ctx, args| {
        Ok(Value::from(args[0].as_int().unwrap() + args[1].as_int().unwrap()))
    });
    let result = run_with_bindings(
        vec![("add", add)],
        vec![
            Bytecode::LoadLocal(0),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::Call(2),
        ],
    );
    assert_eq!(result.tail(), &Value::Int(5));
}

#[test]
fn native_receives_args_in_order() {
    // sub(10, 3) -> 7 ; order-sensitive, so a swapped arg slice would give -7.
    let sub = native("sub", Arity::Exact(2), |_ctx, args| {
        Ok(Value::from(args[0].as_int().unwrap() - args[1].as_int().unwrap()))
    });
    let result = run_with_bindings(
        vec![("sub", sub)],
        vec![
            Bytecode::LoadLocal(0),
            Bytecode::PushInt(10),
            Bytecode::PushInt(3),
            Bytecode::Call(2),
        ],
    );
    assert_eq!(result.tail(), &Value::Int(7));
}

#[test]
fn native_no_args() {
    let answer = native("answer", Arity::Exact(0), |_ctx, _args| Ok(Value::from(42i64)));
    let result = run_with_bindings(
        vec![("answer", answer)],
        vec![Bytecode::LoadLocal(0), Bytecode::Call(0)],
    );
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn native_variadic_receives_all_args() {
    // sum(...) over an AtLeast(0) native: every arg must reach the slice.
    let sum = native("sum", Arity::AtLeast(0), |_ctx, args| {
        let total: i64 = args.iter().map(|v| v.as_int().unwrap()).sum();
        Ok(Value::from(total))
    });
    let result = run_with_bindings(
        vec![("sum", sum)],
        vec![
            Bytecode::LoadLocal(0),
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::PushInt(4),
            Bytecode::Call(4),
        ],
    );
    assert_eq!(result.tail(), &Value::Int(10));
}

#[test]
fn native_call_leaves_exactly_one_value() {
    // A sentinel sits below the call. add(2, 3) consumes the function and both
    // args and leaves exactly one result; Pop reveals the sentinel -- `( f a.. -- r )`.
    let add = native("add", Arity::Exact(2), |_ctx, args| {
        Ok(Value::from(args[0].as_int().unwrap() + args[1].as_int().unwrap()))
    });
    let result = run_with_bindings(
        vec![("add", add)],
        vec![
            Bytecode::PushInt(99), // sentinel
            Bytecode::LoadLocal(0),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::Call(2),
            Bytecode::Pop, // discard the result (5)
        ],
    );
    assert_eq!(result.tail(), &Value::Int(99));
}

#[test]
fn native_result_feeds_native() {
    // inc(answer()) -> 43 : the value answer() returns must flow in as inc's arg.
    let answer = native("answer", Arity::Exact(0), |_ctx, _args| Ok(Value::from(42i64)));
    let inc = native("inc", Arity::Exact(1), |_ctx, args| {
        Ok(Value::from(args[0].as_int().unwrap() + 1))
    });
    let result = run_with_bindings(
        vec![("answer", answer), ("inc", inc)],
        vec![
            Bytecode::LoadLocal(1), // inc
            Bytecode::LoadLocal(0), // answer
            Bytecode::Call(0),      // answer() -> 42
            Bytecode::Call(1),      // inc(42) -> 43
        ],
    );
    assert_eq!(result.tail(), &Value::Int(43));
}
