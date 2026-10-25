//! How a format's data model maps to Frost values, and the encoder and decoder
//! every format shares, driven by that mapping.
//!
//! A format's own serde (de)serializer does the reading and writing. The
//! encoder and decoder stand between it and Frost values in place of the core
//! serde bridge, which carries data across as best it can: they refuse what the
//! format has no form for, so that whatever is encoded decodes back equal.

mod decode;
mod encode;
mod path;

pub(crate) use decode::Decoder;
pub(crate) use encode::{EncodeFailure, encode};

use frostlang::{MapKey, SpecialFloat, TypeSet, Value};

/// How one format's data maps to Frost values.
#[derive(Debug)]
pub(crate) struct Mapping {
    /// The format's name, as errors give it.
    pub(crate) name: &'static str,
    /// The types a whole document may be: the value given to encode.
    pub(crate) document_types: TypeSet,
    /// The types of value the format has a form for. Whatever this holds, the
    /// encoder refuses Functions, and every Opaque not in `opaques`.
    pub(crate) types: TypeSet,
    /// The types of Map key the format has a form for.
    pub(crate) key_types: TypeSet,
    /// The Opaques the format has a form for.
    pub(crate) opaques: &'static [FormatOpaque],
    /// How deep Arrays and Maps may nest, encoding and decoding alike: no
    /// deeper than the format's parser reads, so that whatever is encoded
    /// decodes back.
    pub(crate) max_depth: usize,
    /// The longest String or Bytes, in bytes, and the most elements of an
    /// Array or entries of a Map, that the format can write.
    pub(crate) max_len: usize,
    /// What an integer beyond the Int range decodes to.
    pub(crate) beyond_int: BeyondInt,
    /// Values serde's data model has no type for, which the format's
    /// deserializer passes as a one-entry Map from a private marker String to
    /// the value's text. Each pair is a marker and the decoder for that text.
    pub(crate) marked: &'static [(&'static str, TextDecoder)],
}

/// What an integer beyond the Int range decodes to.
#[derive(Clone, Copy, Debug)]
pub(crate) enum BeyondInt {
    /// The nearest Float.
    NearestFloat,
    /// Nothing: decoding it is an error.
    Refused,
    /// A [`Uint64`](crate::uint64::Uint64) Opaque, if within the `u64`
    /// range; decoding any other is an error.
    Uint64,
}

/// An Opaque holding a value of a type a format has, but Frost does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatOpaque {
    /// A NaN or infinite float.
    SpecialFloat,
    /// A TOML date, time, or datetime.
    #[cfg(feature = "toml")]
    TomlDatetime,
    /// An unsigned 64-bit integer beyond the Int range.
    Uint64,
}

impl FormatOpaque {
    /// Whether `value` is an Opaque of this kind.
    fn matches(self, value: &Value) -> bool {
        match self {
            FormatOpaque::SpecialFloat => value.downcast_opaque::<SpecialFloat>().is_some(),
            #[cfg(feature = "toml")]
            FormatOpaque::TomlDatetime => {
                value.downcast_opaque::<crate::toml::Datetime>().is_some()
            }
            FormatOpaque::Uint64 => value.downcast_opaque::<crate::uint64::Uint64>().is_some(),
        }
    }

    /// Serializes `value`, an Opaque this kind [`matches`](Self::matches), as
    /// the format's own value.
    fn serialize<S: serde::Serializer>(
        self,
        value: &Value,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            FormatOpaque::SpecialFloat => {
                let special = value
                    .downcast_opaque::<SpecialFloat>()
                    .expect("the caller checked `matches`");
                serializer.serialize_f64(special.get())
            }
            #[cfg(feature = "toml")]
            FormatOpaque::TomlDatetime => {
                let datetime = value
                    .downcast_opaque::<crate::toml::Datetime>()
                    .expect("the caller checked `matches`");
                serde::Serialize::serialize(datetime.as_toml(), serializer)
            }
            FormatOpaque::Uint64 => {
                let n = value
                    .downcast_opaque::<crate::uint64::Uint64>()
                    .expect("the caller checked `matches`");
                serializer.serialize_u64(n.get())
            }
        }
    }
}

/// Decodes a value from its text, or says why it cannot.
pub(crate) type TextDecoder = fn(&str) -> Result<Value, String>;

impl Mapping {
    /// The decoder for the text of values under `marker`, if it is one of
    /// this format's markers.
    fn marked_decoder(&self, marker: &str) -> Option<TextDecoder> {
        self.marked
            .iter()
            .find_map(|&(candidate, decoder)| (candidate == marker).then_some(decoder))
    }
}

/// `key` as Frost source writes it, such as `"my-key"` or `x'ff'`.
fn key_source(key: &MapKey) -> String {
    Value::from(key.clone()).to_debug_string()
}

/// What errors name `value` as: its type, or an Opaque's rendering, which
/// names its type and approximates its content, as `<SpecialFloat: NaN>`.
pub(crate) fn describe(value: &Value) -> String {
    match value {
        Value::Opaque(_) => value.to_debug_string(),
        value => value.type_name().to_string(),
    }
}
