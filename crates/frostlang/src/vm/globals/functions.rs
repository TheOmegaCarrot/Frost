//! Higher-order function utilities: calling, error handling, and combinators.

use std::sync::Arc;

use crate::bytecode::{Bytecode, CompiledFunction, FormatVersion};
use crate::core::FrostResult;
use crate::native::{Function, Optional, caught};
use crate::{Arity, Closure, FrostArray, FrostType, NativeCtx, Param, Params, Value, frost_fn};

/// `call(f)` / `call(f, args)`: invoke `f`, spreading the elements of `args` (or
/// no args) as a tail call. A hand-rolled `Between(1, 2)` closure over `DynTailCall`.
pub(super) fn call_global() -> Value {
    Value::Closure(Arc::new(Closure {
        captures: Vec::new(),
        function: Arc::new(CompiledFunction {
            version: FormatVersion,
            name: "call".to_string(),
            origin: None,
            arity: Arity::Between(1, 2),
            num_captures: 0,
            // Slot-free: an empty name_table gives the frame no local slots. The body
            // drops `call`'s own value with `DropBelow`, so no slots are needed.
            name_table: Vec::new(),
            constants: vec![call_type_error()],
            key_constants: Vec::new(),
            child_fns: Vec::new(),
            debug_info: None,
            code: vec![
                // On entry: ( call_self f a? n ),
                // where n is the argc (1 or 2) pushed for a Between closure.

                // Normalize to ( call_self f arr ): make an empty array when no
                // second arg was supplied (n == 1).
                Bytecode::PushInt(1),
                Bytecode::CompareEqual,       // 1: n == 1 ? -> needEmpty
                Bytecode::PeekJumpIfFalse(3), // 2: n == 2 -> a real array was passed (idx 6)
                Bytecode::Pop,                // 3: drop needEmpty
                Bytecode::MakeArray(0),       // 4: ( call_self f [] )
                Bytecode::Jump(1),            // 5: -> idx 7 (skip idx 6)
                Bytecode::Pop,                // 6: (have_arr) drop needEmpty -> ( call_self f a )
                // Type-check ( f arr ): when both are right, skip to idx 18.
                Bytecode::PeekDown(1), // 7: ( call_self f arr f )
                Bytecode::TypeTest(FrostType::FUNCTION), // 8
                Bytecode::JumpIfFalse(3), // 9: not a Function -> idx 13
                Bytecode::PeekDown(0), // 10: ( call_self f arr arr )
                Bytecode::TypeTest(FrostType::ARRAY), // 11
                Bytecode::JumpIfTrue(5), // 12: an Array -> idx 18
                // A wrong type: raise the error naming `call` from this frame.
                Bytecode::LoadConst(0), // 13: ( call_self f arr type_error )
                Bytecode::PeekDown(2),  // 14
                Bytecode::PeekDown(2),  // 15: ( call_self f arr type_error f arr )
                Bytecode::Call(2),      // 16: ( call_self f arr error )
                Bytecode::ProduceError, // 17: raises
                // Drop call's own value (2 below the top) so the callee lands at
                // this frame's base, then hand ( f arr ) to DynTailCall.
                Bytecode::DropBelow(2), // 18: ( f arr )
                Bytecode::DynTailCall,  // 19
            ],
        }),
    }))
}

/// The native giving the error for `call`'s arguments having the wrong types
/// (see `type_error_native`).
fn call_type_error() -> Value {
    const PARAMS: Params =
        Params::new(&[Param::of(FrostType::FUNCTION), Param::of(FrostType::ARRAY)]);
    crate::vm::globals::type_error_native("call", PARAMS)
}

/// `try_call(f, args)`: invoke `f` with `args` (an array) and reify the outcome into a result map
/// rather than letting an error propagate, shaped as [`caught`] shapes it.
///
/// The argument array is interpreted the same as `call`, only the result shape differs.
/// This is Frost's catch primitive, surfaced as a native.
#[frost_fn]
pub(super) fn try_call(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    #[frost(unnamed)] function: Function,
    args: Optional<FrostArray>,
) -> Value {
    let call_args = args.0.unwrap_or_else(FrostArray::empty);
    caught(ctx.invoke(&function, call_args.into_vec()))
}

pub(super) fn error_global() -> Value {
    // ProduceError consumes the top value and raises it: identical to `from_value`.
    super::bytecode_global(
        "error",
        Arity::Exact(1),
        vec![Bytecode::DropBelow(1), Bytecode::ProduceError],
    )
}

#[frost_fn]
pub(super) fn and_then(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    #[frost(unnamed)] value: Value,
    #[frost(unnamed)] function: Function,
) -> FrostResult {
    match value {
        Value::Null => Ok(Value::Null),
        _ => ctx.invoke(&function, [value]),
    }
}

#[frost_fn]
pub(super) fn or_else(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    #[frost(unnamed)] value: Value,
    #[frost(unnamed)] function: Function,
) -> FrostResult {
    match value {
        Value::Null => ctx.invoke(&function, []),
        _ => Ok(value),
    }
}

pub(super) fn inv_global() -> Value {
    super::generated::global("inv")
}

pub(super) fn curry_global() -> Value {
    super::generated::global("curry")
}

pub(super) fn bcurry_global() -> Value {
    super::generated::global("bcurry")
}

pub(super) fn collect_global() -> Value {
    super::generated::global("collect")
}

pub(super) fn spread_global() -> Value {
    super::generated::global("spread")
}

pub(super) fn rev_args_global() -> Value {
    super::generated::global("rev_args")
}

pub(super) fn tap_global() -> Value {
    super::generated::global("tap")
}

pub(super) fn const_global() -> Value {
    super::generated::global("const")
}

pub(super) fn compose_global() -> Value {
    super::generated::global("compose")
}
