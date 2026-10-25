//! The functions of `ext.formats.cbor`: thin wrappers, for scripts, over the
//! module's Rust API.

use frostlang::{ExtensionBuilder, FrostBytes, FrostResult, NativeCtx, Value, frost_fn};

use crate::cbor;
use crate::error::function_error;

/// The `cbor` module, with its functions added.
pub(crate) fn module(cbor: ExtensionBuilder) -> ExtensionBuilder {
    cbor.function(encode_frost()).function(decode_frost())
}

#[frost_fn]
fn encode(#[frost(ctx)] ctx: &mut NativeCtx<'_>, value: Value) -> FrostResult {
    cbor::encode(&value)
        .map(Value::from)
        .map_err(|err| function_error(ctx.name(), err))
}

#[frost_fn]
fn decode(#[frost(ctx)] ctx: &mut NativeCtx<'_>, data: FrostBytes) -> FrostResult {
    cbor::decode(&data).map_err(|err| function_error(ctx.name(), err))
}
