//! [`MapKey`]: the key type of a Frost map, and its conversions.

use std::sync::Arc;

use crate::core::{FrostError, FrostFloat, Value};

/// A valid Frost map key. Only non-null primitive types may be keys.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum MapKey {
    Bool(bool),
    Int(i64),
    Float(FrostFloat),
    String(Arc<[u8]>),
}

/// Renders the key as it would be written, unquoted:
/// a String key prints its own text, so `'{key}'` reads as the source spelled it.
/// Invalid UTF-8 is replaced rather than refused.
impl std::fmt::Display for MapKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(x) => write!(f, "{}", x.get()),
            Self::String(s) => write!(f, "{}", String::from_utf8_lossy(s)),
        }
    }
}

impl From<MapKey> for Value {
    fn from(k: MapKey) -> Value {
        match k {
            MapKey::Bool(b) => Value::Bool(b),
            MapKey::Int(i) => Value::Int(i),
            MapKey::Float(f) => Value::Float(f),
            MapKey::String(s) => Value::String(s),
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
            _ => Err(format!("Type {} is not a valid Map key", v.type_name()).into()),
        }
    }
}

impl From<&str> for MapKey {
    fn from(value: &str) -> Self {
        Self::String(value.as_bytes().into())
    }
}

impl From<String> for MapKey {
    fn from(value: String) -> Self {
        value.as_str().into()
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
