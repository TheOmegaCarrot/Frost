//! The functions of `ext.formats.json`: [`encode`](super::encode) and
//! [`decode`](super::decode), for scripts.

use frostlang::native::{De, Optional};
use frostlang::{
    ExtensionBuilder, FrostError, FrostResult, FrostString, NativeCtx, Value, frost_fn,
};
use serde::Deserialize;

use crate::error::function_error;
use crate::json::{self, EncodeOptions, Indent};

/// The `json` module, with its functions added.
pub(crate) fn module(json: ExtensionBuilder) -> ExtensionBuilder {
    json.function(encode_frost()).function(decode_frost())
}

/// The options Map of a script's `encode`.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ScriptOptions {
    pretty: bool,
    indent: Option<i64>,
}

impl ScriptOptions {
    /// These options, as `encode` takes them. `function` names the function
    /// in errors.
    fn encode_options(self, function: &str) -> Result<EncodeOptions, FrostError> {
        let pretty = match (self.pretty, self.indent) {
            (false, None) => None,
            (false, Some(_)) => {
                return Err(FrostError::from_string(format!(
                    "Function {function} takes option `indent` only with `pretty: true`"
                )));
            }
            (true, None) => Some(Indent::DEFAULT),
            (true, Some(spaces)) => {
                let indent = u8::try_from(spaces).ok().and_then(Indent::new);
                Some(indent.ok_or_else(|| {
                    FrostError::from_string(format!(
                        "Function {function} requires option `indent` from 0 to {}, got {spaces}",
                        Indent::MAX
                    ))
                })?)
            }
        };
        Ok(EncodeOptions { pretty })
    }
}

#[frost_fn]
fn encode(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: Value,
    options: Optional<De<ScriptOptions>>,
) -> FrostResult {
    let function = ctx.name();
    let Optional(options) = options;
    let options = options
        .map_or_else(ScriptOptions::default, |De(options)| options)
        .encode_options(function)?;
    json::encode(&value, options)
        .map(Value::from)
        .map_err(|err| function_error(function, err))
}

#[frost_fn]
fn decode(#[frost(ctx)] ctx: &mut NativeCtx<'_>, text: FrostString) -> FrostResult {
    json::decode(&text).map_err(|err| function_error(ctx.name(), err))
}
