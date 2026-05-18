use std::sync::Arc;

use crate::value::{FrostArray, FrostError, FrostFloat, FrostMap, MapKey, Value};

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

impl From<FrostArray> for Value {
    fn from(a: FrostArray) -> Value {
        Value::Array(a)
    }
}

impl From<FrostMap> for Value {
    fn from(a: FrostMap) -> Value {
        Value::Map(a)
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
            _ => Err(Self::Error::Recoverable(format!(
                "Type {} is not a valid Map key",
                v.type_name()
            ))),
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            _ => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "Bool",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::Array(_) => "Array",
            Value::Map(_) => "Map",
            Value::Function(_) => "Function",
            Value::Opaque(_) => "Opaque",
        }
    }
}
