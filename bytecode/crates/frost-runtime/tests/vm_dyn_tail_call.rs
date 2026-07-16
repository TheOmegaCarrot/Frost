//! Tests for the `DynTailCall` opcode: spread the args array on top of the stack
//! and tail-call the function beneath it: `( f arr -- r )`.
//!
//! These drive the opcode directly via hand-authored bytecode; the `call` global
//! that sits on top of it is covered in `vm_call.rs`. The frame-reuse / TCO path is
//! shared with `TailCall` (covered in `vm_tail_calls`), so these focus on the part
//! unique to `DynTailCall`: exploding the array and dispatching on the count.

use std::collections::BTreeMap;
use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostResult, NameEntry, NativeFunction,
    Value, Vm,
};

use Bytecode::*;

fn entry(name: &str) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported: false,
    }
}

/// A standalone (no-capture) closure function.
fn func(name: &str, code: Vec<Bytecode>, arity: Arity, names: &[&str]) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: name.to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names.iter().map(|n| entry(n)).collect(),
        num_captures: 0,
        arity,
    })
}

/// Run a top-level that captures `f` (slot 0) and executes `body`, returning the tail.
fn run_with_f(f: Value, body: Vec<Bytecode>) -> Value {
    let top = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<top>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: vec![entry("f")],
        num_captures: 1,
        arity: Arity::Exact(0),
    });
    let closure = top.assert_trusted().close(BTreeMap::from([("f".to_string(), f)])).unwrap();
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .unwrap()
        .tail()
        .clone()
}

#[test]
fn dyn_tail_call_spreads_array_into_closure() {
    // add = fn a, b -> a + b ; DynTailCall(add, [10, 20]) -> 30.
    let add = func(
        "add",
        vec![
            DefLocal(1),
            DefLocal(0),
            Pop,
            LoadLocal(0),
            LoadLocal(1),
            Add,
        ],
        Arity::Exact(2),
        &["a", "b"],
    );
    let add = Value::Closure(add.assert_trusted().into_closure().unwrap());
    let result = run_with_f(
        add,
        vec![
            Pop,
            LoadLocal(0), // f = add
            PushInt(10),
            PushInt(20),
            MakeArray(2), // args array [10, 20]
            DynTailCall,
        ],
    );
    assert_eq!(result, Value::Int(30));
}

#[test]
fn dyn_tail_call_spreads_array_into_native() {
    // A native callee takes the spread args as a slice; the count is dynamic.
    let sum = Value::NativeFunction(Arc::new(NativeFunction::new(
        "sum",
        Arity::AtLeast(0),
        |_ctx, args: &mut [Value]| -> FrostResult {
            Ok(Value::Int(args.iter().map(|v| v.as_int().unwrap()).sum()))
        },
    )));
    let result = run_with_f(
        sum,
        vec![
            Pop,
            LoadLocal(0), // f = sum
            PushInt(1),
            PushInt(2),
            PushInt(3),
            MakeArray(3),
            DynTailCall,
        ],
    );
    assert_eq!(result, Value::Int(6));
}

#[test]
fn dyn_tail_call_with_empty_array_calls_with_no_args() {
    // const99 = fn -> 99 ; DynTailCall(const99, []) -> 99 (argc 0 from an empty spread).
    let const99 = func("const99", vec![Pop, PushInt(99)], Arity::Exact(0), &[]);
    let const99 = Value::Closure(const99.assert_trusted().into_closure().unwrap());
    let result = run_with_f(const99, vec![Pop, LoadLocal(0), MakeArray(0), DynTailCall]);
    assert_eq!(result, Value::Int(99));
}

#[test]
fn dyn_tail_call_preserves_argument_order() {
    // sub = fn a, b -> a - b ; a non-commutative op proves the spread keeps order.
    let sub = func(
        "sub",
        vec![
            DefLocal(1),
            DefLocal(0),
            Pop,
            LoadLocal(0),
            LoadLocal(1),
            Subtract,
        ],
        Arity::Exact(2),
        &["a", "b"],
    );
    let sub = Value::Closure(sub.assert_trusted().into_closure().unwrap());
    let result = run_with_f(
        sub,
        vec![
            Pop,
            LoadLocal(0),
            PushInt(10),
            PushInt(3),
            MakeArray(2), // [10, 3]
            DynTailCall,
        ],
    );
    assert_eq!(result, Value::Int(7)); // 10 - 3, not 3 - 10
}
