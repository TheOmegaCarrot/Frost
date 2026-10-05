use std::{ops::Deref, sync::Arc};

use crate::core::{MapKey, Value, ValueMap, types::value_map};

/// Frost's map type. Immutable once created.
///
/// It reads as a [`ValueMap`], its mutable form, through [`Deref`].
#[derive(Clone, Debug)]
pub struct FrostMap {
    pub(crate) inner: Arc<ValueMap>,
}

impl From<ValueMap> for FrostMap {
    fn from(value: ValueMap) -> Self {
        Self {
            inner: Arc::new(value),
        }
    }
}

impl From<Arc<ValueMap>> for FrostMap {
    fn from(value: Arc<ValueMap>) -> Self {
        Self { inner: value }
    }
}

impl Deref for FrostMap {
    type Target = ValueMap;

    fn deref(&self) -> &ValueMap {
        &self.inner
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
    /// A Map of the entries, each [inserted](ValueMap::insert) in turn.
    fn from_iter<T: IntoIterator<Item = (MapKey, Value)>>(iter: T) -> Self {
        ValueMap::from_iter(iter).into()
    }
}

impl<'a> IntoIterator for &'a FrostMap {
    type Item = (&'a MapKey, &'a Value);
    type IntoIter = value_map::Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl FrostMap {
    /// Creates an empty FrostMap.
    pub fn empty() -> Self {
        ValueMap::new().into()
    }

    /// The String key most like `name`, if one is close enough to be what a
    /// mistyped `name` meant: the fewest single-character edits away, a swap of
    /// neighbouring characters counting as one.
    pub(crate) fn closest_string_key(&self, name: &str) -> Option<&str> {
        let length = name.chars().count();
        // As rustc suggests names: within a third of the name's length, at
        // least one edit.
        let limit = length.max(3) / 3;
        self.keys()
            .filter_map(|key| match key {
                MapKey::String(candidate) => Some(&**candidate),
                _ => None,
            })
            // Each differing character is an edit: skip hopeless candidates cheaply.
            .filter(|candidate| candidate.chars().count().abs_diff(length) <= limit)
            .map(|candidate| (strsim::osa_distance(name, candidate), candidate))
            .filter(|(distance, _)| *distance <= limit)
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, candidate)| candidate)
    }

    /// Converts this map into a Value.
    pub fn into_value(self) -> Value {
        Value::from(self)
    }

    /// Extract a mutable map when not shared, or return the FrostMap as-is.
    /// Zero-copy in the `Ok` case; the fallible counterpart of
    /// [`into_map`](Self::into_map).
    pub fn try_into_map(self) -> Result<ValueMap, FrostMap> {
        match Arc::try_unwrap(self.inner) {
            Ok(map) => Ok(map),
            Err(arc) => Err(FrostMap { inner: arc }),
        }
    }

    /// Extract a [`ValueMap`] from a FrostMap.
    /// Zero-copy when possible, but quietly copies when not.
    pub fn into_map(self) -> ValueMap {
        Arc::unwrap_or_clone(self.inner)
    }
}
