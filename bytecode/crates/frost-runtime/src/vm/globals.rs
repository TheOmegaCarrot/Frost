use std::sync::{Arc, LazyLock};

use crate::core::FrostResult;
use crate::{
    Arity, Bytecode, Closure, CompiledFunction, FrostArray, MapKey, NativeCtx, NativeFunction,
    Value,
};

use super::GlobalSet;

// Sync-only macro: names and slot initializers expand from the same `name => init`
// list in the same order, so `names[i]` and `slots[i]` cannot drift apart.
macro_rules! define_globals {
    ($($name:literal => $init:expr),* $(,)?) => {
        impl GlobalSet {
            /// Names of all predefined globals.
            /// Every `GlobalSet` shares the same names, so it is not a field.
            /// The ordering here determines the slot indices used by `LoadGlobal`.
            const NAMES: &'static [&'static str] = &[ $($name),* ];

            fn build_defaults() -> Self {
                GlobalSet ( vec![ $($init),* ] )
            }
        }
    };
}

define_globals! {
     // TODO: actually implement all the globals
    "print"     => Value::Null,
    "transform" => Value::Null,
    "try_call"  => try_call_global(),
    "call"      => call_global(),
}

/// Builds the `try_call` global -- Frost's catch primitive, surfaced as a native.
fn try_call_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(
        try_call,
        "try_call",
        // At least the function to call; any further args are passed to it.
        Arity::AtLeast(1),
    )))
}

fn call_global() -> Value {
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
    let call_args = args[1..]
        .iter_mut()
        .map(|v| std::mem::replace(v, Value::Null));

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

static DEFAULT_GLOBALS: LazyLock<Arc<GlobalSet>> =
    LazyLock::new(|| Arc::new(GlobalSet::build_defaults()));

impl GlobalSet {
    pub fn defaults() -> Arc<Self> {
        DEFAULT_GLOBALS.clone()
    }

    /// Look up a global's slot index by name, or `None` if it is not a predefined global.
    /// The result is stable for a given build, so callers (e.g. a compiler emitting `LoadGlobal`) may cache it.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        // Yes, this is a linear scan, but this should be a pretty cold path.
        Self::NAMES.iter().position(|&n| n == name)
    }

    /// Get the value at a global slot index.
    pub fn get(&self, idx: usize) -> &Value {
        &self.0[idx]
    }
}
