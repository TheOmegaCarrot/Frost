use std::{
    collections::{BTreeMap, btree_map},
    sync::Arc,
};

use crate::{FrostMap, MapKey, value::Value};

impl From<BTreeMap<MapKey, Value>> for FrostMap {
    fn from(value: BTreeMap<MapKey, Value>) -> Self {
        Self {
            inner: Arc::from(value),
        }
    }
}

impl From<Arc<BTreeMap<MapKey, Value>>> for FrostMap {
    fn from(value: Arc<BTreeMap<MapKey, Value>>) -> Self {
        Self { inner: value }
    }
}

impl PartialEq for FrostMap {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner) || self.inner == other.inner
    }
}

impl Eq for FrostMap {}

impl Default for FrostMap {
    fn default() -> Self {
        Self::empty()
    }
}

impl FromIterator<(MapKey, Value)> for FrostMap {
    fn from_iter<T: IntoIterator<Item = (MapKey, Value)>>(iter: T) -> Self {
        Self {
            inner: Arc::new(iter.into_iter().collect()),
        }
    }
}

impl<'a> IntoIterator for &'a FrostMap {
    type Item = (&'a MapKey, &'a Value);
    type IntoIter = btree_map::Iter<'a, MapKey, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl FrostMap {
    /// Creates an empty FrostMap.
    pub fn empty() -> Self {
        Self {
            inner: Arc::from(BTreeMap::new()),
        }
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the value associated with the given key, or None.
    pub fn get(&self, key: &MapKey) -> Option<&Value> {
        self.inner.get(key)
    }

    /// Returns true if the map contains the given key.
    pub fn contains_key(&self, key: &MapKey) -> bool {
        self.inner.contains_key(key)
    }

    /// Returns an iterator over the keys.
    pub fn keys(&self) -> impl Iterator<Item = &MapKey> {
        self.inner.keys()
    }

    /// Returns an iterator over the values.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.inner.values()
    }

    /// Returns an iterator over key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&MapKey, &Value)> {
        self.inner.iter()
    }

    /// Convenience for string-keyed lookups without manually wrapping in MapKey.
    pub fn get_str(&self, key: &str) -> Option<&Value> {
        self.inner.get(&MapKey::String(Arc::from(key.as_bytes())))
    }

    /// Converts this map into a Value.
    pub fn into_value(self) -> Value {
        Value::from(self)
    }
}
