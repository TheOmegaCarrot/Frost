//! The functions of `ext.formats.u64`: thin wrappers, for scripts, over the
//! module's Rust API.

use frostlang::{
    ExtensionBuilder, FrostError, FrostResult, FrostString, NativeCtx, Value, frost_fn,
};

use crate::mapping::describe;
use crate::uint64;

/// The `u64` module, with its functions added.
pub(crate) fn module(u64_module: ExtensionBuilder) -> ExtensionBuilder {
    u64_module
        .function(from_string_frost())
        .function(to_string_frost())
        .function(to_float_frost())
}

#[frost_fn]
fn from_string(text: FrostString) -> Value {
    text.parse::<u64>().map_or(Value::Null, uint64::to_value)
}

/// The unsigned integer `n`, the argument of the function named `function`.
/// Taken untyped, so that every argument but an unsigned integer is refused
/// in the one message.
fn uint_arg(function: &str, n: &Value) -> Result<u64, FrostError> {
    uint64::from_value(n).ok_or_else(|| {
        // A negative Int is named by its value, which says more than its type.
        let got = match n {
            Value::Int(i) => i.to_string(),
            other => describe(other),
        };
        FrostError::from_string(format!(
            "Function {function} requires an Int that is not negative, or a Uint64, as \
             argument 1 (n), got {got}"
        ))
    })
}

#[frost_fn]
fn to_string(#[frost(ctx)] ctx: &mut NativeCtx<'_>, n: Value) -> FrostResult {
    uint_arg(ctx.name(), &n).map(|n| Value::from(n.to_string()))
}

#[frost_fn]
fn to_float(#[frost(ctx)] ctx: &mut NativeCtx<'_>, n: Value) -> FrostResult {
    uint_arg(ctx.name(), &n).map(|n| Value::from_f64(n as f64))
}
