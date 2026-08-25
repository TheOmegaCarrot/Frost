use std::cell::Cell;
use std::fmt;

use serde::de::{self, IntoDeserializer, Visitor};
use serde::{Deserialize, Deserializer};

use crate::core::{FrostArray, FrostFloat, FrostMap, MapKey, Value};

thread_local! {
    /// Carries a whole `Value` from our own [`ValueDeserializer::deserialize_newtype_struct`]
    /// to [`ValueVisitor::visit_newtype_struct`], so the in-memory bridge preserves the
    /// Functions and Opaques that the serde data model cannot express. Set and taken within
    /// one call, with no foreign code in between.
    static INCOMING_VALUE: Cell<Option<Value>> = const { Cell::new(None) };
}

/// Clears [`INCOMING_VALUE`] on drop, so a value deposited for the token path leaves no
/// residue if the visitor does not take it (a forged token) or the call unwinds.
struct IncomingGuard;

impl Drop for IncomingGuard {
    fn drop(&mut self) {
        INCOMING_VALUE.with(|slot| drop(slot.take()));
    }
}

// -- Error --

#[derive(Debug)]
pub struct DeError(String);

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
pub fn from_value<'de, T: serde::Deserialize<'de>>(
    value: Value,
) -> Result<T, crate::core::FrostError> {
    T::deserialize(ValueDeserializer(value)).map_err(|e| e.0.into())
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
            Value::String(ref s) => visitor.visit_str(s),
            Value::Bytes(ref b) => visitor.visit_bytes(b),
            Value::Array(_) => self.deserialize_seq(visitor),
            Value::Map(_) => self.deserialize_map(visitor),
            Value::NativeFunction(_) => Err(de::Error::custom("cannot deserialize Function")),
            Value::Closure(_) => Err(de::Error::custom("cannot deserialize Function")),
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
            Value::String(s) => visitor.visit_enum(s.as_ref().into_deserializer()),
            Value::Map(map) => {
                if map.len() != 1 {
                    return Err(de::Error::custom(
                        "expected Map with exactly one entry for enum variant",
                    ));
                }
                let (key, value) = map.iter().next().unwrap();
                let variant_name = match key {
                    MapKey::String(s) => s.as_ref(),
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

    /// Text targets take a String and nothing else.
    ///
    /// Routing these through `deserialize_any` would let serde's default visitors
    /// accept either type, which would undo the String/Bytes distinction at the
    /// boundary where a host states which one it wants.
    ///
    /// This holds for a field deserialized directly. It cannot hold for `flatten`,
    /// untagged enums, or internally and adjacently tagged enums: serde buffers those
    /// through its own `Content` type and replays them through a lenient deserializer
    /// of its own, which never consults these methods.
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.0 {
            Value::String(ref s) => visitor.visit_str(s),
            _ => Err(de::Error::custom(format!(
                "expected String, got {}",
                self.0.type_name()
            ))),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        self.deserialize_str(visitor)
    }

    /// Byte targets take Bytes and nothing else.
    ///
    /// A plain `Vec<u8>` asks for a sequence, not for bytes, and so is served by
    /// [`deserialize_seq`](Self::deserialize_seq) from an Array: the mirror of how
    /// it serializes. Reaching a Bytes value requires `#[serde(with = "serde_bytes")]`.
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        match self.0 {
            Value::Bytes(ref b) => visitor.visit_bytes(b),
            _ => Err(de::Error::custom(format!(
                "expected Bytes, got {}",
                self.0.type_name()
            ))),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        self.deserialize_bytes(visitor)
    }

    /// A field name is text, so binary cannot spell one.
    ///
    /// Only Bytes is turned away: serde's derived identifier visitor matches a field
    /// by its bytes as readily as by its text, which is the one way binary could name
    /// a String-spelled field. Every other key type keeps reaching its own visitor
    /// method unchanged.
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        if self.0.is_bytes() {
            return Err(de::Error::custom(format!(
                "expected String, got {}",
                self.0.type_name()
            )));
        }
        self.deserialize_any(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DeError> {
        if name == super::VALUE_NEWTYPE_TOKEN {
            // Our own `Value::deserialize` is asking for the value whole. Deposit it in the
            // slot for `ValueVisitor::visit_newtype_struct` to lift out. The guard clears the
            // slot on the way out, including on unwind or a forged token, so no dirty value
            // survives; the dummy deserializer is consulted only if the visitor is not ours.
            let _guard = IncomingGuard;
            let stale = INCOMING_VALUE.with(|slot| slot.replace(Some(self.0)));
            // A dirty slot means some middleware forged the token without draining it;
            // fail loudly in debug, overwrite (identical effect) in release.
            debug_assert!(stale.is_none(), "Value deposit found the slot occupied");
            return visitor.visit_newtype_struct(ValueDeserializer(Value::Null));
        }
        visitor.visit_newtype_struct(self)
    }

    /// An ignored field's value is discarded without inspection, so a Function or Opaque
    /// in a field the target type does not name is skipped rather than erroring.
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeError> {
        visitor.visit_unit()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char
        unit unit_struct
        tuple tuple_struct
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
        // Route through the token newtype so our own deserializer can hand a Value across
        // whole (Functions/Opaques included); a foreign deserializer treats it as a
        // transparent newtype and reaches `visit_newtype_struct` with an empty slot.
        deserializer.deserialize_newtype_struct(super::VALUE_NEWTYPE_TOKEN, ValueVisitor)
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

    fn visit_newtype_struct<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Value, D::Error> {
        match INCOMING_VALUE.with(|slot| slot.take()) {
            // Our own deserializer deposited the value whole, Functions and Opaques included.
            Some(value) => Ok(value),
            // A foreign deserializer's transparent newtype: deserialize the inner normally.
            None => deserializer.deserialize_any(ValueVisitor),
        }
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
