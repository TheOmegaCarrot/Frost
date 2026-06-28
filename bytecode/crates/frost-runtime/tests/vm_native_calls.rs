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

use std::collections::BTreeMap;
use std::sync::Arc;

use common::{entry, func};
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostArray, FrostResult, NameEntry, NativeCtx,
    NativeFunction, ProgramResult, Value, Vm,
};

/// Build a native function `Value`.
fn native(
    name: &str,
    arity: Arity,
    f: impl Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static,
) -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(f, name, arity)))
}

/// Build a top-level closure whose captures are `bindings` (host-provided values,
/// in order so name `i` is capture slot `i`) plus `children`, then run `code`.
fn run_with(
    bindings: Vec<(&str, Value)>,
    children: Vec<Arc<CompiledFunction>>,
    code: Vec<Bytecode>,
) -> ProgramResult {
    let names: Vec<NameEntry> = bindings
        .iter()
        .map(|(name, _)| entry(name, false))
        .collect();
    let num_captures = names.len();
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        name: "<test>".to_string(),
        code: body,
        child_fns: children,
        constants: Vec::new(),
        name_table: names,
        num_captures,
        arity: Arity::Exact(0),
    });
    let captures: BTreeMap<String, Value> = bindings
        .into_iter()
        .map(|(n, v)| (n.to_string(), v))
        .collect();
    let closure = program.close(captures).expect("all captures provided");
    Vm::new(closure).unwrap().run().unwrap()
}

/// Seat each `(name, value)` as a capture (in order, so name `i` is slot `i`),
/// then run `code` to completion.
fn run_with_bindings(bindings: Vec<(&str, Value)>, code: Vec<Bytecode>) -> ProgramResult {
    run_with(bindings, Vec::new(), code)
}

#[test]
fn native_returns_result() {
    // add(2, 3) -> 5, computed in Rust.
    let add = native("add", Arity::Exact(2), |_ctx, args| {
        Ok(Value::from(
            args[0].as_int().unwrap() + args[1].as_int().unwrap(),
        ))
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
        Ok(Value::from(
            args[0].as_int().unwrap() - args[1].as_int().unwrap(),
        ))
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
    let answer = native("answer", Arity::Exact(0), |_ctx, _args| {
        Ok(Value::from(42i64))
    });
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
        Ok(Value::from(
            args[0].as_int().unwrap() + args[1].as_int().unwrap(),
        ))
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
    let answer = native("answer", Arity::Exact(0), |_ctx, _args| {
        Ok(Value::from(42i64))
    });
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

// ============================================================
// Re-entrancy: NativeCtx::invoke (native -> native, native -> closure)
// ============================================================

/// Like `run_with_bindings`, but the top-level also carries `children` reachable
/// via `CreateClosure`, so a native can be handed a closure to invoke.
fn run_native_program(
    bindings: Vec<(&str, Value)>,
    children: Vec<Arc<CompiledFunction>>,
    code: Vec<Bytecode>,
) -> ProgramResult {
    run_with(bindings, children, code)
}

/// `apply(f, ...rest)` -- a native that invokes `f` with the rest of its args,
/// stealing them. Drives `NativeCtx::invoke` for either a native or closure `f`.
fn apply_native() -> Value {
    native("apply", Arity::AtLeast(1), |mut ctx, args| {
        let f = args[0].clone();
        ctx.invoke(
            &f,
            args[1..]
                .iter_mut()
                .map(|v| std::mem::replace(v, Value::Null)),
        )
    })
}

#[test]
fn native_invokes_native() {
    // apply(inc, 5) -> invoke(inc, [5]) -> 6
    let inc = native("inc", Arity::Exact(1), |_ctx, args| {
        Ok(Value::from(args[0].as_int().unwrap() + 1))
    });
    let result = run_native_program(
        vec![("apply", apply_native()), ("inc", inc)],
        vec![],
        vec![
            Bytecode::LoadLocal(0), // apply
            Bytecode::LoadLocal(1), // inc
            Bytecode::PushInt(5),
            Bytecode::Call(2),
        ],
    );
    assert_eq!(result.tail(), &Value::Int(6));
}

#[test]
fn native_invokes_closure() {
    // apply(identity, 7) -> invoke -> 7 ; proves the arg flows native -> closure.
    let identity = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let result = run_native_program(
        vec![("apply", apply_native())],
        vec![identity],
        vec![
            Bytecode::LoadLocal(0),
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(7),
            Bytecode::Call(2),
        ],
    );
    assert_eq!(result.tail(), &Value::Int(7));
}

#[test]
fn native_invokes_closure_multiple_args() {
    // fn a, b -> a ; apply(first, 10, 20) -> invoke(first, [10, 20]) -> 10
    let first = func(
        vec![
            Bytecode::DefLocal(1),
            Bytecode::DefLocal(0),
            Bytecode::Pop,
            Bytecode::LoadLocal(0),
        ],
        Arity::Exact(2),
        vec![entry("a", false), entry("b", false)],
        vec![],
    );
    let result = run_native_program(
        vec![("apply", apply_native())],
        vec![first],
        vec![
            Bytecode::LoadLocal(0),
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(10),
            Bytecode::PushInt(20),
            Bytecode::Call(3),
        ],
    );
    assert_eq!(result.tail(), &Value::Int(10));
}

#[test]
fn native_invokes_zero_arg_closure() {
    // fn -> 99 ; apply(const) -> invoke(const, []) -> 99
    let const99 = func(
        vec![Bytecode::Pop, Bytecode::PushInt(99)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let result = run_native_program(
        vec![("apply", apply_native())],
        vec![const99],
        vec![
            Bytecode::LoadLocal(0),
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(1),
        ],
    );
    assert_eq!(result.tail(), &Value::Int(99));
}

#[test]
fn native_invokes_variadic_closure() {
    // fn ...rest -> rest ; apply(rest, 1, 2, 3) -> invoke(rest, [1,2,3]) -> [1,2,3]
    let rest = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::AtLeast(0),
        vec![entry("rest", false)],
        vec![],
    );
    let result = run_native_program(
        vec![("apply", apply_native())],
        vec![rest],
        vec![
            Bytecode::LoadLocal(0),
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::Call(4),
        ],
    );
    let expected = Value::Array(FrostArray::from(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ]));
    assert_eq!(result.tail(), &expected);
}

#[test]
fn nested_native_invoke() {
    // apply(apply, inc, 5) -> invoke(apply, [inc, 5]) -> apply(inc, 5)
    //   -> invoke(inc, [5]) -> 6. Exercises invoke nesting (pool + frame depth).
    let inc = native("inc", Arity::Exact(1), |_ctx, args| {
        Ok(Value::from(args[0].as_int().unwrap() + 1))
    });
    let result = run_native_program(
        vec![("apply", apply_native()), ("inc", inc)],
        vec![],
        vec![
            Bytecode::LoadLocal(0), // apply
            Bytecode::LoadLocal(0), // apply
            Bytecode::LoadLocal(1), // inc
            Bytecode::PushInt(5),
            Bytecode::Call(3),
        ],
    );
    assert_eq!(result.tail(), &Value::Int(6));
}
