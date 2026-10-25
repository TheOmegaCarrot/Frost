use std::cell::Cell;
use std::fmt;

use serde::ser::{self, Serialize};

use crate::core::serde::{int_value, special_float};
use crate::core::{FrostArray, FrostFloat, MapKey, Value, ValueMap};

thread_local! {
    /// Set while our own serializer is lifting a `Value` out of a [`ValueCarrier`], so the
    /// carrier deposits the value whole instead of serializing its data.
    static LIFTING_VALUE: Cell<bool> = const { Cell::new(false) };
    /// Carries a whole `Value` from a [`ValueCarrier`] to our
    /// [`ValueSerializer::serialize_newtype_struct`]. Set and taken within one call, with no
    /// foreign code in between.
    static OUTGOING_VALUE: Cell<Option<Value>> = const { Cell::new(None) };
}

// -- Error --

#[derive(Debug, thiserror::Error)]
pub(super) enum SerError {
    #[error("{0}")]
    Message(String),
    /// An entry's `None`, which [`insert_if_present`] leaves out rather than
    /// failing on. It never leaves [`to_value`].
    #[error("an absent entry, which should have been left out")]
    AbsentEntry,
}

impl ser::Error for SerError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerError::Message(msg.to_string())
    }
}

// -- Public API --

/// Serializes any `Serialize` type into a Frost `Value`.
///
/// An `Option` is presence: see [`Nullable`](crate::Nullable) for how `None`
/// and Null serialize. An `f64` or `f32` is finite: see
/// [`SpecialFloat`](crate::SpecialFloat).
pub fn to_value<T: Serialize>(value: &T) -> Result<Value, crate::core::FrostError> {
    value
        .serialize(ValueSerializer::Required)
        .map_err(|e| e.to_string().into())
}

// -- Serializer --

/// Serializes into a `Value`. The variant is where the value sits, which
/// decides what its `None` becomes.
#[derive(Clone, Copy)]
enum ValueSerializer {
    /// Anything not directly an entry's or a `Nullable`'s value: the top
    /// level, an element, a key, or what a `Some` or a newtype holds. Its
    /// `None` is an error.
    Required,
    /// A struct field's or Map entry's value: its `None` leaves the entry
    /// out.
    Entry,
    /// What a [`Nullable`](crate::Nullable) holds: its `None` is Null.
    InNullable,
}

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
        int_value(v).map_err(SerError::Message)
    }

    fn serialize_i128(self, v: i128) -> Result<Value, SerError> {
        int_value(v).map_err(SerError::Message)
    }

    fn serialize_u128(self, v: u128) -> Result<Value, SerError> {
        int_value(v).map_err(SerError::Message)
    }

    fn serialize_f32(self, v: f32) -> Result<Value, SerError> {
        self.serialize_f64(v as f64)
    }

    /// A Float is finite, so a NaN or infinity is an error. A `Value` holding a
    /// [`SpecialFloat`](crate::SpecialFloat) never reaches here: it crosses whole.
    fn serialize_f64(self, v: f64) -> Result<Value, SerError> {
        FrostFloat::new(v)
            .map(Value::Float)
            .map_err(|_| ser::Error::custom(format!("{v} is not finite")))
    }

    fn serialize_char(self, v: char) -> Result<Value, SerError> {
        let mut buf = [0u8; 4];
        let s = v.encode_utf8(&mut buf);
        Ok(Value::from(&*s))
    }

    fn serialize_str(self, v: &str) -> Result<Value, SerError> {
        Ok(Value::from(v))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value, SerError> {
        Ok(Value::from(v))
    }

    /// What `None` is where it sits; see [`Nullable`](crate::Nullable).
    fn serialize_none(self) -> Result<Value, SerError> {
        match self {
            Self::Required => Err(ser::Error::custom(
                "`None` can only leave out a struct field or Map entry; for Null, use `Nullable`",
            )),
            Self::Entry => Err(SerError::AbsentEntry),
            Self::InNullable => Ok(Value::Null),
        }
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value, SerError> {
        value.serialize(Self::Required)
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
        name: &'static str,
        value: &T,
    ) -> Result<Value, SerError> {
        if name == super::VALUE_NEWTYPE_TOKEN {
            // Our own `Value::serialize` wrapped a `ValueCarrier`.
            // Hold the lifting flag while the carrier deposits the value whole, then take it from the slot.
            // A payload that did not deposit (only a forged token can) is a recoverable error, not a panic.
            let _guard = LiftGuard::arm();
            value.serialize(self)?;
            return OUTGOING_VALUE
                .with(std::cell::Cell::take)
                .ok_or_else(|| ser::Error::custom("expected a Frost Value newtype payload"));
        }
        if name == super::NULLABLE_NEWTYPE_TOKEN {
            return value.serialize(Self::InNullable);
        }
        value.serialize(Self::Required)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, SerError> {
        let inner = value.serialize(Self::Required)?;
        Ok(Value::map([(variant, inner)]))
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

    fn serialize_map(self, _len: Option<usize>) -> Result<SerializeMap, SerError> {
        Ok(SerializeMap {
            entries: ValueMap::new(),
            pending_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<SerializeStruct, SerError> {
        Ok(SerializeStruct {
            entries: ValueMap::new(),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<SerializeStructVariant, SerError> {
        Ok(SerializeStructVariant {
            variant,
            entries: ValueMap::new(),
        })
    }
}

// -- Compound serializers --

pub(super) struct SerializeArray {
    elements: Vec<Value>,
}

impl ser::SerializeSeq for SerializeArray {
    type Ok = Value;
    type Error = SerError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerError> {
        self.elements
            .push(value.serialize(ValueSerializer::Required)?);
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

pub(super) struct SerializeTupleVariant {
    variant: &'static str,
    elements: Vec<Value>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerError> {
        self.elements
            .push(value.serialize(ValueSerializer::Required)?);
        Ok(())
    }

    fn end(self) -> Result<Value, SerError> {
        let arr = Value::from(FrostArray::from(self.elements));
        Ok(Value::map([(self.variant, arr)]))
    }
}

pub(super) struct SerializeMap {
    entries: ValueMap,
    pending_key: Option<MapKey>,
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = SerError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), SerError> {
        let key_value = key.serialize(ValueSerializer::Required)?;
        let map_key = MapKey::try_from(key_value).map_err(ser::Error::custom)?;
        self.pending_key = Some(map_key);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerError> {
        let key = self
            .pending_key
            .take()
            .expect("serialize_value called before serialize_key");
        insert_if_present(&mut self.entries, key, value)
    }

    fn end(self) -> Result<Value, SerError> {
        Ok(Value::from(self.entries))
    }
}

/// Serializes `value` into `entries` under `key`, leaving the entry out if
/// `value` serializes as `None`.
fn insert_if_present<T: ?Sized + Serialize>(
    entries: &mut ValueMap,
    key: impl Into<MapKey>,
    value: &T,
) -> Result<(), SerError> {
    match value.serialize(ValueSerializer::Entry) {
        Ok(serialized) => {
            entries.insert(key.into(), serialized);
            Ok(())
        }
        Err(SerError::AbsentEntry) => Ok(()),
        Err(err) => Err(err),
    }
}

pub(super) struct SerializeStruct {
    entries: ValueMap,
}

impl ser::SerializeStruct for SerializeStruct {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), SerError> {
        insert_if_present(&mut self.entries, key, value)
    }

    fn end(self) -> Result<Value, SerError> {
        Ok(Value::from(self.entries))
    }
}

pub(super) struct SerializeStructVariant {
    variant: &'static str,
    entries: ValueMap,
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), SerError> {
        insert_if_present(&mut self.entries, key, value)
    }

    fn end(self) -> Result<Value, SerError> {
        Ok(Value::map([(self.variant, Value::from(self.entries))]))
    }
}

// -- Serialize impl for Value itself --

/// Serializes a `Value`'s data through any serializer, erroring on Functions and on
/// Opaques other than a [`SpecialFloat`](crate::SpecialFloat), which is its float.
/// This is the path a foreign serializer takes for a `Value`; our own serializer lifts the
/// value whole instead (see [`ValueCarrier`]).
fn serialize_data<S: ser::Serializer>(value: &Value, serializer: S) -> Result<S::Ok, S::Error> {
    match value {
        Value::Null => serializer.serialize_unit(),
        Value::Bool(b) => serializer.serialize_bool(*b),
        Value::Int(i) => serializer.serialize_i64(*i),
        Value::Float(f) => serializer.serialize_f64(f.get()),
        Value::String(s) => serializer.serialize_str(s),
        Value::Bytes(b) => serializer.serialize_bytes(b),
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
        Value::NativeFunction(_) | Value::Closure(_) => {
            Err(ser::Error::custom("cannot serialize Function"))
        }
        Value::Opaque(_) => match special_float(value) {
            Some(f) => serializer.serialize_f64(f),
            None => Err(ser::Error::custom("cannot serialize Opaque")),
        },
    }
}

/// Wraps a `Value` so it can cross serde's generic `serialize_newtype_struct` boundary,
/// where the receiving serializer sees only an opaque `&impl Serialize`. A foreign
/// serializer serializes the value's data (see [`serialize_data`]); our own
/// serializer sets [`LIFTING_VALUE`], and the carrier deposits the value whole instead.
struct ValueCarrier<'a>(&'a Value);

impl Serialize for ValueCarrier<'_> {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if LIFTING_VALUE.with(std::cell::Cell::get) {
            let stale = OUTGOING_VALUE.with(|slot| slot.replace(Some(self.0.clone())));
            // A dirty slot means some middleware forged the token without draining it;
            // fail loudly in debug, overwrite (identical effect) in release.
            debug_assert!(stale.is_none(), "Value deposit found the slot occupied");
            // Placeholder; our serializer discards it and takes the value from the slot.
            serializer.serialize_unit()
        } else {
            serialize_data(self.0, serializer)
        }
    }
}

/// Holds [`LIFTING_VALUE`] for the span of a token lift and restores it on drop, clearing
/// [`OUTGOING_VALUE`] too. Drop runs even on unwind, so a panic or a forged token cannot
/// leave the flag set or a value stranded for the next call on this thread.
struct LiftGuard;

impl LiftGuard {
    fn arm() -> Self {
        LIFTING_VALUE.with(|f| f.set(true));
        LiftGuard
    }
}

impl Drop for LiftGuard {
    fn drop(&mut self) {
        LIFTING_VALUE.with(|f| f.set(false));
        OUTGOING_VALUE.with(|slot| drop(slot.take()));
    }
}

impl Serialize for Value {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Route through the token newtype so our own serializer can lift a Value across
        // whole (Functions/Opaques included); a foreign serializer serializes the carrier's
        // data transparently, which errors on Functions at any nesting depth.
        serializer.serialize_newtype_struct(super::VALUE_NEWTYPE_TOKEN, &ValueCarrier(self))
    }
}
