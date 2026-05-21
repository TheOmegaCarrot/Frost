use std::fmt;
use std::sync::Arc;

use serde::ser::{self, Serialize};

use crate::{FrostArray, FrostFloat, FrostMap, MapKey, Value};

// -- Error --

#[derive(Debug)]
pub struct SerError(String);

impl fmt::Display for SerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SerError {}

impl ser::Error for SerError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerError(msg.to_string())
    }
}

// -- Public API --

pub fn to_value<T: Serialize>(value: &T) -> Result<Value, crate::FrostError> {
    value
        .serialize(ValueSerializer)
        .map_err(|e| crate::FrostError::Recoverable(e.0))
}

// -- Serializer --

struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = SerError;

    type SerializeSeq = SerializeArray;
    type SerializeTuple = SerializeArray;
    type SerializeTupleStruct = SerializeArray;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeStruct;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Value, SerError> {
        Ok(Value::from(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value, SerError> {
        Ok(Value::from(v as i64))
    }

    fn serialize_i16(self, v: i16) -> Result<Value, SerError> {
        Ok(Value::from(v as i64))
    }

    fn serialize_i32(self, v: i32) -> Result<Value, SerError> {
        Ok(Value::from(v as i64))
    }

    fn serialize_i64(self, v: i64) -> Result<Value, SerError> {
        Ok(Value::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Value, SerError> {
        Ok(Value::from(v as i64))
    }

    fn serialize_u16(self, v: u16) -> Result<Value, SerError> {
        Ok(Value::from(v as i64))
    }

    fn serialize_u32(self, v: u32) -> Result<Value, SerError> {
        Ok(Value::from(v as i64))
    }

    fn serialize_u64(self, v: u64) -> Result<Value, SerError> {
        i64::try_from(v)
            .map(Value::from)
            .map_err(|_| ser::Error::custom("u64 value exceeds i64 range"))
    }

    fn serialize_f32(self, v: f32) -> Result<Value, SerError> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Value, SerError> {
        FrostFloat::new(v)
            .map(Value::from)
            .map_err(ser::Error::custom)
    }

    fn serialize_char(self, v: char) -> Result<Value, SerError> {
        let mut buf = [0u8; 4];
        let s = v.encode_utf8(&mut buf);
        Ok(Value::from(s.as_bytes()))
    }

    fn serialize_str(self, v: &str) -> Result<Value, SerError> {
        Ok(Value::from(v))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value, SerError> {
        Ok(Value::from(v))
    }

    fn serialize_none(self) -> Result<Value, SerError> {
        Ok(Value::Null)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value, SerError> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value, SerError> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value, SerError> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value, SerError> {
        Ok(Value::from(variant))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value, SerError> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, SerError> {
        let inner = value.serialize(ValueSerializer)?;
        let map: FrostMap = vec![(
            MapKey::String(Arc::from(variant.as_bytes())),
            inner,
        )]
        .into_iter()
        .collect();
        Ok(Value::from(map))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<SerializeArray, SerError> {
        Ok(SerializeArray {
            elements: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<SerializeArray, SerError> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<SerializeArray, SerError> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<SerializeTupleVariant, SerError> {
        Ok(SerializeTupleVariant {
            variant,
            elements: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<SerializeMap, SerError> {
        Ok(SerializeMap {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            pending_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<SerializeStruct, SerError> {
        Ok(SerializeStruct {
            entries: Vec::with_capacity(len),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<SerializeStructVariant, SerError> {
        Ok(SerializeStructVariant {
            variant,
            entries: Vec::with_capacity(len),
        })
    }
}

// -- Compound serializers --

pub struct SerializeArray {
    elements: Vec<Value>,
}

impl ser::SerializeSeq for SerializeArray {
    type Ok = Value;
    type Error = SerError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerError> {
        self.elements.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, SerError> {
        Ok(Value::from(FrostArray::from(self.elements)))
    }
}

impl ser::SerializeTuple for SerializeArray {
    type Ok = Value;
    type Error = SerError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerError> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, SerError> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SerializeArray {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerError> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, SerError> {
        ser::SerializeSeq::end(self)
    }
}

pub struct SerializeTupleVariant {
    variant: &'static str,
    elements: Vec<Value>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerError> {
        self.elements.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, SerError> {
        let arr = Value::from(FrostArray::from(self.elements));
        let map: FrostMap = vec![(
            MapKey::String(Arc::from(self.variant.as_bytes())),
            arr,
        )]
        .into_iter()
        .collect();
        Ok(Value::from(map))
    }
}

pub struct SerializeMap {
    entries: Vec<(MapKey, Value)>,
    pending_key: Option<MapKey>,
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = SerError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), SerError> {
        let key_value = key.serialize(ValueSerializer)?;
        let map_key = MapKey::try_from(key_value)
            .map_err(ser::Error::custom)?;
        self.pending_key = Some(map_key);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerError> {
        let key = self
            .pending_key
            .take()
            .expect("serialize_value called before serialize_key");
        self.entries.push((key, value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Value, SerError> {
        Ok(Value::from(FrostMap::from(
            self.entries.into_iter().collect::<std::collections::BTreeMap<_, _>>(),
        )))
    }
}

pub struct SerializeStruct {
    entries: Vec<(MapKey, Value)>,
}

impl ser::SerializeStruct for SerializeStruct {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), SerError> {
        self.entries.push((
            MapKey::String(Arc::from(key.as_bytes())),
            value.serialize(ValueSerializer)?,
        ));
        Ok(())
    }

    fn end(self) -> Result<Value, SerError> {
        Ok(Value::from(FrostMap::from(
            self.entries.into_iter().collect::<std::collections::BTreeMap<_, _>>(),
        )))
    }
}

pub struct SerializeStructVariant {
    variant: &'static str,
    entries: Vec<(MapKey, Value)>,
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), SerError> {
        self.entries.push((
            MapKey::String(Arc::from(key.as_bytes())),
            value.serialize(ValueSerializer)?,
        ));
        Ok(())
    }

    fn end(self) -> Result<Value, SerError> {
        let inner: FrostMap = self
            .entries
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>()
            .into();
        let map: FrostMap = vec![(
            MapKey::String(Arc::from(self.variant.as_bytes())),
            Value::from(inner),
        )]
        .into_iter()
        .collect();
        Ok(Value::from(map))
    }
}

// -- Serialize impl for Value itself --

impl Serialize for Value {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Int(i) => serializer.serialize_i64(*i),
            Value::Float(f) => serializer.serialize_f64(f.get()),
            Value::String(s) => match std::str::from_utf8(s) {
                Ok(utf8) => serializer.serialize_str(utf8),
                Err(_) => serializer.serialize_bytes(s),
            },
            Value::Array(arr) => {
                use ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                for elem in arr {
                    seq.serialize_element(elem)?;
                }
                seq.end()
            }
            Value::Map(map) => {
                use ser::SerializeMap;
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    let key_value: Value = k.clone().into();
                    m.serialize_entry(&key_value, v)?;
                }
                m.end()
            }
            Value::Function(_) => Err(ser::Error::custom("cannot serialize Function")),
            Value::Opaque(_) => Err(ser::Error::custom("cannot serialize Opaque")),
        }
    }
}
