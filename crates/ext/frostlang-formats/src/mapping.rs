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
    /// How deep Arrays and Maps may nest: as deep as the format's decoder
    /// reads, so that whatever is encoded decodes back.
    pub(crate) max_depth: usize,
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
}

/// An Opaque holding a value of a type a format has, but Frost does not.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FormatOpaque {
    /// A NaN or infinite float.
    SpecialFloat,
    /// A TOML date, time, or datetime.
    #[cfg(feature = "toml")]
    TomlDatetime,
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
