//! JSON (RFC 8259).
//!
//! [`encode`] gives JSON text, a String, and [`decode`] reads it back. Scripts
//! call them as `encode(value, options?)` and `decode(text)` in
//! `ext.formats.json`, where `options` is a Map holding:
//!
//! - `pretty`: a Bool, `false` by default, choosing pretty text as
//!   [`EncodeOptions::pretty`] describes.
//! - `indent`: the spaces per level when `pretty`, as an [`Indent`] holds;
//!   2 by default. Given without `pretty: true`, it is an error.
//!
//! # Type mapping
//!
//! | Frost | JSON |
//! |---|---|
//! | Null | `null` |
//! | Bool | `true` or `false` |
//! | Int | a number with neither a fraction nor an exponent, such as `3` |
//! | Float | a number with a fraction or an exponent, such as `3.0` |
//! | String | a string |
//! | Array | an array |
//! | Map with String keys | an object |
//!
//! Anything else has no JSON form: Bytes, a Map key of another type,
//! Functions, and Opaques, including NaN and infinity as
//! [`SpecialFloat`](frostlang::SpecialFloat). Arrays and Maps nest at most 127
//! deep.
//!
//! When decoding, a number becomes the nearest Float when the type the table
//! gives it cannot hold it exactly: a fraction finer than a Float's precision,
//! or an integer outside the Int range. `-0` is the Float `-0.0`, which keeps
//! its sign, unless `serde_json`'s `arbitrary_precision` feature is enabled in
//! the build, which makes it the Int `0`.
//!
//! Decoding is an error for:
//!
//! - a number beyond every Float, such as `1e400`;
//! - a string escaping half of a surrogate pair alone, which a String cannot
//!   hold;
//! - an object holding a key twice.

mod natives;

pub(crate) use natives::module;

use frostlang::{FrostType, TypeSet, Value};
use serde_json::ser::PrettyFormatter;

use crate::mapping::{self, BeyondInt, EncodeFailure, Mapping};
use crate::{DecodeError, EncodeError};

/// The types JSON has a form for, any of which may be a whole document.
const TYPES: TypeSet = FrostType::Null
    .or(FrostType::Bool)
    .or(FrostType::Int)
    .or(FrostType::Float)
    .or(FrostType::String)
    .or(FrostType::Array)
    .or(FrostType::Map);

/// JSON's data model.
pub(crate) const JSON: Mapping = Mapping {
    name: "JSON",
    document_types: TYPES,
    types: TYPES,
    key_types: FrostType::STRING,
    opaques: &[],
    // `serde_json` reads 127 levels, failing on the 128th.
    max_depth: 127,
    // JSON text writes no lengths.
    max_len: usize::MAX,
    // JSON numbers carry no Int/Float distinction, so an integer beyond the
    // Int range is the nearest Float, as `serde_json` itself gives an integer
    // beyond the `u64` range.
    beyond_int: BeyondInt::NearestFloat,
    // With its `arbitrary_precision` feature, which any crate in a host's
    // build may enable, `serde_json` passes each number that fits neither an
    // `i64` nor a `u64` as its text, under a private marker.
    marked: &[("$serde_json::private::Number", decode_number_text)],
};

/// The value of a number given as its text: one with a fraction or an
/// exponent, or an integer beyond both an `i64` and a `u64`. Each is the
/// nearest Float, as `serde_json` reads it without `arbitrary_precision`.
fn decode_number_text(text: &str) -> Result<Value, String> {
    match text.parse::<f64>() {
        Ok(f) if f.is_finite() => Ok(Value::from_f64(f)),
        _ => Err("number out of range".to_string()),
    }
}

/// How [`encode`] lays out its text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EncodeOptions {
    /// `None` for compact text, all on one line. Otherwise pretty text, with
    /// each element of an Array or Map on a line of its own, indented this
    /// much deeper than its container.
    pub pretty: Option<Indent>,
}

/// The spaces per level of pretty JSON text, from 0 to [`MAX`](Self::MAX).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Indent(u8);

impl Indent {
    /// The most spaces per level.
    pub const MAX: u8 = 16;

    /// Two spaces per level: the [`Default`], and a script's unless it gives
    /// one.
    pub const DEFAULT: Indent = Indent(2);

    /// `spaces` per level, or `None` beyond [`MAX`](Self::MAX).
    pub const fn new(spaces: u8) -> Option<Indent> {
        if spaces <= Self::MAX {
            Some(Indent(spaces))
        } else {
            None
        }
    }

    /// The spaces per level.
    pub const fn spaces(self) -> u8 {
        self.0
    }
}

impl Default for Indent {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// `value` as JSON text.
///
/// # Errors
///
/// An [`EncodeError`] if `value` holds anything JSON has no form for, or
/// nests too deep; see the [module docs](self).
pub fn encode(value: &Value, options: EncodeOptions) -> Result<String, EncodeError> {
    let mut text = Vec::new();
    let encoded = match options.pretty {
        None => mapping::encode(&JSON, value, &mut serde_json::Serializer::new(&mut text)),
        Some(indent) => {
            let indent = vec![b' '; usize::from(indent.spaces())];
            let formatter = PrettyFormatter::with_indent(&indent);
            let mut serializer = serde_json::Serializer::with_formatter(&mut text, formatter);
            mapping::encode(&JSON, value, &mut serializer)
        }
    };
    encoded.map_err(|failure| match failure {
        EncodeFailure::Refused(refusal) => refusal,
        // Writing into memory, `serde_json` has no I/O to fail, and is given
        // only String keys: it fails only when the encoder refuses.
        EncodeFailure::Serializer(err) => unreachable!("serde_json failed on its own: {err}"),
    })?;
    Ok(String::from_utf8(text).expect("JSON text is UTF-8"))
}

/// The value the JSON text `text` holds. A byte order mark at its start is
/// skipped, as RFC 8259 allows.
///
/// # Errors
///
/// A [`DecodeError`] if `text` is not JSON, or holds what Frost cannot; see
/// the [module docs](self).
pub fn decode(text: &str) -> Result<Value, DecodeError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let decode_whole = || -> serde_json::Result<Value> {
        let mut deserializer = serde_json::Deserializer::from_str(text);
        let value = mapping::decode(&JSON, &mut deserializer).map_err(|failure| failure.error)?;
        // Anything but whitespace after the value is an error.
        deserializer.end()?;
        Ok(value)
    };
    decode_whole().map_err(|err| DecodeError::new(JSON.name, err))
}
