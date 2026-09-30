//! Printing and message formatting.

use crate::core::util::identifier::is_identifier_like;
use crate::{Arity, FrostError, FrostMap, FrostType, Param, Params, Value};

/// `print(value)`: hand the value, as `to_string` renders it, to the Vm's print
/// sink. Returns Null.
pub(super) fn print_global() -> Value {
    Value::native("print", Arity::Exact(1), |ctx, args| {
        ctx.vm.config.print_sink.print(&args[0].to_frost_string());
        Ok(Value::Null)
    })
}

/// The spec shared by `mformat` and `mprint`.
const MFORMAT_PARAMS: Params = Params::new(&[
    Param::of(FrostType::STRING).named("format string"),
    Param::of(FrostType::MAP).named("replacement map"),
]);

pub(super) fn mformat_global() -> Value {
    Value::checked_native("mformat", MFORMAT_PARAMS, |_, args| {
        let (Value::String(template), Value::Map(replacements)) = (&args[0], &args[1]) else {
            unreachable!()
        };
        Ok(Value::from(mformat(template, replacements)?))
    })
}

/// `mprint(format, replacements)`: print what `mformat` returns. Returns Null.
pub(super) fn mprint_global() -> Value {
    Value::checked_native("mprint", MFORMAT_PARAMS, |ctx, args| {
        let (Value::String(template), Value::Map(replacements)) = (&args[0], &args[1]) else {
            unreachable!()
        };
        ctx.vm
            .config
            .print_sink
            .print(&mformat(template, replacements)?);
        Ok(Value::Null)
    })
}

/// One piece of a parsed `mformat` template.
enum Segment<'a> {
    /// Text copied to the output as is.
    Literal(&'a str),
    /// A `${key}` placeholder, holding the key.
    Placeholder(&'a str),
}

/// Replace each `${key}` placeholder in `template` with the `to_string` rendering
/// of `replacements[key]`.
fn mformat(template: &str, replacements: &FrostMap) -> Result<String, FrostError> {
    // The whole template is parsed before any lookup, so a malformed placeholder
    // is reported even when an earlier one's key is missing.
    parse_template(template)?.into_iter().try_fold(
        String::with_capacity(template.len()),
        |mut out, segment| {
            match segment {
                Segment::Literal(text) => out.push_str(text),
                Segment::Placeholder(key) => {
                    let value = replacements.get_str(key).ok_or_else(|| {
                        FrostError::from_string(format!("Missing replacement for key: {key}"))
                    })?;
                    out.push_str(&value.to_frost_string());
                }
            }
            Ok(out)
        },
    )
}

/// Split `template` into literal text and placeholders.
///
/// `\$` and `\\` escape their second character; any other backslash is literal.
/// `${` opens a placeholder that runs to the next `}`, and a `$` that does not
/// open one is literal.
fn parse_template(template: &str) -> Result<Vec<Segment<'_>>, FrostError> {
    let mut segments = Vec::new();
    let mut rest = template;
    // Every special character is ASCII, so each split below lands on a char boundary.
    while let Some(at) = rest.find(['\\', '$']) {
        let (literal, special) = rest.split_at(at);
        segments.push(Segment::Literal(literal));
        rest = if let Some(after_backslash) = special.strip_prefix('\\') {
            let escapes = after_backslash.starts_with(['$', '\\']);
            let (text, after) = if escapes {
                after_backslash.split_at(1)
            } else {
                special.split_at(1)
            };
            segments.push(Segment::Literal(text));
            after
        } else if let Some(opened) = special.strip_prefix("${") {
            let Some(close) = opened.find('}') else {
                return Err(FrostError::from_string(format!(
                    "Unterminated format placeholder: {special}"
                )));
            };
            let key = &opened[..close];
            if !is_placeholder_key(key) {
                return Err(FrostError::from_string(format!(
                    "Invalid format placeholder: ${{{key}}}"
                )));
            }
            segments.push(Segment::Placeholder(key));
            &opened[close + 1..]
        } else {
            let (dollar, after) = special.split_at(1);
            segments.push(Segment::Literal(dollar));
            after
        };
    }
    segments.push(Segment::Literal(rest));
    Ok(segments)
}

/// Whether `key` may name a placeholder: identifier-shaped (reserved words
/// included), or one of `$`, `$$`, and `$0` to `$9`.
fn is_placeholder_key(key: &str) -> bool {
    is_identifier_like(key) || matches!(key.as_bytes(), [b'$'] | [b'$', b'$' | b'0'..=b'9'])
}
