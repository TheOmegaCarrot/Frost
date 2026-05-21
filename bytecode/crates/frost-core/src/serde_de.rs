use std::fmt;
use std::sync::Arc;

use serde::de::{self, IntoDeserializer, Visitor};
use serde::{Deserialize, Deserializer};

use crate::{FrostArray, FrostFloat, FrostMap, MapKey, Value};

// -- Error --

#[derive(Debug)]
pub struct DeError(String);

impl DeError {
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DeError {}

impl de::Error for DeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DeError(msg.to_string())
    }
}

// -- Public API --

/// Deserializes a Frost `Value` into any `Deserialize` type.
pub fn from_value<'de, T: serde::Deserialize<'de>>(value: Value) -> Result<T, crate::FrostError> {
    T::deserialize(ValueDeserializer(value)).map_err(|e| crate::FrostError::Recoverable(e.0))
}

// -- Deserializer --

struct ValueDeserializer(Value);

impl<'de> de::Deserializer<'de> for ValueDeserializer {
    type Error = DeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Int(i) => visitor.visit_i64(i),
            Value::Float(f) => visitor.visit_f64(f.get()),
            Value::String(ref s) => match std::str::from_utf8(s) {
                Ok(utf8) => visitor.visit_str(utf8),
                Err(_) => visitor.visit_bytes(s),
            },
            Value::Array(_) => self.deserialize_seq(visitor),
            Value::Map(_) => self.deserialize_map(visitor),
            Value::Function(_) => Err(de::Error::custom("cannot deserialize Function")),
            Value::Opaque(_) => Err(de::Error::custom("cannot deserialize Opaque")),
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.0 {
            Value::Array(arr) => {
                let len = arr.len();
                let elements: Vec<Value> = arr.iter().cloned().collect();
                visitor.visit_seq(ArrayAccess {
                    iter: elements.into_iter(),
                    len,
                })
            }
            _ => Err(de::Error::custom(format!(
                "expected Array, got {}",
                self.0.type_name()
            ))),
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.0 {
            Value::Map(ref map) => visitor.visit_map(MapAccess {
                iter: map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>()
                    .into_iter(),
                pending_value: None,
            }),
            _ => Err(de::Error::custom(format!(
                "expected Map, got {}",
                self.0.type_name()
            ))),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError> {
        self.deserialize_map(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.0 {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError> {
        match &self.0 {
            Value::String(s) => {
                let s = std::str::from_utf8(s).map_err(de::Error::custom)?;
                visitor.visit_enum(s.into_deserializer())
            }
            Value::Map(map) => {
                if map.len() != 1 {
                    return Err(de::Error::custom(
                        "expected Map with exactly one entry for enum variant",
                    ));
                }
                let (key, value) = map.iter().next().unwrap();
                let variant_name = match key {
                    MapKey::String(s) => std::str::from_utf8(s).map_err(de::Error::custom)?,
                    _ => return Err(de::Error::custom("enum variant key must be a String")),
                };
                visitor.visit_enum(EnumAccess {
                    variant: variant_name.to_owned(),
                    value: value.clone(),
                })
            }
            _ => Err(de::Error::custom(format!(
                "expected String or Map for enum, got {}",
                self.0.type_name()
            ))),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DeError> {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char str string bytes byte_buf
        unit unit_struct
        tuple tuple_struct
        identifier ignored_any
    }
}

// -- SeqAccess for Array --

struct ArrayAccess {
    iter: std::vec::IntoIter<Value>,
    len: usize,
}

impl<'de> de::SeqAccess<'de> for ArrayAccess {
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DeError>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(ValueDeserializer(value)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

// -- MapAccess for Map --

struct MapAccess {
    iter: std::vec::IntoIter<(MapKey, Value)>,
    pending_value: Option<Value>,
}

impl<'de> de::MapAccess<'de> for MapAccess {
    type Error = DeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, DeError>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.pending_value = Some(value);
                let key_value: Value = key.into();
                seed.deserialize(ValueDeserializer(key_value)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, DeError>
    where
        V: de::DeserializeSeed<'de>,
    {
        let value = self
            .pending_value
            .take()
            .expect("next_value_seed called before next_key_seed");
        seed.deserialize(ValueDeserializer(value))
    }
}

// -- EnumAccess for data-carrying enums --

struct EnumAccess {
    variant: String,
    value: Value,
}

impl<'de> de::EnumAccess<'de> for EnumAccess {
    type Error = DeError;
    type Variant = VariantAccess;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantAccess), DeError>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((variant, VariantAccess(self.value)))
    }
}

struct VariantAccess(Value);

impl<'de> de::VariantAccess<'de> for VariantAccess {
    type Error = DeError;

    fn unit_variant(self) -> Result<(), DeError> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, DeError>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(ValueDeserializer(self.0))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer(self.0).deserialize_seq(visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer(self.0).deserialize_map(visitor)
    }
}

// -- Deserialize impl for Value itself --

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a Frost value")
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        i64::try_from(v)
            .map(Value::from)
            .map_err(|_| de::Error::custom("u64 value exceeds i64 range"))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        FrostFloat::new(v)
            .map(Value::from)
            .map_err(de::Error::custom)
    }

    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_some<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        Value::deserialize(deserializer)
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut elements = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(elem) = seq.next_element()? {
            elements.push(elem);
        }
        Ok(Value::from(FrostArray::from(elements)))
    }

    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, value)) = map.next_entry::<Value, Value>()? {
            let map_key = MapKey::try_from(key).map_err(de::Error::custom)?;
            entries.push((map_key, value));
        }
        Ok(Value::from(FrostMap::from(
            entries
                .into_iter()
                .collect::<std::collections::BTreeMap<_, _>>(),
        )))
    }
}
