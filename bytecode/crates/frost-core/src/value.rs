use std::{any::Any, collections::BTreeMap, sync::Arc};

pub use crate::error::FrostError;
pub use crate::frost_float::FrostFloat;

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
    /// A callable function (Frost closure or native).
    Function(Arc<dyn Callable>),
    /// Interpreter-managed opaque data. Native functions downcast to their concrete type.
    Opaque(Arc<dyn Any + Send + Sync>),
}

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

/// A callable Frost function: either a user-defined closure or a native builtin.
pub trait Callable: std::fmt::Debug + Send + Sync {
    /// Invoke this function with the given arguments.
    fn call(&self, args: &[Value]) -> Result<Value, FrostError>;
    /// The display name of this function, used in error messages.
    fn name(&self) -> &str;
}

/// Frost's array type. Immutable once created.
#[derive(Clone, Debug)]
pub struct FrostArray {
    pub(crate) inner: Arc<[Value]>,
}

/// Frost's map type. Immutable once created.
#[derive(Clone, Debug)]
pub struct FrostMap {
    pub(crate) inner: Arc<BTreeMap<MapKey, Value>>,
}
