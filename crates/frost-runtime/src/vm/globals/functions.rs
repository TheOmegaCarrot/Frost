//! Higher-order function utilities: calling, error handling, and combinators.

use std::sync::Arc;

use crate::core::FrostResult;
use crate::{
    Arity, Bytecode, Closure, CompiledFunction, FormatVersion, FrostArray, FrostType, MapKey,
    NativeCtx, Param, Params, Value,
};

/// `call(f)` / `call(f, args)`: invoke `f`, spreading the elements of `args` (or
/// no args) as a tail call. A hand-rolled `Between(1, 2)` closure over `DynTailCall`.
pub(super) fn call_global() -> Value {
    Value::Closure(Arc::new(Closure {
        captures: Vec::new(),
        function: Arc::new(CompiledFunction {
            version: FormatVersion,
            name: "call".to_string(),
            arity: Arity::Between(1, 2),
            num_captures: 0,
            // Slot-free: an empty name_table gives the frame no local slots. The body
            // drops `call`'s own value with `DropBelow`, so no slots are needed.
            name_table: Vec::new(),
            constants: vec![call_type_check()],
            key_constants: Vec::new(),
            child_fns: Vec::new(),
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
                // A wrong type: the check raises its error, naming `call`.
                Bytecode::LoadConst(0), // 13: ( call_self f arr check )
                Bytecode::PeekDown(2),  // 14
                Bytecode::PeekDown(2),  // 15: ( call_self f arr check f arr )
                Bytecode::Call(2),      // 16: raises
                Bytecode::Pop,          // 17
                // Drop call's own value (2 below the top) so the callee lands at
                // this frame's base, then hand ( f arr ) to DynTailCall.
                Bytecode::DropBelow(2), // 18: ( f arr )
                Bytecode::DynTailCall,  // 19
            ],
        }),
    }))
}

/// A native named `call` whose only work is checking `call`'s argument types,
/// so a wrong type raises as a native's type check does. `call` runs it only
/// once its own fast check has failed.
fn call_type_check() -> Value {
    const PARAMS: Params =
        Params::new(&[Param::of(FrostType::FUNCTION), Param::of(FrostType::ARRAY)]);
    Value::checked_native("call", PARAMS, |_, _| Ok(Value::Null))
}

/// Builds the `try_call` global: Frost's catch primitive, surfaced as a native.
pub(super) fn try_call_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::FUNCTION),
        Param::of(FrostType::ARRAY).optional(),
    ]);
    Value::checked_native("try_call", PARAMS, try_call)
}

/// `try_call(f, args)`: invoke `f` with `args` (an array) and reify the outcome into a result map
/// rather than letting an error propagate:
///   success: `{ ok: true,  value: <result> }`
///   failure: `{ ok: false, error: <error value>, trace: [<frame names>] }`
///
/// The argument array is interpreted the same as `call`, only the result shape differs.
fn try_call(mut ctx: NativeCtx<'_>, args: &mut [Value]) -> FrostResult {
    // Preflight type-checking guarantees the first argument is a Function,
    // and the second is an Array (if present).
    let function = args[0].take();
    let call_args = match args.get_mut(1).map(Value::take) {
        Some(Value::Array(arr)) => arr,
        None => FrostArray::empty(),
        _ => unreachable!("Unreachable due to prior type-checking"),
    };

    match ctx.invoke(&function, call_args.into_vec()) {
        Ok(value) => Ok(result_map([
            (string_key("ok"), Value::Bool(true)),
            (string_key("value"), value),
        ])),
        Err(err) => {
            let trace = err
                .backtrace
                .iter()
                .map(|name| Value::from(name.as_str()))
                .collect::<Vec<_>>();
            Ok(result_map([
                (string_key("ok"), Value::Bool(false)),
                (string_key("error"), err.into_value()),
                (string_key("trace"), Value::Array(FrostArray::from(trace))),
            ]))
        }
    }
}

/// A string `MapKey` from a `&str` literal.
fn string_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s))
}

/// Build a `Value::Map` from a fixed set of entries.
fn result_map<const N: usize>(entries: [(MapKey, Value); N]) -> Value {
    Value::Map(entries.into_iter().collect())
}

pub(super) fn error_global() -> Value {
    // ProduceError consumes the top value and raises it: identical to `from_value`.
    super::bytecode_global(
        "error",
        Arity::Exact(1),
        vec![Bytecode::DropBelow(1), Bytecode::ProduceError],
    )
}

pub(super) fn and_then_global() -> Value {
    const PARAMS: Params = Params::new(&[Param::any(), Param::of(FrostType::FUNCTION)]);
    Value::checked_native("and_then", PARAMS, |mut ctx, args| {
        let value = args[0].take();
        match value {
            Value::Null => Ok(Value::Null),
            _ => ctx.invoke(&args[1], [value]),
        }
    })
}

pub(super) fn or_else_global() -> Value {
    const PARAMS: Params = Params::new(&[Param::any(), Param::of(FrostType::FUNCTION)]);
    Value::checked_native("or_else", PARAMS, |mut ctx, args| {
        let value = args[0].take();
        match value {
            Value::Null => ctx.invoke(&args[1], []),
            _ => Ok(value),
        }
    })
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
