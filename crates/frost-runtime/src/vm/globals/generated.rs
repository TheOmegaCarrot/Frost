//! Globals written in Frost, compiled ahead of time and embedded as data.
//!
//! `generated/source.frst` is their source, and `generated/compiled.rs` its
//! compiled form. The runtime cannot depend on the compiler, so `compiled.rs` is
//! written by the `generated_globals` test in `frost-compile`, which fails while
//! the file is stale. To regenerate it after a change to the source or to the
//! compiler:
//!
//! ```text
//! UPDATE_GENERATED=1 cargo test -p frost-compile --test generated_globals
//! ```
//!
//! `compiled.rs` is a [`Value`] tree: a Map from each function's name to that
//! function as the runtime's serde bridge represents it, which [`from_value`]
//! decodes. Being plain data, it compiles whatever the bytecode looks like; a
//! stale one fails only to decode, which leaves the generator free to run.

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use crate::{CompiledFunction, Value, from_value};

#[rustfmt::skip]
mod compiled;

/// Each generated function, by name, decoded on first use.
static FUNCTIONS: LazyLock<BTreeMap<String, Arc<CompiledFunction>>> = LazyLock::new(|| {
    let Value::Map(functions) = compiled::functions() else {
        panic!("the generated functions are not a Map; regenerate compiled.rs");
    };
    functions
        .iter()
        .map(|(name, function)| {
            let name = name.to_string();
            let function = from_value::<CompiledFunction>(function.clone()).unwrap_or_else(|err| {
                panic!(
                    "the generated function `{name}` does not decode ({}); regenerate compiled.rs",
                    err.message()
                )
            });
            (name, Arc::new(function))
        })
        .collect()
});

/// The generated global named `name`, as a closure.
pub(super) fn global(name: &str) -> Value {
    let function = FUNCTIONS
        .get(name)
        .unwrap_or_else(|| panic!("there is no generated function `{name}`"));
    let closure = Arc::clone(function)
        .assert_trusted()
        .close(BTreeMap::new())
        .expect("a generated function has no captures");
    Value::Closure(closure)
}

/// The constructors `compiled.rs` is written in, one per kind of [`Value`].
// Whichever ones the current functions happen not to need go unused.
#[allow(dead_code)]
mod build {
    use crate::{FrostFloat, MapKey, Value};

    pub(super) fn null() -> Value {
        Value::Null
    }

    pub(super) fn boolean(b: bool) -> Value {
        Value::Bool(b)
    }

    pub(super) fn int(i: i64) -> Value {
        Value::Int(i)
    }

    /// A Float, from its bits, so it survives the trip through source text exactly.
    pub(super) fn float(bits: u64) -> Value {
        Value::Float(FrostFloat::new(f64::from_bits(bits)).expect("a generated Float is finite"))
    }

    pub(super) fn string(text: &str) -> Value {
        Value::from(text)
    }

    pub(super) fn bytes(octets: &[u8]) -> Value {
        Value::from(octets)
    }

    pub(super) fn array<const N: usize>(elements: [Value; N]) -> Value {
        Value::from(Vec::from(elements))
    }

    pub(super) fn map<const N: usize>(entries: [(Value, Value); N]) -> Value {
        entries
            .into_iter()
            .map(|(key, value)| {
                let key = MapKey::try_from(key).expect("a generated Map key is a valid key");
                (key, value)
            })
            .collect()
    }
}
