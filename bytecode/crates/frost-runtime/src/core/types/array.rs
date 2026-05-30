use std::sync::Arc;
use std::{cmp::Ordering, ops::Index};

use crate::core::value::{FrostArray, Value};

impl<'a> IntoIterator for &'a FrostArray {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl FromIterator<Value> for FrostArray {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().collect::<Arc<[Value]>>(),
        }
    }
}

impl Index<usize> for FrostArray {
    type Output = Value;

    fn index(&self, index: usize) -> &Value {
        &self.inner[index]
    }
}

impl From<&[Value]> for FrostArray {
    fn from(slice: &[Value]) -> Self {
        Self::new(slice)
    }
}

impl From<Vec<Value>> for FrostArray {
    fn from(vec: Vec<Value>) -> Self {
        Self {
            inner: Arc::from(vec),
        }
    }
}

impl PartialEq for FrostArray {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner) || self.as_slice() == other.as_slice()
    }
}

impl Eq for FrostArray {}

impl PartialOrd for FrostArray {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if Arc::ptr_eq(&self.inner, &other.inner) {
            Some(Ordering::Equal)
        } else {
            self.inner.iter().partial_cmp(other.inner.iter())
        }
    }
}

impl Default for FrostArray {
    fn default() -> Self {
        Self::empty()
    }
}

impl FrostArray {
    /// Creates a new FrostArray from a slice of values.
    pub fn new(elems: &[Value]) -> Self {
        Self {
            inner: Arc::from(elems),
        }
    }

    /// Creates an empty FrostArray.
    pub fn empty() -> Self {
        Self {
            inner: Arc::from([]),
        }
    }

    /// Returns an iterator over references to the elements.
    pub fn iter(&self) -> std::slice::Iter<'_, Value> {
        self.inner.iter()
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the array has no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the elements as a slice.
    pub fn as_slice(&self) -> &[Value] {
        &self.inner
    }

    /// Frost-semantic indexing: negative indices count from the end,
    /// out-of-bounds returns None.
    pub fn frost_get(&self, index: i64) -> Option<&Value> {
        let len = self.inner.len() as i64;
        let resolved = if index < 0 { index + len } else { index };
        if resolved < 0 || resolved >= len {
            None
        } else {
            Some(&self.inner[resolved as usize])
        }
    }

    /// Converts this array into a Value.
    pub fn into_value(self) -> Value {
        Value::from(self)
    }
}
