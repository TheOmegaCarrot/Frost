mod compare;
mod convert;
mod operators;
mod stringify;
mod type_checks;

use std::sync::Arc;

use crate::core::types::opaque::FrostOpaque;
use crate::core::{FrostArray, FrostFloat, FrostMap};
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
    /// Text, valid UTF-8 by construction.
    String(Arc<str>),
    /// An arbitrary byte sequence, carrying no encoding guarantee.
    Bytes(Arc<[u8]>),
    /// An ordered, immutable sequence of values.
    Array(FrostArray),
    /// An ordered, immutable key-value mapping.
    Map(FrostMap),
    /// A native-backed function.
    NativeFunction(Arc<NativeFunction>),
    /// A Frost closure.
    Closure(Arc<Closure>),
    /// Runtime-managed opaque data. Native functions downcast to their concrete type.
    Opaque(Arc<dyn FrostOpaque>),
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Value>();
};

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

    /// Returns the text if this is a `String`, or `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the bytes if this is a `Bytes`, or `None`.
    /// For the bytes underlying either text or binary, use
    /// [`as_byte_slice`](Self::as_byte_slice).
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Returns the underlying bytes of a `String` or a `Bytes`, or `None` for
    /// anything else.
    pub fn as_byte_slice(&self) -> Option<&[u8]> {
        match self {
            Self::String(s) => Some(s.as_bytes()),
            Self::Bytes(b) => Some(b),
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

    /// Extract the contained FrostArray, if present.
    /// Otherwise returns the original value as-is.
    pub fn try_into_array(self) -> Result<FrostArray, Value> {
        match self {
            Self::Array(a) => Ok(a),
            _ => Err(self),
        }
    }

    /// Returns a reference to the inner `FrostMap` if this is a `Map`, or `None`.
    pub fn as_map(&self) -> Option<&FrostMap> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Extract the contained FrostMap, if present.
    /// Otherwise returns the original value as-is.
    pub fn try_into_map(self) -> Result<FrostMap, Value> {
        match self {
            Self::Map(m) => Ok(m),
            _ => Err(self),
        }
    }

    /// Returns a reference to the inner opaque handle if this is `Opaque`, or `None`.
    ///
    /// For the concrete payload type rather than the handle, use
    /// [`downcast_opaque`](Self::downcast_opaque).
    pub fn as_opaque(&self) -> Option<&Arc<dyn FrostOpaque>> {
        match self {
            Self::Opaque(o) => Some(o),
            _ => None,
        }
    }

    /// Extract the contained opaque handle, if present.
    /// Otherwise returns the original value as-is.
    pub fn try_into_opaque(self) -> Result<Arc<dyn FrostOpaque>, Value> {
        match self {
            Self::Opaque(o) => Ok(o),
            _ => Err(self),
        }
    }

    /// Moves the value out, leaving `Null` in its place.
    ///
    /// The idiom for taking ownership of a value held behind a `&mut`: most often
    /// a native function consuming one of its arguments to steal its backing storage
    /// instead of cloning. Shorthand for `std::mem::replace(&mut value, Value::Null)`.
    pub fn take(&mut self) -> Value {
        std::mem::replace(self, Value::Null)
    }

    /// Wrap host data as an `Opaque` value.
    ///
    /// The usual way to hand a [`FrostOpaque`] instance into Frost; see the
    /// trait for what Frost does (and refuses to do) with it.
    pub fn opaque<T: FrostOpaque>(x: T) -> Value {
        Value::Opaque(Arc::new(x))
    }

    /// Borrows the concrete `T` inside an `Opaque` value: `None` when this is
    /// not `Opaque`, or the payload is some other type.
    pub fn downcast_opaque<T: FrostOpaque>(&self) -> Option<&T> {
        self.as_opaque()?.downcast_ref::<T>()
    }
}
