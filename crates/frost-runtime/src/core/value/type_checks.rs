//! Type classification of a [`Value`]: [`frost_type`](Value::frost_type),
//! [`fits`](Value::fits), and the `is_*` predicates.

use enumset::EnumSet;

use crate::core::{FrostType, Value};

type Ft = FrostType;

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
            Value::Bytes(_) => Ft::Bytes,
            Value::Array(_) => Ft::Array,
            Value::Map(_) => Ft::Map,
            Value::NativeFunction(_) => Ft::Function,
            Value::Closure(_) => Ft::Function,
            Value::Opaque(_) => Ft::Opaque,
        }
    }

    /// Returns true if this value's type is in `types`.
    /// Use with `|`-built sets or the named categories ([`FrostType::NUMERIC`], etc.).
    pub fn fits(&self, types: EnumSet<FrostType>) -> bool {
        types.contains(self.frost_type())
    }

    /// Returns true if this value is a Null.
    pub fn is_null(&self) -> bool {
        self.frost_type() == Ft::Null
    }

    /// Returns true if this value is a Bool.
    pub fn is_bool(&self) -> bool {
        self.frost_type() == Ft::Bool
    }

    /// Returns true if this value is an Int.
    pub fn is_int(&self) -> bool {
        self.frost_type() == Ft::Int
    }

    /// Returns true if this value is a Float.
    pub fn is_float(&self) -> bool {
        self.frost_type() == Ft::Float
    }

    /// Returns true if this value is a String.
    pub fn is_string(&self) -> bool {
        self.frost_type() == Ft::String
    }

    /// Returns true if this value is a Bytes.
    pub fn is_bytes(&self) -> bool {
        self.frost_type() == Ft::Bytes
    }

    /// Returns true if this value is an Array.
    pub fn is_array(&self) -> bool {
        self.frost_type() == Ft::Array
    }

    /// Returns true if this value is a Map.
    pub fn is_map(&self) -> bool {
        self.frost_type() == Ft::Map
    }

    /// Returns true if this value is a Function (native or closure).
    pub fn is_function(&self) -> bool {
        self.frost_type() == Ft::Function
    }

    /// Returns true if this value is an Opaque.
    pub fn is_opaque(&self) -> bool {
        self.frost_type() == Ft::Opaque
    }

    /// Returns true if this value is Int or Float.
    pub fn is_numeric(&self) -> bool {
        self.fits(Ft::NUMERIC)
    }

    /// Returns true if this value is Null, Bool, Int, Float, String, or Bytes.
    pub fn is_primitive(&self) -> bool {
        self.fits(Ft::PRIMITIVE)
    }

    /// Returns true if this value is Array or Map.
    pub fn is_structured(&self) -> bool {
        self.fits(Ft::STRUCTURED)
    }

    /// Returns true if this value is String or Bytes.
    pub fn is_flat(&self) -> bool {
        self.fits(Ft::FLAT)
    }

    /// Returns true if this value is not Null.
    pub fn is_nonnull(&self) -> bool {
        self.fits(Ft::NONNULL)
    }
}
