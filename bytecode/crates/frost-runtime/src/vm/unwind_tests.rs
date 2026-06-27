//! White-box checks that integration tests cannot reach: after an error is
//! caught, the operand stack, the frame stack, and the native-arg pool must all
//! be restored. As a child of `vm`, this module can assert the private `Vm`
//! internals directly.

use std::sync::Arc;

use super::*;

fn try_call_slot() -> usize {
    GlobalSet::defaults()
        .index_of("try_call")
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
    name_table: Vec<NameEntry>,
    children: Vec<Arc<CompiledFunction>>,
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: name.to_string(),
        code,
        child_fns: children,
        constants: Vec::new(),
        name_table,
        arity: Arity::Exact(0),
    })
}

/// `fn -> 1 / 0`
fn fail_fn() -> Arc<CompiledFunction> {
    func(
        "fail",
        vec![
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::PushInt(0),
            Bytecode::Divide,
        ],
        vec![],
        vec![],
    )
}

/// `apply(f, ...rest)` -- a re-entrant native that invokes `f` and propagates.
fn apply_native() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(
        |mut ctx, args| {
            let f = args[0].clone();
            ctx.invoke(
                &f,
                args[1..].iter_mut().map(|v| std::mem::replace(v, Value::Null)),
            )
        },
        "apply",
        Arity::AtLeast(1),
    )))
}

#[test]
fn catch_restores_operand_stack_and_frames() {
    // try_call(fail): after the catch, exactly the result map is left on the
    // operand stack, and only the base frame remains.
    let program = func(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        vec![],
        vec![fail_fn()],
    );
    let result = Vm::new(program).unwrap().run().unwrap();
    let vm = &result.0;
    assert_eq!(vm.stack.len(), 1, "exactly the result map should remain");
    assert!(
        vm.stack[0].as_map().is_some(),
        "the surviving value should be the result map"
    );
    assert_eq!(vm.stack_frames.len(), 1, "only the base frame should remain");
}

#[test]
fn catch_recycles_native_arg_buffer() {
    // try_call checks out a pooled buffer for its own args; after the call it must
    // be returned, so the pool is non-empty.
    let program = func(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        vec![],
        vec![fail_fn()],
    );
    let result = Vm::new(program).unwrap().run().unwrap();
    assert_eq!(
        result.0.native_arg_pool.len(),
        1,
        "try_call's arg buffer should be recycled"
    );
}

#[test]
fn catch_recycles_every_intermediate_buffer() {
    // try_call(apply, fail): two natives each check out a buffer (try_call's and
    // apply's). Both must be recycled across the caught error -- one through
    // `run_native`'s Ok arm (try_call), one through its Err arm (apply).
    let program = func(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            Bytecode::LoadLocal(0), // apply
            closure(0),             // fail
            Bytecode::Call(2),      // try_call(apply, fail)
        ],
        vec![NameEntry {
            name: "apply".to_string(),
            exported: false,
        }],
        vec![fail_fn()],
    );
    let mut vm = Vm::new(program).unwrap();
    assert!(vm.set_binding("apply", apply_native()));
    let result = vm.run().unwrap();
    assert_eq!(
        result.0.native_arg_pool.len(),
        2,
        "both intermediate arg buffers should be recycled"
    );
    assert_eq!(result.0.stack.len(), 1);
    assert_eq!(result.0.stack_frames.len(), 1);
}
