//! TOML (v1.1).
//!
//! [`encode`] gives a TOML document, a String, and [`decode`] reads one back.
//! Scripts call them as `encode(document, options?)` and `decode(text)` in
//! `ext.formats.toml`, where `options` is a Map holding:
//!
//! - `pretty`: a Bool, `false` by default, choosing pretty text as
//!   [`EncodeOptions::pretty`] describes.
//!
//! Scripts make a [`Datetime`] with `datetime(text)`, from TOML's text for any
//! kind of date or time; it gives Null for text that spells none.
//!
//! The text is laid out as people usually write TOML: a table's plain values
//! first, in key order, then its Maps and Arrays of Maps, in key order among
//! themselves, each Map as a `[table]` named by its dotted path and each Array
//! of Maps as an `[[array of tables]]`. A Map within any other Array is an
//! inline table.
//!
//! # Type mapping
//!
//! | Frost | TOML |
//! |---|---|
//! | Bool | `true` or `false` |
//! | Int | an integer |
//! | Float | a float |
//! | [`SpecialFloat`](frostlang::SpecialFloat) | `nan`, `inf`, or `-inf` |
//! | String | a string |
//! | Array | an array |
//! | Map with String keys | a table |
//! | [`Datetime`] | a date, time, or datetime |
//!
//! Anything else has no TOML form: Null, Bytes, a Map key of another type,
//! Functions, and other Opaques. A document must be a Map. Arrays and Maps
//! nest at most 81 deep, the document included.
//!
//! When decoding, an integer outside the Int range is an error, as TOML
//! requires. A NaN's sign is not kept.

mod natives;

pub(crate) use natives::module;

use std::fmt;

use frostlang::{FrostOpaque, FrostType, Value};

use crate::mapping::{self, BeyondInt, EncodeFailure, FormatOpaque, Mapping};
use crate::{DecodeError, EncodeError};

/// TOML's data model.
pub(crate) const TOML: Mapping = Mapping {
    name: "TOML",
    document_types: FrostType::MAP,
    types: FrostType::Bool
        .or(FrostType::Int)
        .or(FrostType::Float)
        .or(FrostType::String)
        .or(FrostType::Array)
        .or(FrostType::Map),
    key_types: FrostType::STRING,
    opaques: &[FormatOpaque::SpecialFloat, FormatOpaque::TomlDatetime],
    // The `toml` parser reads 81 levels of inline nesting, the document's own
    // included, failing on the 82nd.
    max_depth: 81,
    // TOML text writes no lengths.
    max_len: usize::MAX,
    // TOML integers are 64-bit signed, and the specification requires an
    // error for any other.
    beyond_int: BeyondInt::Refused,
    // `toml` passes a date, time, or datetime as its text, under a private
    // marker.
    marked: &[("$__toml_private_datetime", decode_datetime_text)],
};

/// The [`Datetime`] whose TOML text is `text`.
fn decode_datetime_text(text: &str) -> Result<Value, String> {
    text.parse::<Datetime>()
        .map(Value::opaque)
        .map_err(|err| err.to_string())
}

/// A TOML date, time, or datetime, of any of its four kinds: an offset
/// datetime, a local datetime, a local date, or a local time.
///
/// Frost has no type for these, so they decode to this Opaque, which encodes
/// back as the same TOML value. It renders as its TOML text, as in
/// `<TOML Datetime: 1979-05-27T07:32:00Z>`, and is `==` to another with the
/// same TOML text. [`FromStr`](std::str::FromStr) reads that text, and
/// [`Display`](fmt::Display) writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Datetime(::toml::value::Datetime);

impl Datetime {
    /// `toml`'s own datetime, which its serializer recognizes.
    pub(crate) fn as_toml(&self) -> &::toml::value::Datetime {
        &self.0
    }
}

impl std::str::FromStr for Datetime {
    type Err = DatetimeParseError;

    fn from_str(text: &str) -> Result<Self, DatetimeParseError> {
        text.parse::<::toml::value::Datetime>()
            .map(Self)
            .map_err(|_| DatetimeParseError(text.to_string()))
    }
}

impl fmt::Display for Datetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FrostOpaque for Datetime {
    fn type_name() -> &'static str {
        "TOML Datetime"
    }

    fn try_to_string(&self) -> Option<String> {
        Some(self.to_string())
    }

    fn equals(&self, other: &dyn FrostOpaque) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }
}

/// The error from parsing a [`Datetime`] from text that spells no TOML date,
/// time, or datetime.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not a TOML date, time, or datetime: {0:?}")]
pub struct DatetimeParseError(String);

/// How [`encode`] lays out its text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EncodeOptions {
    /// Whether each element of an Array of more than one element takes a
    /// line of its own, as long Arrays often do in TOML written by hand.
    /// Otherwise each Array is on one line.
    pub pretty: bool,
}

/// `document` as TOML text.
///
/// # Errors
///
/// An [`EncodeError`] if `document` is not a Map, holds anything TOML has no
/// form for, or nests too deep; see the [module docs](self).
pub fn encode(document: &Value, options: EncodeOptions) -> Result<String, EncodeError> {
    let mut text = ::toml::ser::Buffer::new();
    let serializer = if options.pretty {
        ::toml::ser::Serializer::pretty(&mut text)
    } else {
        ::toml::ser::Serializer::new(&mut text)
    };
    mapping::encode(&TOML, document, serializer).map_err(|failure| match failure {
        EncodeFailure::Refused(refusal) => refusal,
        // `toml` fails on its own only for a value the encoder refuses first,
        // or for a malformed datetime, which a `Datetime` cannot hold.
        EncodeFailure::Serializer(err) => unreachable!("toml failed on its own: {err}"),
    })?;
    Ok(text.to_string())
}

/// The document the TOML text `text` holds, as a Map. A byte order mark at
/// its start is skipped.
///
/// # Errors
///
/// A [`DecodeError`] if `text` is not TOML, or holds what Frost cannot; see
/// the [module docs](self).
pub fn decode(text: &str) -> Result<Value, DecodeError> {
    ::toml::de::Deserializer::parse(text)
        .and_then(|deserializer| {
            mapping::decode(&TOML, deserializer).map_err(|failure| failure.error)
        })
        .map_err(|err| decode_error(text, &err))
}

/// The [`DecodeError`] for `error`, met in decoding `text`: its message and
/// where it is, on one line, rather than the source excerpt `toml` renders.
fn decode_error(text: &str, error: &::toml::de::Error) -> DecodeError {
    let message = error.message();
    let Some(span) = error.span() else {
        return DecodeError::new(TOML.name, message);
    };
    // `toml`'s spans fall on character boundaries; one that did not would be
    // reported from the end of the text, rather than panic.
    let before = text.get(..span.start).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let mut line_up_to_error = before.rsplit('\n').next().unwrap_or_default();
    // A byte order mark takes no column.
    if line == 1 {
        line_up_to_error = line_up_to_error
            .strip_prefix('\u{feff}')
            .unwrap_or(line_up_to_error);
    }
    // An error at a `\r\n` line ending falls past the `\r`, which takes no
    // column either.
    let column = line_up_to_error.trim_end_matches('\r').chars().count() + 1;
    DecodeError::new(
        TOML.name,
        format!("{message} at line {line} column {column}"),
    )
}
