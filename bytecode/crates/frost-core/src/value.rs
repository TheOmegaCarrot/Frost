use std::{collections::BTreeMap, sync::Arc, any::Any};

pub use crate::error::FrostError;
pub use crate::frost_float::FrostFloat;

/// The basic value type of Frost.
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(FrostFloat),
    String(Arc<str>),
    Array(FrostArray),
    Map(FrostMap),
    Function(Arc<dyn Callable>),
    Opaque(Arc<dyn Any>),
}

/// A Frost Map key, only a subset of types.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapKey {
    Bool(bool),
    Int(i64),
    Float(FrostFloat),
    String(Arc<str>),
}

pub trait Callable: std::fmt::Debug + Send + Sync {
    fn call(&self, args: &[Value]) -> Result<Value, FrostError>;
}

/// Frost's array type. Implementation is opaque so the backing
/// data structure can be changed without affecting consumers.
#[derive(Clone, Debug)]
pub struct FrostArray {
    pub(crate) inner: Arc<Vec<Value>>,
}

/// Frost's map type. Implementation is opaque so the backing
/// data structure can be changed (e.g. to a persistent/immutable
/// map) without affecting consumers.
#[derive(Clone, Debug)]
pub struct FrostMap {
    pub(crate) inner: Arc<BTreeMap<MapKey, Value>>,
}
