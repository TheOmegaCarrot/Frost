//! The functions of `ext.formats.toml`: thin wrappers, for scripts, over the
//! module's Rust API.

use frostlang::native::{De, Optional};
use frostlang::{ExtensionBuilder, FrostResult, FrostString, NativeCtx, Value, frost_fn};
use serde::Deserialize;

use crate::error::function_error;
use crate::toml::{self, Datetime, EncodeOptions};

/// The `toml` module, with its functions added.
pub(crate) fn module(toml: ExtensionBuilder) -> ExtensionBuilder {
    toml.function(encode_frost())
        .function(decode_frost())
        .function(datetime_frost())
}

/// The options Map of a script's `encode`.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ScriptOptions {
    pretty: bool,
}

#[frost_fn]
fn encode(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    document: Value,
    options: Optional<De<ScriptOptions>>,
) -> FrostResult {
    let Optional(options) = options;
    let ScriptOptions { pretty } =
        options.map_or_else(ScriptOptions::default, |De(options)| options);
    toml::encode(&document, EncodeOptions { pretty })
        .map(Value::from)
        .map_err(|err| function_error(ctx.name(), err))
}

#[frost_fn]
fn decode(#[frost(ctx)] ctx: &mut NativeCtx<'_>, text: FrostString) -> FrostResult {
    toml::decode(&text).map_err(|err| function_error(ctx.name(), err))
}

#[frost_fn]
fn datetime(text: FrostString) -> Value {
    text.parse::<Datetime>().map_or(Value::Null, Value::opaque)
}
