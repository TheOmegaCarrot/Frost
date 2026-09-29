use std::sync::Arc;

use crate::core::{FrostArray, FrostError, FrostFloat, FrostMap, MapKey, Value, ValueMap};

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Value {
        Value::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Value {
        Value::Int(i as i64)
    }
}

impl TryFrom<f64> for Value {
    type Error = FrostError;
    fn try_from(f: f64) -> Result<Value, Self::Error> {
        Ok(Value::Float(FrostFloat::new(f)?))
    }
}

impl From<FrostFloat> for Value {
    fn from(f: FrostFloat) -> Value {
        Value::Float(f)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::String(Arc::from(s))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::String(Arc::from(s))
    }
}

impl From<Arc<str>> for Value {
    fn from(s: Arc<str>) -> Value {
        Value::String(s)
    }
}

// The `[u8]` family produces `Bytes`: a byte sequence with no encoding guarantee
// is exactly what `Bytes` is for. Text comes from the `str` family above.
impl From<&[u8]> for Value {
    fn from(b: &[u8]) -> Value {
        Value::Bytes(Arc::from(b))
    }
}

impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Value {
        Value::Bytes(Arc::from(b))
    }
}

impl From<Arc<[u8]>> for Value {
    fn from(b: Arc<[u8]>) -> Value {
        Value::Bytes(b)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        FrostArray::from(value).into()
    }
}

impl From<&[Value]> for Value {
    fn from(value: &[Value]) -> Self {
        FrostArray::from(value).into()
    }
}

impl FromIterator<Value> for Value {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        FrostArray::from_iter(iter).into()
    }
}

impl From<FrostArray> for Value {
    fn from(a: FrostArray) -> Value {
        Value::Array(a)
    }
}

impl FromIterator<(MapKey, Value)> for Value {
    fn from_iter<T: IntoIterator<Item = (MapKey, Value)>>(iter: T) -> Self {
        FrostMap::from_iter(iter).into()
    }
}

impl From<FrostMap> for Value {
    fn from(a: FrostMap) -> Value {
        Value::Map(a)
    }
}

impl From<ValueMap> for Value {
    fn from(value: ValueMap) -> Self {
        Value::Map(value.into())
    }
}

impl Value {
    /// The conversion behind Frost's `to_int` for Numeric and String values:
    /// Int passes through, Float truncates toward zero, String parses as an integer.
    /// Any other type returns Null.
    pub fn to_frost_int(&self) -> Value {
        match self {
            Value::Int(_) => self.clone(),
            Value::Float(f) => Value::from(f.get() as i64),
            Value::String(s) => s.parse::<i64>().ok().map_or(Value::Null, Value::from),
            _ => Value::Null,
        }
    }

    /// The conversion behind Frost's `to_float` for Numeric and String values:
    /// Float passes through, Int promotes, String parses as a float.
    /// Any other type returns Null.
    pub fn to_frost_float(&self) -> Value {
        match self {
            Value::Float(_) => self.clone(),
            Value::Int(i) => FrostFloat::new(*i as f64).map_or(Value::Null, Value::from),
            Value::String(s) => s
                .parse::<f64>()
                .ok()
                .and_then(|f| FrostFloat::new(f).ok())
                .map_or(Value::Null, Value::from),
            _ => Value::Null,
        }
    }

    /// Builds a Map from key-value pairs, as in `Value::map([("a", Value::from(1))])`.
    /// Each entry is [inserted](ValueMap::insert) in turn.
    pub fn map<K: Into<MapKey>, const N: usize>(entries: [(K, Value); N]) -> Value {
        Value::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// Builds an Array from elements convertible to [`Value`], as in `Value::array([1, 2, 3])`.
    pub fn array<K: Into<Value>, const N: usize>(elements: [K; N]) -> Value {
        Value::from_iter(elements.into_iter().map(std::convert::Into::into))
    }
}
