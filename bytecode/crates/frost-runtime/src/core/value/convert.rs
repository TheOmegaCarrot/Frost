use std::{collections::BTreeMap, sync::Arc};

use crate::core::{FrostArray, FrostError, FrostFloat, FrostMap, MapKey, Value};

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
        Value::String(Arc::from(s.as_bytes()))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::String(Arc::from(s.into_bytes()))
    }
}

impl From<&[u8]> for Value {
    fn from(s: &[u8]) -> Value {
        Value::String(Arc::from(s))
    }
}

impl From<Vec<u8>> for Value {
    fn from(s: Vec<u8>) -> Value {
        Value::String(Arc::from(s))
    }
}

impl From<Arc<[u8]>> for Value {
    fn from(s: Arc<[u8]>) -> Value {
        Value::String(s)
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

impl From<BTreeMap<MapKey, Value>> for Value {
    fn from(value: BTreeMap<MapKey, Value>) -> Self {
        Value::Map(value.into())
    }
}

impl Value {
    /// Frost's `to_int`: Int passes through, Float truncates toward zero, String parses as an integer.
    /// Everything else returns Null.
    pub fn to_frost_int(&self) -> Value {
        match self {
            Value::Int(_) => self.clone(),
            Value::Float(f) => Value::from(f.get() as i64),
            Value::String(s) => std::str::from_utf8(s)
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
                .map(Value::from)
                .unwrap_or(Value::Null),
            _ => Value::Null,
        }
    }

    /// Frost's `to_float`: Float passes through, Int promotes, String parses as a float.
    /// Everything else returns Null.
    pub fn to_frost_float(&self) -> Value {
        match self {
            Value::Float(_) => self.clone(),
            Value::Int(i) => FrostFloat::new(*i as f64)
                .map(Value::from)
                .unwrap_or(Value::Null),
            Value::String(s) => std::str::from_utf8(s)
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .and_then(|f| FrostFloat::new(f).ok())
                .map(Value::from)
                .unwrap_or(Value::Null),
            _ => Value::Null,
        }
    }

    pub fn map<K: Into<MapKey>, const N: usize>(entries: [(K, Value); N]) -> Value {
        Value::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn array<K: Into<Value>, const N: usize>(elements: [K; N]) -> Value {
        Value::from_iter(elements.into_iter().map(|e| e.into()))
    }
}
