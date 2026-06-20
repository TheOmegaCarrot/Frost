mod compare;
mod convert;
mod operators;
mod stringify;

use std::{any::Any, collections::BTreeMap, sync::Arc};

pub use crate::core::error::FrostError;
pub use crate::core::types::float::FrostFloat;
use crate::vm::Closure;
use crate::vm::NativeFunction;

/// The fundamental runtime value type of Frost.
///
/// Every Frost value is one of these variants. `Clone` is cheap:
/// primitive variants copy, heap-backed variants bump a reference count.
///
/// All Values are immutable once created.
#[derive(Clone, Debug)]
pub enum Value {
    /// The absence of a value.
    Null,
    /// A boolean.
    Bool(bool),
    /// A 64-bit signed integer.
    Int(i64),
    /// A 64-bit float, guaranteed non-NaN and non-Infinity.
    Float(FrostFloat),
    /// A binary-safe byte string. Usually valid UTF-8, but not guaranteed.
    String(Arc<[u8]>),
    /// An ordered, immutable sequence of values.
    Array(FrostArray),
    /// An ordered, immutable key-value mapping.
    Map(FrostMap),
    /// A native-backed function.
    NativeFunction(Arc<NativeFunction>),
    /// A Frost closure.
    Closure(Arc<Closure>),
    /// Interpreter-managed opaque data. Native functions downcast to their concrete type.
    Opaque(FrostOpaque),
}

type FrostOpaque = Arc<dyn Any + Send + Sync>;

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Value>();
};

/// A valid Frost map key. Only non-null primitive types may be keys.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapKey {
    Bool(bool),
    Int(i64),
    Float(FrostFloat),
    String(Arc<[u8]>),
}

/// Frost's array type. Immutable once created.
#[derive(Clone, Debug)]
pub struct FrostArray {
    pub(crate) inner: Arc<Vec<Value>>,
}

/// Frost's map type. Immutable once created.
#[derive(Clone, Debug)]
pub struct FrostMap {
    pub(crate) inner: Arc<BTreeMap<MapKey, Value>>,
}

impl Value {
    /// Returns the inner `i64` if this is an `Int`, or `None`.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the inner `f64` if this is a `Float`, or `None`.
    /// If `Some`, guaranteed not to be NaN or infinite.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(f.get()),
            _ => None,
        }
    }

    /// Returns the inner `bool` if this is a `Bool`, or `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the string contents as `&str` if this is a valid UTF-8 `String`, or `None`.
    /// Use [`as_byte_string`](Self::as_byte_string) for strings that may not be valid UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => std::str::from_utf8(s).ok(),
            _ => None,
        }
    }

    /// Returns the raw bytes if this is a `String`, or `None`.
    /// Unlike [`as_str`](Self::as_str), this always succeeds for String values
    /// regardless of UTF-8 validity.
    pub fn as_byte_string(&self) -> Option<&[u8]> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns a reference to the inner `FrostArray` if this is an `Array`, or `None`.
    pub fn as_array(&self) -> Option<&FrostArray> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Returns a reference to the inner `FrostMap` if this is a `Map`, or `None`.
    pub fn as_map(&self) -> Option<&FrostMap> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Returns a reference to the inner `Arc<dyn Any + Send + Sync>` if this is `Opaque`, or `None`.
    pub fn as_opaque(&self) -> Option<&FrostOpaque> {
        match self {
            Self::Opaque(o) => Some(o),
            _ => None,
        }
    }
}
