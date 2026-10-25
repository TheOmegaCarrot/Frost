//! The functions of `ext.formats.yaml`: thin wrappers, for scripts, over the
//! module's Rust API.

use frostlang::{ExtensionBuilder, FrostResult, FrostString, NativeCtx, Value, frost_fn};

use crate::error::function_error;
use crate::yaml;

/// The `yaml` module, with its functions added.
pub(crate) fn module(yaml: ExtensionBuilder) -> ExtensionBuilder {
    yaml.function(encode_frost()).function(decode_frost())
}

#[frost_fn]
fn encode(#[frost(ctx)] ctx: &mut NativeCtx<'_>, value: Value) -> FrostResult {
    yaml::encode(&value)
        .map(Value::from)
        .map_err(|err| function_error(ctx.name(), err))
}

#[frost_fn]
fn decode(#[frost(ctx)] ctx: &mut NativeCtx<'_>, text: FrostString) -> FrostResult {
    yaml::decode(&text).map_err(|err| function_error(ctx.name(), err))
}
