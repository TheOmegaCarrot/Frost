use std::sync::Arc;

use crate::core::value::{
    FrostArray, FrostError, FrostFloat, FrostMap, FrostType, FrostTypeCategory, MapKey, Value,
};

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
            _ => Err(format!("Type {} is not a valid Map key", v.type_name()).into()),
        }
    }
}

impl From<&str> for MapKey {
    fn from(value: &str) -> Self {
        Self::String(value.as_bytes().into())
    }
}

type Ft = FrostType;
type Ftc = FrostTypeCategory;

impl FrostType {
    pub fn name(&self) -> &'static str {
        match self {
            Ft::Null => "Null",
            Ft::Bool => "Bool",
            Ft::Int => "Int",
            Ft::Float => "Float",
            Ft::String => "String",
            Ft::Array => "Array",
            Ft::Map => "Map",
            Ft::Function => "Function",
            Ft::Opaque => "Opaque",
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
        self.frost_type().name()
    }

    pub fn frost_type(&self) -> FrostType {
        match self {
            Value::Null => Ft::Null,
            Value::Bool(_) => Ft::Bool,
            Value::Int(_) => Ft::Int,
            Value::Float(_) => Ft::Float,
            Value::String(_) => Ft::String,
            Value::Array(_) => Ft::Array,
            Value::Map(_) => Ft::Map,
            Value::NativeFunction(_) => Ft::Function,
            Value::Closure(_) => Ft::Function,
            Value::Opaque(_) => Ft::Opaque,
        }
    }

    pub fn fits_category(&self, category: FrostTypeCategory) -> bool {
        match (self.frost_type(), category) {
            (t, Ftc::Exact(c)) => t == c,
            (t, Ftc::NonNull) => !matches!(t, Ft::Null),
            (t, Ftc::Primitive) => {
                matches!(t, Ft::Null | Ft::Int | Ft::Float | Ft::Bool | Ft::String)
            }
            (t, Ftc::Numeric) => matches!(t, Ft::Int | Ft::Float),
            (t, Ftc::Structured) => matches!(t, Ft::Array | Ft::Map),
        }
    }

    /// Returns true if this value is a Null.
    pub fn is_null(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::Null))
    }

    /// Returns true if this value is a Bool.
    pub fn is_bool(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::Bool))
    }

    /// Returns true if this value is an Int.
    pub fn is_int(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::Int))
    }

    /// Returns true if this value is a Float.
    pub fn is_float(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::Float))
    }

    /// Returns true if this value is a String.
    pub fn is_string(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::String))
    }

    /// Returns true if this value is an Array.
    pub fn is_array(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::Array))
    }

    /// Returns true if this value is a Map.
    pub fn is_map(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::Map))
    }

    /// Returns true if this value is a Function (native or closure).
    pub fn is_function(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::Function))
    }

    /// Returns true if this value is an Opaque.
    pub fn is_opaque(&self) -> bool {
        self.fits_category(Ftc::Exact(Ft::Opaque))
    }

    /// Returns true if this value is Int or Float.
    pub fn is_numeric(&self) -> bool {
        self.fits_category(Ftc::Numeric)
    }

    /// Returns true if this value is Null, Bool, Int, Float, or String.
    pub fn is_primitive(&self) -> bool {
        self.fits_category(Ftc::Primitive)
    }

    /// Returns true if this value is Array or Map.
    pub fn is_structured(&self) -> bool {
        self.fits_category(Ftc::Structured)
    }

    /// Returns true if this value is not Null.
    pub fn is_nonnull(&self) -> bool {
        self.fits_category(Ftc::NonNull)
    }

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
}
