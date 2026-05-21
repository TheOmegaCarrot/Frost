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
    /// Returns `true` if the value is truthy. Only `Null` and `Bool(false)` are falsy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            _ => true,
        }
    }

    /// Returns the Frost type name of this value (e.g. `"Int"`, `"String"`, `"Array"`).
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

    /// Returns true if this value is a Null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns true if this value is a Bool.
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns true if this value is an Int.
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }

    /// Returns true if this value is a Float.
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Returns true if this value is a String.
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns true if this value is an Array.
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Returns true if this value is a Map.
    pub fn is_map(&self) -> bool {
        matches!(self, Value::Map(_))
    }

    /// Returns true if this value is a Function.
    pub fn is_function(&self) -> bool {
        matches!(self, Value::Function(_))
    }

    /// Returns true if this value is an Opaque.
    pub fn is_opaque(&self) -> bool {
        matches!(self, Value::Opaque(_))
    }

    /// Returns true if this value is Int or Float.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Float(_))
    }

    /// Returns true if this value is Null, Bool, Int, Float, or String.
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::String(_)
        )
    }

    /// Returns true if this value is Array or Map.
    pub fn is_structured(&self) -> bool {
        matches!(self, Value::Array(_) | Value::Map(_))
    }

    /// Returns true if this value is not Null.
    pub fn is_nonnull(&self) -> bool {
        !self.is_null()
    }

    /// Frost's `to_int`: Int passes through, Float truncates toward zero,
    /// String parses as an integer. Everything else returns Null.
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

    /// Frost's `to_float`: Float passes through, Int promotes,
    /// String parses as a float. Everything else returns Null.
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
}
