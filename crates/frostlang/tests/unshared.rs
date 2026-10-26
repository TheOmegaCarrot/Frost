//! Harness for the tests of in-place updates: [`unshared`], a native telling
//! whether a structure is referenced by nothing else, and so could be updated in
//! place.

use frostlang::{Value, frost_closure};

/// `unshared(structure)`: whether the Array or Map passed in is referenced by
/// nothing but the argument itself.
pub(crate) fn unshared() -> Value {
    frost_closure!("unshared", |structure: Value| match structure {
        Value::Array(array) => array.try_into_vec().is_ok(),
        Value::Map(map) => map.try_into_map().is_ok(),
        other => panic!("`unshared` takes an Array or Map, not {other:?}"),
    })
}
