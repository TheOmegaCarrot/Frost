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

/// Decodes a value by a [`Mapping`], failing on any Map holding a key twice or
/// a key Frost cannot hold.
#[derive(Clone, Copy)]
pub(crate) struct Decoder<'a> {
    mapping: &'a Mapping,
}

impl<'a> Decoder<'a> {
    pub(crate) fn new(mapping: &'a Mapping) -> Self {
        Self { mapping }
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

    fn visit_u64<E>(self, n: u64) -> Result<Value, E> {
        Ok(match i64::try_from(n) {
            Ok(n) => Value::Int(n),
            Err(_) => match self.mapping.beyond_int {
                BeyondInt::NearestFloat => Value::from_f64(n as f64),
            },
        })
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
        let hint = seq.size_hint().unwrap_or(0).min(MAX_PREALLOCATION);
        let mut elements = Vec::with_capacity(hint);
        while let Some(element) = seq.next_element_seed(self)? {
            elements.push(element);
        }
        Ok(Value::from(FrostArray::from(elements)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let Some(first) = map.next_key_seed(self)? else {
            return Ok(Value::from(ValueMap::new()));
        };
        // A value serde has no type for arrives as a one-entry Map whose key
        // is a private marker; see `Mapping::marked`.
        if let Value::String(key) = &first
            && let Some(decode_text) = self.mapping.marked_decoder(key)
        {
            let text: String = map.next_value()?;
            return decode_text(&text).map_err(A::Error::custom);
        }
        let mut entries = ValueMap::new();
        self.insert_entry(&mut entries, first, &mut map)?;
        while let Some(key) = map.next_key_seed(self)? {
            self.insert_entry(&mut entries, key, &mut map)?;
        }
        Ok(Value::from(entries))
    }
}
