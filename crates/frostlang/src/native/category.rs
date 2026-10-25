//! Argument types for Frost's type categories, and for Function: each accepts
//! exactly its category's types, as [`FrostType`]'s constants define them.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::core::deserialize_of_types;
use crate::native::{ArgSite, FromArg};
use crate::vm::expected_types;
use crate::{
    FrostArray, FrostBytes, FrostError, FrostFloat, FrostMap, FrostString, FrostType, TypeSet,
    Value,
};

/// The error converting `value`, which is not of `types`.
fn mismatch(types: TypeSet, value: &Value) -> FrostError {
    FrostError::from_string(format!(
        "Expected {}, got {}",
        expected_types(types),
        value.type_name()
    ))
}

/// Implements, for each category type, [`FromArg`] by its `TryFrom<Value>`
/// (an argument checked against its types always converts), and serde's
/// traits as the `Value` it holds.
macro_rules! category {
    ($($category:ident: $types:expr),* $(,)?) => {$(
        impl FromArg for $category {
            const TYPES: TypeSet = $types;

            fn from_arg(value: Value, _: &ArgSite<'_>) -> Result<Self, FrostError> {
                Self::try_from(value)
            }
        }

        /// Serializes as the value it holds.
        impl Serialize for $category {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                Value::from(self.clone()).serialize(serializer)
            }
        }

        /// Deserializes a value of its category's types, failing for any other.
        impl<'de> Deserialize<'de> for $category {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = deserialize_of_types(deserializer, $types)?;
                Ok(Self::try_from(value).expect("a value of the category's types converts"))
            }
        }
    )*};
}

category!(
    Numeric: FrostType::NUMERIC,
    Flat: FrostType::FLAT,
    Structured: FrostType::STRUCTURED,
    Primitive: FrostType::PRIMITIVE,
    Nonnull: FrostType::NONNULL,
    Function: FrostType::FUNCTION,
);

/// A Numeric argument: an Int or a Float, as given.
///
/// It is the type for accepting either; an `f64` accepts only a Float. Read
/// either as a float with [`to_f64`](Self::to_f64).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Numeric {
    /// An Int.
    Int(i64),
    /// A Float.
    Float(FrostFloat),
}

impl Numeric {
    /// This number as a float, rounding an Int beyond 2^53 to the nearest one.
    pub fn to_f64(self) -> f64 {
        match self {
            Numeric::Int(i) => i as f64,
            Numeric::Float(f) => f.get(),
        }
    }
}

/// Fails for a value neither an Int nor a Float.
impl TryFrom<Value> for Numeric {
    type Error = FrostError;

    fn try_from(value: Value) -> Result<Self, FrostError> {
        match value {
            Value::Int(i) => Ok(Numeric::Int(i)),
            Value::Float(f) => Ok(Numeric::Float(f)),
            other => Err(mismatch(FrostType::NUMERIC, &other)),
        }
    }
}

impl From<Numeric> for Value {
    fn from(numeric: Numeric) -> Self {
        match numeric {
            Numeric::Int(i) => Value::Int(i),
            Numeric::Float(f) => Value::Float(f),
        }
    }
}

/// A Flat argument: a String or Bytes, the flat sequence types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Flat {
    /// A String.
    String(FrostString),
    /// Bytes.
    Bytes(FrostBytes),
}

impl Flat {
    /// The bytes of either: a String's are its UTF-8 encoding.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Flat::String(s) => s.as_bytes(),
            Flat::Bytes(b) => b,
        }
    }
}

/// Fails for a value neither a String nor Bytes.
impl TryFrom<Value> for Flat {
    type Error = FrostError;

    fn try_from(value: Value) -> Result<Self, FrostError> {
        match value {
            Value::String(s) => Ok(Flat::String(s)),
            Value::Bytes(b) => Ok(Flat::Bytes(b)),
            other => Err(mismatch(FrostType::FLAT, &other)),
        }
    }
}

impl From<Flat> for Value {
    fn from(flat: Flat) -> Self {
        match flat {
            Flat::String(s) => Value::String(s),
            Flat::Bytes(b) => Value::Bytes(b),
        }
    }
}

/// A Structured argument: an Array or a Map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Structured {
    /// An Array.
    Array(FrostArray),
    /// A Map.
    Map(FrostMap),
}

/// Fails for a value neither an Array nor a Map.
impl TryFrom<Value> for Structured {
    type Error = FrostError;

    fn try_from(value: Value) -> Result<Self, FrostError> {
        match value {
            Value::Array(a) => Ok(Structured::Array(a)),
            Value::Map(m) => Ok(Structured::Map(m)),
            other => Err(mismatch(FrostType::STRUCTURED, &other)),
        }
    }
}

impl From<Structured> for Value {
    fn from(structured: Structured) -> Self {
        match structured {
            Structured::Array(a) => Value::Array(a),
            Structured::Map(m) => Value::Map(m),
        }
    }
}

/// A Primitive argument: Null, a Bool, an Int, a Float, a String, or Bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Primitive {
    /// Null.
    Null,
    /// A Bool.
    Bool(bool),
    /// An Int.
    Int(i64),
    /// A Float.
    Float(FrostFloat),
    /// A String.
    String(FrostString),
    /// Bytes.
    Bytes(FrostBytes),
}

/// Fails for an Array, a Map, a Function, or an Opaque.
impl TryFrom<Value> for Primitive {
    type Error = FrostError;

    fn try_from(value: Value) -> Result<Self, FrostError> {
        match value {
            Value::Null => Ok(Primitive::Null),
            Value::Bool(b) => Ok(Primitive::Bool(b)),
            Value::Int(i) => Ok(Primitive::Int(i)),
            Value::Float(f) => Ok(Primitive::Float(f)),
            Value::String(s) => Ok(Primitive::String(s)),
            Value::Bytes(b) => Ok(Primitive::Bytes(b)),
            other => Err(mismatch(FrostType::PRIMITIVE, &other)),
        }
    }
}

impl From<Primitive> for Value {
    fn from(primitive: Primitive) -> Self {
        match primitive {
            Primitive::Null => Value::Null,
            Primitive::Bool(b) => Value::Bool(b),
            Primitive::Int(i) => Value::Int(i),
            Primitive::Float(f) => Value::Float(f),
            Primitive::String(s) => Value::String(s),
            Primitive::Bytes(b) => Value::Bytes(b),
        }
    }
}

/// A Nonnull argument: a value of any type but Null.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nonnull(Value);

impl Nonnull {
    /// The value.
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// The value, by ownership.
    pub fn into_value(self) -> Value {
        self.0
    }
}

/// Fails for Null.
impl TryFrom<Value> for Nonnull {
    type Error = FrostError;

    fn try_from(value: Value) -> Result<Self, FrostError> {
        match value {
            Value::Null => Err(mismatch(FrostType::NONNULL, &value)),
            value => Ok(Self(value)),
        }
    }
}

impl From<Nonnull> for Value {
    fn from(nonnull: Nonnull) -> Self {
        nonnull.0
    }
}

/// A Function argument: any Frost function, to call with
/// [`NativeCtx::invoke`](crate::NativeCtx::invoke).
///
/// ```
/// use frostlang::native::Function;
/// use frostlang::{FrostResult, NativeCtx, Value, frost_fn};
///
/// #[frost_fn]
/// fn apply(#[frost(ctx)] ctx: &mut NativeCtx<'_>, function: Function, value: Value) -> FrostResult {
///     ctx.invoke(&function, [value])
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function(Value);

impl Function {
    /// The function, as a value.
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// The function, as a value, by ownership.
    pub fn into_value(self) -> Value {
        self.0
    }
}

/// Fails for a value that is not a Function.
impl TryFrom<Value> for Function {
    type Error = FrostError;

    fn try_from(value: Value) -> Result<Self, FrostError> {
        match value {
            Value::NativeFunction(_) | Value::Closure(_) => Ok(Self(value)),
            other => Err(mismatch(FrostType::FUNCTION, &other)),
        }
    }
}

impl From<Function> for Value {
    fn from(function: Function) -> Self {
        function.0
    }
}
