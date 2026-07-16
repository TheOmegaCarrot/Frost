//! White-box tests for the one unwind invariant that is *not* observable through
//! the public API: recycling of `native_arg_pool` buffers on the error path.
//!
//! Recycling a buffer versus allocating a fresh one produces identical results,
//! so a black-box test cannot distinguish them; only a direct check of the
//! private pool can. (Operand-stack and frame-stack restoration *are* observable
//! and are covered black-box in `tests/vm_errors.rs`.) As a child of `vm`, this
//! module can read the private `Vm` internals directly.

use std::sync::Arc;

use super::*;

fn try_call_slot() -> usize {
    GLOBAL_NAMES
        .iter()
        .position(|&n| n == "try_call")
        .expect("try_call is a predefined global")
}

fn closure(idx: u32) -> Bytecode {
    Bytecode::CreateClosure {
        num_captures: 0,
        function: idx,
    }
}

fn func(
    name: &str,
    code: Vec<Bytecode>,
    num_captures: usize,
    name_table: Vec<NameEntry>,
    children: Vec<Arc<CompiledFunction>>,
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: name.to_string(),
        code,
        child_fns: children,
        constants: Vec::new(),
        name_table,
        num_captures,
        arity: Arity::Exact(0),
    })
}

/// `fn -> 1 / 0`. Built as a closure callee, so its body pops its own fn value.
fn fail_fn() -> Arc<CompiledFunction> {
    func(
        "fail",
        vec![
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::PushInt(0),
            Bytecode::Divide,
        ],
        0,
        vec![],
        vec![],
    )
}

/// `apply(f, ...rest)`: a re-entrant native that invokes `f` and propagates.
fn apply_native() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(
        "apply",
        Arity::AtLeast(1),
        |mut ctx, args| {
            let f = args[0].clone();
            ctx.invoke(&f, args[1..].iter_mut().map(Value::take))
        },
    )))
}

#[test]
fn catch_recycles_native_arg_buffer() {
    // try_call checks out a pooled buffer for its own args; after the caught
    // error it must be returned, so the pool is non-empty.
    let program = func(
        "main",
        vec![
            Bytecode::Pop, // pop the closure value the runner pushes
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        0,
        vec![],
        vec![fail_fn()],
    );
    let result = Vm::factory()
        .build(program.assert_trusted().into_closure().unwrap())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(
        result.0.native_arg_pool.len(),
        1,
        "try_call's arg buffer should be recycled"
    );
}

#[test]
fn catch_recycles_every_intermediate_buffer() {
    // try_call(apply, fail): two natives each check out a buffer (try_call's and apply's).
    // Both must be recycled across the caught error: one through `run_native`'s Ok arm (try_call), one through its Err arm (apply).
    let program = func(
        "main",
        vec![
            Bytecode::Pop,                         // pop the closure value the runner pushes
            Bytecode::LoadGlobal(try_call_slot()), //
            Bytecode::LoadLocal(0),                // apply (capture slot 0)
            closure(0),                            // fail
            Bytecode::Call(2),                     // try_call(apply, fail)
        ],
        1,
        vec![NameEntry {
            name: "apply".to_string(),
            exported: false,
        }],
        vec![fail_fn()],
    );
    let captures = BTreeMap::from([("apply".to_string(), apply_native())]);
    let result = Vm::factory()
        .build(program.assert_trusted().close(captures).unwrap())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(
        result.0.native_arg_pool.len(),
        2,
        "both intermediate arg buffers should be recycled"
    );
}
