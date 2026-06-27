use std::sync::{Arc, LazyLock};

use crate::core::FrostResult;
use crate::{Arity, FrostArray, MapKey, NativeCtx, NativeFunction, Value};

use super::GlobalSet;

// Sync-only macro: names and slot initializers expand from the same `name => init`
// list in the same order, so `names[i]` and `slots[i]` cannot drift apart.
macro_rules! define_globals {
    ($($name:literal => $init:expr),* $(,)?) => {
        impl GlobalSet {
            /// Names of all predefined globals.
            /// Every GlobalSet shares the same names, so it is not a
            /// field. The single source of truth for global ordering.
            const NAMES: &'static [&'static str] = &[ $($name),* ];

            fn build_defaults() -> Self {
                GlobalSet ( vec![ $($init),* ] )
            }
        }
    };
}

define_globals! {
     // TODO: actually implement these
    "print"     => Value::Null,
    "transform" => Value::Null,
    "try_call"  => try_call_global(),
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

/// `try_call(f, ...args)` -- invoke `f` with `args` and reify the outcome into a
/// result map rather than letting an error propagate:
///   success: `{ ok: true,  value: <result> }`
///   failure: `{ ok: false, error: <message>, trace: [<frame names>] }`
///
/// This is "the native that declines to `?`": Frost's error model is uniform `?`
/// propagation through native frames, and `try_call` is the one place that catches
/// the unwinding error instead of re-raising it. By the time `invoke` returns Err,
/// the boundary has already restored the Vm, so building the map here is safe.
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
            let trace = err.backtrace.into_iter().map(Value::from).collect::<Vec<_>>();
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

    /// Override an existing global by name. Returns the set on
    /// success, or `None` if `name` is not a known global, the set is closed,
    /// so new globals cannot be added.
    pub fn with_override(mut self: Arc<Self>, name: &str, value: Value) -> Option<Arc<GlobalSet>> {
        let idx = self.index_of(name)?;
        Arc::make_mut(&mut self).0[idx] = value;
        Some(self)
    }

    /// Look up a global's slot index by name, or `None` if it is not a predefined
    /// global. Stable for a given build, so callers (e.g. the compiler, or a
    /// `LoadGlobal` emitter) may cache the result.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        // Yes, this is a linear scan, but this should be a pretty cold path.
        Self::NAMES.iter().position(|&n| n == name)
    }

    /// Get the value at a global slot index. Used by the VM during execution.
    pub fn get(&self, idx: usize) -> &Value {
        &self.0[idx]
    }
}
