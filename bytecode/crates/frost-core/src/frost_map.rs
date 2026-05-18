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
    pub fn empty() -> Self {
        Self {
            inner: Arc::from(BTreeMap::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn get(&self, key: &MapKey) -> Option<&Value> {
        self.inner.get(key)
    }

    pub fn contains_key(&self, key: &MapKey) -> bool {
        self.inner.contains_key(key)
    }

    pub fn keys(&self) -> btree_map::Keys<'_, MapKey, Value> {
        self.inner.keys()
    }

    pub fn values(&self) -> btree_map::Values<'_, MapKey, Value> {
        self.inner.values()
    }

    pub fn iter(&self) -> btree_map::Iter<'_, MapKey, Value> {
        self.inner.iter()
    }

    /// Convenience for string-keyed lookups without manually wrapping in MapKey.
    pub fn get_str(&self, key: &str) -> Option<&Value> {
        self.inner.get(&MapKey::String(Arc::from(key.as_bytes())))
    }

    pub fn into_value(self) -> Value {
        Value::from(self)
    }
}
