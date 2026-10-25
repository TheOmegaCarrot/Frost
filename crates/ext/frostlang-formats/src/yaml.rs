//! YAML.
//!
//! [`encode`] gives YAML text, a String, and [`decode`] reads it back. Scripts
//! call them as `encode(value)` and `decode(text)` in `ext.formats.yaml`.
//! Encoding writes block style and takes no options.
//!
//! # Type mapping
//!
//! | Frost | YAML |
//! |---|---|
//! | Null | `null` |
//! | Bool | `true` or `false` |
//! | Int | an integer, such as `3` |
//! | Float | a float with a fraction or an exponent, such as `3.0` |
//! | [`SpecialFloat`] | `.inf`, `-.inf`, or `.nan` |
//! | String | a string, quoted or in block style wherever plain text would not read back as itself |
//! | Array | a sequence |
//! | Map | a mapping |
//!
//! Map keys may be Bools, Ints, Floats, and Strings. Anything else has no YAML
//! form: Bytes, Functions, and other Opaques. Arrays and Maps nest at most 64
//! deep. A NaN's sign is not kept.
//!
//! When decoding:
//!
//! - A Bool is `true` or `false`, however capitalized (`True`, `tRUE`); YAML
//!   1.1's other spellings, such as `yes` and `on`, are Strings.
//! - Null is `null`, however capitalized, `~`, or an empty value, as in `a:`.
//!   Empty text, or text holding only comments, is Null too.
//! - An integer may be decimal, or hexadecimal, octal, or binary after `0x`,
//!   `0o`, or `0b`, and may hold `_` between digits. One outside the Int range
//!   is the nearest Float, as is a decimal with leading zeros, such as `017`;
//!   but one in hexadecimal, octal, or binary beyond 64 bits is a String.
//! - A float beyond every Float, such as `1e400`, is the infinity of its sign.
//! - Dates and times are Strings.
//! - An alias is the value its anchor marks, and a merge key (`<<`) merges
//!   the mappings it names.
//! - The tags `!!str`, `!!int`, `!!float`, `!!bool`, and `!!null` choose a
//!   scalar's type, and `!!seq` and `!!map` mark a sequence and a mapping.
//!   `!!binary` data whose bytes are UTF-8 is that text, as a String.
//! - After an explicit document end (`...`), text that is not YAML is
//!   ignored; text that is YAML is a second document.
//!
//! Decoding is an error for:
//!
//! - text holding more than one document;
//! - any other tag, or `!!binary` data that is not UTF-8;
//! - a Map key of a type a Frost Map key cannot be, such as null;
//! - a Map holding a key twice;
//! - a Bool or Float key beside the String key spelled the same, such as
//!   `true` and `"true"`: a Map holding both encodes but does not decode back;
//! - aliases that would expand to an outsize value.
//!
//! [`SpecialFloat`]: frostlang::SpecialFloat

mod natives;

pub(crate) use natives::module;

use frostlang::{FrostType, TypeSet, Value};
use serde_saphyr::{Budget, NonFiniteFloatPolicy, Options, UserMessageFormatter};

use crate::mapping::{self, BeyondInt, EncodeFailure, FormatOpaque, Mapping};
use crate::{DecodeError, EncodeError};

/// The types YAML has a form for, Opaques aside.
const TYPES: TypeSet = FrostType::Null
    .or(FrostType::Bool)
    .or(FrostType::Int)
    .or(FrostType::Float)
    .or(FrostType::String)
    .or(FrostType::Array)
    .or(FrostType::Map);

/// YAML's data model.
pub(crate) const YAML: Mapping = Mapping {
    name: "YAML",
    // Anything with a YAML form may be a whole document, including the
    // Opaques in `opaques`.
    document_types: TYPES.or(FrostType::Opaque),
    types: TYPES,
    key_types: FrostType::Bool
        .or(FrostType::Int)
        .or(FrostType::Float)
        .or(FrostType::String),
    opaques: &[FormatOpaque::SpecialFloat],
    // Deeper nesting could overflow a thread's stack in an unoptimized build:
    // `serde-saphyr` takes far more stack per level than the other formats'
    // parsers.
    max_depth: 64,
    // YAML text writes no lengths.
    max_len: usize::MAX,
    // YAML integers have no fixed range, and `serde-saphyr` gives a decimal
    // integer beyond both the `i64` and `u64` ranges as the nearest Float
    // itself.
    beyond_int: BeyondInt::NearestFloat,
    marked: &[],
};

/// `value` as YAML text.
///
/// # Errors
///
/// An [`EncodeError`] if `value` holds anything YAML has no form for, or
/// nests too deep; see the [module docs](self).
pub fn encode(value: &Value) -> Result<String, EncodeError> {
    let mut text = String::new();
    let encoded = mapping::encode(&YAML, value, &mut serde_saphyr::Serializer::new(&mut text));
    encoded.map_err(|failure| match failure {
        EncodeFailure::Refused(refusal) => refusal,
        // Writing into memory, `serde-saphyr` has no I/O to fail, and is given
        // only scalar keys.
        EncodeFailure::Serializer(err) => unreachable!("serde-saphyr failed on its own: {err}"),
    })?;
    Ok(text)
}

/// The value the YAML text `text` holds. A byte order mark at its start is
/// skipped.
///
/// # Errors
///
/// A [`DecodeError`] if `text` is not YAML, holds more than one document, or
/// holds what Frost cannot; see the [module docs](self).
pub fn decode(text: &str) -> Result<Value, DecodeError> {
    serde_saphyr::with_deserializer_from_str_with_options(text, options(), |deserializer| {
        mapping::decode(&YAML, deserializer).map_err(|failure| failure.error)
    })
    // Worded for whoever wrote the text, not for a Rust programmer setting
    // `serde-saphyr`'s options.
    .map_err(|err| DecodeError::new(YAML.name, err.render_with_formatter(&UserMessageFormatter)))
}

/// How `serde-saphyr` reads YAML for [`decode`]. Merge keys are merged, as
/// they are by default.
fn options() -> Options {
    let mut options = Options::default();
    // Pass `.inf` and `.nan` through, to decode as SpecialFloats; by default
    // they are errors.
    options.non_finite_float_policy = NonFiniteFloatPolicy::PassThrough;
    // Rather than dropping the tag, and decoding what it marks as if
    // untagged.
    options.reject_unsupported_tags = true;
    // YAML 1.2's booleans, rather than 1.1's, which take `yes`, `on`, and `y`
    // as well.
    options.strict_booleans = true;
    options.budget = Some(depth_budget());
    // One line, giving the line and column, rather than an excerpt of the
    // text.
    options.with_snippet = false;
    options
}

/// A budget limiting depth but not size, as no other format limits size.
///
/// `serde-saphyr` reads a mapping key, and the mappings a merge key names,
/// whole before the decoder sees them, expanding aliases as it goes; the
/// decoder's own depth limit comes too late for those. The budget counts the
/// same nesting. Set one level beyond the decoder's limit, it lets the decoder
/// refuse ordinary nesting first, with its own message.
fn depth_budget() -> Budget {
    let mut budget = Budget::default();
    budget.max_depth = YAML.max_depth + 1;
    budget.max_events = usize::MAX;
    budget.max_nodes = usize::MAX;
    budget.max_total_scalar_bytes = usize::MAX;
    budget.max_merge_keys = usize::MAX;
    // Alias expansion stays limited by `Options::alias_limits`, left at its
    // default.
    budget.max_aliases = usize::MAX;
    budget.max_anchors = usize::MAX;
    budget.max_recorded_anchor_events = usize::MAX;
    budget.max_recorded_anchor_bytes = usize::MAX;
    budget.enforce_alias_anchor_ratio = false;
    budget
}
