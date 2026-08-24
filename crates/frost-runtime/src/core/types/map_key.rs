//! [`MapKey`]: the key type of a Frost map, and its conversions.

use std::sync::Arc;

use crate::core::{FrostError, FrostFloat, Value};

/// A valid Frost map key. Only non-null primitive types may be keys.
/// Totally ordered, such that ordering of keys of the same type agrees with Frost's `<` operator.
/// Ordering across types follows the variant order: Bool < Int < Float < String < Bytes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum MapKey {
    Bool(bool),
    Int(i64),
    Float(FrostFloat),
    String(Arc<str>),
    Bytes(Arc<[u8]>),
}

/// Renders the key exactly as the [`Value`] it stands for renders compactly.
/// A MapKey is a subset of Value, so its display mirrors that Value's: a String
/// prints its own text (unquoted), a Float as `3.0`, a Bytes key as `x'6869'`.
impl std::fmt::Display for MapKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&Value::from(self.clone()).to_frost_string())
    }
}

impl From<MapKey> for Value {
    fn from(k: MapKey) -> Value {
        match k {
            MapKey::Bool(b) => Value::Bool(b),
            MapKey::Int(i) => Value::Int(i),
            MapKey::Float(f) => Value::Float(f),
            MapKey::String(s) => Value::String(s),
            MapKey::Bytes(b) => Value::Bytes(b),
        }
    }
}

impl TryFrom<Value> for MapKey {
    type Error = FrostError;
    fn try_from(v: Value) -> Result<MapKey, Self::Error> {
        match v {
            Value::Bool(b) => Ok(MapKey::Bool(b)),
            Value::Int(i) => Ok(MapKey::Int(i)),
            Value::Float(f) => Ok(MapKey::Float(f)),
            Value::String(s) => Ok(MapKey::String(s)),
            Value::Bytes(b) => Ok(MapKey::Bytes(b)),
            _ => Err(format!("Type {} is not a valid Map key", v.type_name()).into()),
        }
    }
}

impl From<&str> for MapKey {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for MapKey {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<Arc<str>> for MapKey {
    fn from(value: Arc<str>) -> Self {
        Self::String(value)
    }
}

/// Byte sequences key as `Bytes`, mirroring `Value`'s `[u8]` conversions.
impl From<&[u8]> for MapKey {
    fn from(value: &[u8]) -> Self {
        Self::Bytes(value.into())
    }
}

impl From<Vec<u8>> for MapKey {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value.into())
    }
}

impl From<Arc<[u8]>> for MapKey {
    fn from(value: Arc<[u8]>) -> Self {
        Self::Bytes(value)
    }
}

impl From<i64> for MapKey {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<bool> for MapKey {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<FrostFloat> for MapKey {
    fn from(value: FrostFloat) -> Self {
        Self::Float(value)
    }
}

impl TryFrom<f64> for MapKey {
    type Error = FrostError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Ok(FrostFloat::try_from(value)?.into())
    }
}
