//! Tests for the `call` global -- the hand-rolled `Between(1, 2)` closure that
//! normalizes its args to an array, type-checks, and hands off to `DynTailCall`.
//!
//! `call(f)`        -> f()
//! `call(f, array)` -> f(*array)
//! with type errors for a non-Function `f` or non-Array second arg, and an arity
//! error outside 1..=2.

use std::collections::BTreeMap;
use std::sync::Arc;

mod common;

use common::global_slot;
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostArray, FrostError, FrostResult, NameEntry,
    NativeFunction, ProgramResult, Value, Vm,
};

use Bytecode::*;

fn call_slot() -> usize {
    global_slot("call")
}

fn entry(name: &str) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported: false,
    }
}

/// A standalone (no-capture) closure value.
fn func(name: &str, code: Vec<Bytecode>, arity: Arity, names: &[&str]) -> Value {
    let f = Arc::new(CompiledFunction {
        name: name.to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names.iter().map(|n| entry(n)).collect(),
        num_captures: 0,
        arity,
    });
    Value::Closure(f.assert_trusted().into_closure().unwrap())
}

/// Build and run a top-level "main" that, after popping its own value, runs `body`.
/// `caps` are seated as captures (slot `i` is the i-th entry).
fn run_main(caps: Vec<(&str, Value)>, body: Vec<Bytecode>) -> Result<ProgramResult, FrostError> {
    let names: Vec<NameEntry> = caps.iter().map(|(n, _)| entry(n)).collect();
    let mut code = vec![Pop];
    code.extend(body);
    let main = Arc::new(CompiledFunction {
        name: "main".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names,
        num_captures: caps.len(),
        arity: Arity::Exact(0),
    });
    let map: BTreeMap<String, Value> = caps.into_iter().map(|(n, v)| (n.to_string(), v)).collect();
    Vm::factory().build(main.assert_trusted().close(map).unwrap()).unwrap().run().map_err(|e| e.into_error())
}

fn add() -> Value {
    func(
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
    )
}

// ============================================================
// Happy paths
// ============================================================

#[test]
fn call_spreads_array_into_function() {
    // call(add, [10, 20]) -> 30
    let r = run_main(
        vec![("f", add())],
        vec![
            LoadGlobal(call_slot()),
            LoadLocal(0),
            PushInt(10),
            PushInt(20),
            MakeArray(2),
            Call(2),
        ],
    )
    .unwrap();
    assert_eq!(r.tail(), &Value::Int(30));
}

#[test]
fn call_with_explicit_empty_array_calls_with_no_args() {
    // call(const99, []) -> 99
    let c = func("const99", vec![Pop, PushInt(99)], Arity::Exact(0), &[]);
    let r = run_main(
        vec![("f", c)],
        vec![LoadGlobal(call_slot()), LoadLocal(0), MakeArray(0), Call(2)],
    )
    .unwrap();
    assert_eq!(r.tail(), &Value::Int(99));
}

#[test]
fn call_one_arg_form_supplies_an_empty_array() {
    // call(const99) -> 99 ; the missing second arg becomes an empty array.
    let c = func("const99", vec![Pop, PushInt(99)], Arity::Exact(0), &[]);
    let r = run_main(
        vec![("f", c)],
        vec![LoadGlobal(call_slot()), LoadLocal(0), Call(1)],
    )
    .unwrap();
    assert_eq!(r.tail(), &Value::Int(99));
}

#[test]
fn call_preserves_argument_order() {
    // call(sub, [10, 3]) -> 7, not -7 (non-commutative op pins the spread order).
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
    let r = run_main(
        vec![("f", sub)],
        vec![
            LoadGlobal(call_slot()),
            LoadLocal(0),
            PushInt(10),
            PushInt(3),
            MakeArray(2),
            Call(2),
        ],
    )
    .unwrap();
    assert_eq!(r.tail(), &Value::Int(7));
}

#[test]
fn call_spreads_into_a_native() {
    // call(sum, [1, 2, 3]) -> 6 ; the native receives the spread args as a slice.
    let sum = Value::NativeFunction(Arc::new(NativeFunction::new(
        "sum",
        Arity::AtLeast(0),
        |_ctx, args: &mut [Value]| -> FrostResult {
            Ok(Value::Int(args.iter().map(|v| v.as_int().unwrap()).sum()))
        },
    )));
    let r = run_main(
        vec![("f", sum)],
        vec![
            LoadGlobal(call_slot()),
            LoadLocal(0),
            PushInt(1),
            PushInt(2),
            PushInt(3),
            MakeArray(3),
            Call(2),
        ],
    )
    .unwrap();
    assert_eq!(r.tail(), &Value::Int(6));
}

#[test]
fn call_passes_a_null_argument_through() {
    // call(id, [null]) -> null ; null is a real value spread as the single arg.
    let id = func(
        "id",
        vec![DefLocal(0), Pop, LoadLocal(0)],
        Arity::Exact(1),
        &["x"],
    );
    let r = run_main(
        vec![("f", id)],
        vec![
            LoadGlobal(call_slot()),
            LoadLocal(0),
            PushNull,
            MakeArray(1),
            Call(2),
        ],
    )
    .unwrap();
    assert_eq!(r.tail(), &Value::Null);
}

#[test]
fn call_leaves_a_balanced_stack() {
    // [call(c1), call(c2)] must be [1, 2]. If `call` leaked its own closure value
    // onto the stack, MakeArray would grab the leaked value instead of result 1.
    let c1 = func("c1", vec![Pop, PushInt(1)], Arity::Exact(0), &[]);
    let c2 = func("c2", vec![Pop, PushInt(2)], Arity::Exact(0), &[]);
    let r = run_main(
        vec![("c1", c1), ("c2", c2)],
        vec![
            LoadGlobal(call_slot()),
            LoadLocal(0),
            Call(1), // call(c1) -> 1
            LoadGlobal(call_slot()),
            LoadLocal(1),
            Call(1), // call(c2) -> 2
            MakeArray(2),
        ],
    )
    .unwrap();
    assert_eq!(
        r.tail(),
        &Value::Array(FrostArray::from(vec![Value::Int(1), Value::Int(2)]))
    );
}

// ============================================================
// Errors
// ============================================================

#[test]
fn call_non_array_second_arg_is_type_error() {
    // call(add, 5) -- the second arg must be an Array.
    let err = run_main(
        vec![("f", add())],
        vec![LoadGlobal(call_slot()), LoadLocal(0), PushInt(5), Call(2)],
    )
    .unwrap_err();
    assert!(err.message.contains("Array"), "got: {}", err.message);
}

#[test]
fn call_non_function_first_arg_is_type_error() {
    // call(5, []) -- the first arg must be callable. The error now comes from
    // DynTailCall's `not_callable`, which names the offending type.
    let err = run_main(
        vec![],
        vec![LoadGlobal(call_slot()), PushInt(5), MakeArray(0), Call(2)],
    )
    .unwrap_err();
    assert!(err.message.contains("non-function"), "got: {}", err.message);
    assert!(err.message.contains("Int"), "got: {}", err.message);
}

#[test]
fn call_with_no_args_is_arity_error() {
    // call() -- below the Between(1, 2) lower bound.
    let err = run_main(vec![], vec![LoadGlobal(call_slot()), Call(0)]).unwrap_err();
    assert!(
        err.message.contains("between 1 and 2"),
        "got: {}",
        err.message
    );
}

#[test]
fn call_with_too_many_args_is_arity_error() {
    // call(add, 1, 2) -- above the Between(1, 2) upper bound (caught before the body).
    let err = run_main(
        vec![("f", add())],
        vec![
            LoadGlobal(call_slot()),
            LoadLocal(0),
            PushInt(1),
            PushInt(2),
            Call(3),
        ],
    )
    .unwrap_err();
    assert!(
        err.message.contains("between 1 and 2"),
        "got: {}",
        err.message
    );
}

// ============================================================
// Double TCO: loop recurses *through* `call`
// ============================================================

#[test]
fn call_drives_a_recursive_loop() {
    // loop(self, n) -> if n <= 0: n else: call(self, [self, n - 1])
    // The recursive step tail-calls `call`, which tail-calls `self` -- two TCO hops
    // per iteration. A deep count completing proves the loop runs through `call`
    // without growing without bound.
    let loop_fn = func(
        "loop",
        vec![
            DefLocal(1),             // 0: n
            DefLocal(0),             // 1: self
            Pop,                     // 2: own fn value
            LoadLocal(1),            // 3: n
            PushInt(0),              // 4
            CompareLessThanOrEqual,  // 5: n <= 0 ?
            JumpIfTrue(9),           // 6: -> base (idx 16)
            Pop,                     // 7: recurse: drop the comparison bool
            LoadGlobal(call_slot()), // 8: call
            LoadLocal(0),            // 9: self (the function call() will invoke)
            LoadLocal(0),            // 10: self (first array element)
            LoadLocal(1),            // 11: n
            PushInt(1),              // 12
            Subtract,                // 13: n - 1
            MakeArray(2),            // 14: [self, n - 1]
            TailCall(2),             // 15: tail-call call(self, [self, n - 1])
            Pop,                     // 16: base: drop the comparison bool
            LoadLocal(1),            // 17: return n
        ],
        Arity::Exact(2),
        &["self", "n"],
    );
    let r = run_main(
        vec![("loop", loop_fn)],
        vec![
            LoadLocal(0),     // loop (function)
            LoadLocal(0),     // self = loop
            PushInt(100_000), // n
            Call(2),          // loop(loop, 100_000)
        ],
    )
    .unwrap();
    assert_eq!(r.tail(), &Value::Int(0));
}
