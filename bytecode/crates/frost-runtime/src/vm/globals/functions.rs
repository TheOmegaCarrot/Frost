//! Higher-order function utilities: calling, error handling, and combinators.

use std::sync::Arc;

use crate::core::FrostResult;
use crate::{Arity, Bytecode, Closure, CompiledFunction, FrostArray, MapKey, NativeCtx, Value};

/// `call(f)` / `call(f, args)` -- invoke `f`, spreading the elements of `args` (or
/// no args) as a tail call. A hand-rolled `Between(1, 2)` closure over `DynTailCall`.
pub(super) fn call_global() -> Value {
    Value::Closure(Arc::new(Closure {
        captures: Vec::new(),
        function: Arc::new(CompiledFunction {
            name: "call".to_string(),
            arity: Arity::Between(1, 2),
            num_captures: 0,
            // Slot-free: an empty name_table means `push_closure_frame` allocates no
            // local_slots Vec. The body drops `call`'s own value with `DropBelow`
            // and lets `DynTailCall` validate the operands, so no slots are needed.
            name_table: Vec::new(),
            constants: Vec::new(),
            child_fns: Vec::new(),
            code: vec![
                // On entry: ( call_self f a? n ),
                // where n is the argc (1 or 2) pushed for a Between closure.

                // Normalize to ( call_self f arr ): make an empty array when no
                // second arg was supplied (n == 1).
                Bytecode::PushInt(1),
                Bytecode::CompareEqual,   // 1: n == 1 ? -> needEmpty
                Bytecode::JumpIfFalse(3), // 2: n == 2 -> a real array was passed (idx 6)
                Bytecode::Pop,            // 3: drop needEmpty
                Bytecode::MakeArray(0),   // 4: ( call_self f [] )
                Bytecode::Jump(1),        // 5: -> idx 7 (skip idx 6)
                Bytecode::Pop,            // 6: (have_arr) drop needEmpty -> ( call_self f a )
                // Drop call's own value (2 below the top) so the callee lands at
                // this frame's base, then hand ( f arr ) to DynTailCall, which
                // validates that arr is an Array and f is callable.
                Bytecode::DropBelow(2), // 7: ( f arr )
                Bytecode::DynTailCall,  // 8
            ],
        }),
    }))
}

/// Builds the `try_call` global -- Frost's catch primitive, surfaced as a native.
pub(super) fn try_call_global() -> Value {
    // At least the function to call; any further args are passed to it.
    Value::native("try_call", Arity::AtLeast(1), try_call)
}

/// `try_call(f, ...args)` -- invoke `f` with `args` and reify the outcome into a result map rather than letting an error propagate:
///   success: `{ ok: true,  value: <result> }`
///   failure: `{ ok: false, error: <message>, trace: [<frame names>] }`
///
/// This is "the native that declines to `?`": Frost's error model is uniform `?` propagation through native frames,
/// and `try_call` is the one place that catches the unwinding error instead of re-raising it.
/// By the time `invoke` returns Err, the boundary has already restored the Vm, so building the map here is safe.
fn try_call(mut ctx: NativeCtx<'_>, args: &mut [Value]) -> FrostResult {
    // Arity::AtLeast(1) guarantees args[0] exists.
    let function = args[0].clone();
    let call_args = args[1..].iter_mut().map(Value::take);

    match ctx.invoke(&function, call_args) {
        Ok(value) => Ok(result_map([
            (string_key("ok"), Value::Bool(true)),
            (string_key("value"), value),
        ])),
        Err(err) => {
            let trace = err
                .backtrace
                .into_iter()
                .map(Value::from)
                .collect::<Vec<_>>();
            Ok(result_map([
                (string_key("ok"), Value::Bool(false)),
                (string_key("error"), Value::from(err.message)),
                (string_key("trace"), Value::Array(FrostArray::from(trace))),
            ]))
        }
    }
}

/// A string `MapKey` from a `&str` literal.
fn string_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s.as_bytes()))
}

/// Build a `Value::Map` from a fixed set of entries.
fn result_map<const N: usize>(entries: [(MapKey, Value); N]) -> Value {
    Value::Map(entries.into_iter().collect())
}

// --- Not yet implemented ---

pub(super) fn error_global() -> Value {
    super::stub("error")
}

pub(super) fn and_then_global() -> Value {
    super::stub("and_then")
}

pub(super) fn or_else_global() -> Value {
    super::stub("or_else")
}

pub(super) fn inv_global() -> Value {
    super::stub("inv")
}

pub(super) fn curry_global() -> Value {
    super::stub("curry")
}

pub(super) fn bcurry_global() -> Value {
    super::stub("bcurry")
}

pub(super) fn collect_global() -> Value {
    super::stub("collect")
}

pub(super) fn spread_global() -> Value {
    super::stub("spread")
}

pub(super) fn rev_args_global() -> Value {
    super::stub("rev_args")
}

pub(super) fn tap_global() -> Value {
    super::stub("tap")
}

pub(super) fn const_global() -> Value {
    super::stub("const")
}

pub(super) fn compose_global() -> Value {
    super::stub("compose")
}
