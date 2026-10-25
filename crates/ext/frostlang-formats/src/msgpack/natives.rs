//! The functions of `ext.formats.msgpack`: thin wrappers, for scripts, over
//! the module's Rust API.

use frostlang::{ExtensionBuilder, FrostBytes, FrostResult, NativeCtx, Value, frost_fn};

use crate::error::function_error;
use crate::msgpack;

/// The `msgpack` module, with its functions added.
pub(crate) fn module(msgpack: ExtensionBuilder) -> ExtensionBuilder {
    msgpack.function(encode_frost()).function(decode_frost())
}

#[frost_fn]
fn encode(#[frost(ctx)] ctx: &mut NativeCtx<'_>, value: Value) -> FrostResult {
    msgpack::encode(&value)
        .map(Value::from)
        .map_err(|err| function_error(ctx.name(), err))
}

#[frost_fn]
fn decode(#[frost(ctx)] ctx: &mut NativeCtx<'_>, data: FrostBytes) -> FrostResult {
    msgpack::decode(&data).map_err(|err| function_error(ctx.name(), err))
}
