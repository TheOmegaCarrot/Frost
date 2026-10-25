//! Decoding a Frost value through a format's deserializer.

use std::fmt;

use frostlang::value_map::Entry;
use frostlang::{FrostArray, MapKey, Value, ValueMap};
use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};

use crate::mapping::{BeyondInt, Mapping, key_source};

/// The most Array elements to reserve room for before reading them: a
/// mebibyte's worth, as serde caps its own collections. The input's size hint
/// may claim far more elements than it holds.
const MAX_PREALLOCATION: usize = 1024 * 1024 / std::mem::size_of::<Value>();

/// Decodes a value by a [`Mapping`], failing on anything the mapping or a
/// Frost value cannot hold.
#[derive(Clone, Copy)]
pub(crate) struct Decoder<'a> {
    mapping: &'a Mapping,
    /// How many Arrays and Maps enclose the value to decode. The decoder
    /// keeps its own count, rather than trusting the parser's limit, which a
    /// Cargo feature of the parser may lift.
    depth: usize,
}

impl<'a> Decoder<'a> {
    /// The decoder for a whole value, by `mapping`.
    pub(crate) fn new(mapping: &'a Mapping) -> Self {
        Self { mapping, depth: 0 }
    }

    /// Enters the Array or Map being visited: the decoder for its elements or
    /// entry values, or an error if it nests deeper than the format allows.
    fn enter_container<E: Error>(self) -> Result<Self, E> {
        if self.depth == self.mapping.max_depth {
            return Err(E::custom(format_args!(
                "Arrays and Maps nest more than {} deep",
                self.mapping.max_depth
            )));
        }
        Ok(Self {
            depth: self.depth + 1,
            ..self
        })
    }

    /// The value the integer `n` decodes to: an Int, or what the mapping
    /// makes of an integer beyond the Int range. `nearest` is the Float
    /// nearest `n`.
    fn integer<E: Error>(
        self,
        n: impl TryInto<i64> + fmt::Display,
        nearest: f64,
    ) -> Result<Value, E> {
        let shown = n.to_string();
        match n.try_into() {
            Ok(n) => Ok(Value::Int(n)),
            Err(_) => match self.mapping.beyond_int {
                BeyondInt::NearestFloat => Ok(Value::from_f64(nearest)),
                BeyondInt::Refused => Err(E::custom(format_args!(
                    "integer {shown} is beyond the Int range"
                ))),
            },
        }
    }

    /// Adds to `entries` the entry keyed `key`, whose value `map` reads next.
    fn insert_entry<'de, A: MapAccess<'de>>(
        self,
        entries: &mut ValueMap,
        key: Value,
        map: &mut A,
    ) -> Result<(), A::Error> {
        let key = MapKey::try_from(key).map_err(|err| A::Error::custom(err.message()))?;
        match entries.entry(key) {
            Entry::Occupied(entry) => Err(A::Error::custom(format_args!(
                "duplicate key {}",
                key_source(entry.key())
            ))),
            Entry::Vacant(entry) => {
                entry.insert(map.next_value_seed(self)?);
                Ok(())
            }
        }
    }
}

impl<'de> DeserializeSeed<'de> for Decoder<'_> {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Decoder<'_> {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a {} value", self.mapping.name)
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        self.deserialize(deserializer)
    }

    fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
        Ok(Value::Bool(b))
    }

    fn visit_i64<E>(self, n: i64) -> Result<Value, E> {
        Ok(Value::Int(n))
    }

    fn visit_u64<E: Error>(self, n: u64) -> Result<Value, E> {
        self.integer(n, n as f64)
    }

    fn visit_i128<E: Error>(self, n: i128) -> Result<Value, E> {
        self.integer(n, n as f64)
    }

    fn visit_u128<E: Error>(self, n: u128) -> Result<Value, E> {
        self.integer(n, n as f64)
    }

    fn visit_f64<E>(self, f: f64) -> Result<Value, E> {
        Ok(Value::from_f64(f))
    }

    fn visit_str<E>(self, s: &str) -> Result<Value, E> {
        Ok(Value::from(s))
    }

    fn visit_string<E>(self, s: String) -> Result<Value, E> {
        Ok(Value::from(s))
    }

    fn visit_bytes<E>(self, b: &[u8]) -> Result<Value, E> {
        Ok(Value::from(b))
    }

    fn visit_byte_buf<E>(self, b: Vec<u8>) -> Result<Value, E> {
        Ok(Value::from(b))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let element_decoder = self.enter_container()?;
        let hint = seq.size_hint().unwrap_or(0).min(MAX_PREALLOCATION);
        let mut elements = Vec::with_capacity(hint);
        while let Some(element) = seq.next_element_seed(element_decoder)? {
            elements.push(element);
        }
        Ok(Value::from(FrostArray::from(elements)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        // Keys are decoded by `self`: a key is a scalar, which nests nothing.
        // The first is read apart from the rest, since it may be a marker.
        let Some(first) = map.next_key_seed(self)? else {
            return Ok(Value::from(ValueMap::new()));
        };
        // A value serde has no type for arrives as a one-entry Map whose key
        // is a private marker; see `Mapping::marked`. It is no Frost Map, so
        // it nests nothing.
        if let Value::String(key) = &first
            && let Some(decode_text) = self.mapping.marked_decoder(key)
        {
            let text: String = map.next_value()?;
            return decode_text(&text).map_err(A::Error::custom);
        }
        let entry_decoder = self.enter_container()?;
        let mut entries = ValueMap::new();
        entry_decoder.insert_entry(&mut entries, first, &mut map)?;
        while let Some(key) = map.next_key_seed(self)? {
            entry_decoder.insert_entry(&mut entries, key, &mut map)?;
        }
        Ok(Value::from(entries))
    }
}
