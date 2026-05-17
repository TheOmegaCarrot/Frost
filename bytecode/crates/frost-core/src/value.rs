use std::{collections::BTreeMap, sync::Arc};

use crate::error::FrostError;

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Arc<str>),
    Array(FrostArray),
    Map(FrostMap),
    Function(Arc<dyn Callable>),
}

#[derive(Clone, Debug)]
pub enum MapKey {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Arc<str>),
}

pub trait Callable: std::fmt::Debug + Send + Sync {
    fn call(&self, args: &[Value]) -> Result<Value, FrostError>;
}

/// Frost's array type. Implementation is opaque so the backing
/// data structure can be changed without affecting consumers.
#[derive(Clone, Debug)]
pub struct FrostArray {
    inner: Arc<Vec<Value>>,
}

/// Frost's map type. Implementation is opaque so the backing
/// data structure can be changed (e.g. to a persistent/immutable
/// map) without affecting consumers.
#[derive(Clone, Debug)]
pub struct FrostMap {
    inner: Arc<BTreeMap<MapKey, Value>>,
}
