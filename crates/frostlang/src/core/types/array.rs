use std::fmt;
use std::ops::{Deref, DerefMut, Index};
use std::sync::Arc;

use crate::core::{Value, equality, free};

/// Frost's array type. Immutable once created.
#[derive(Clone, Debug)]
pub struct FrostArray {
    pub(crate) inner: Arc<Elements>,
}

/// An Array's elements, freed by [`free::free`].
#[derive(Clone)]
pub(crate) struct Elements(Vec<Value>);

impl Elements {
    fn into_vec(mut self) -> Vec<Value> {
        std::mem::take(&mut self.0)
    }
}

impl Drop for Elements {
    fn drop(&mut self) {
        free::free(std::mem::take(&mut self.0), Vec::into_iter);
    }
}

impl Deref for Elements {
    type Target = Vec<Value>;

    fn deref(&self) -> &Vec<Value> {
        &self.0
    }
}

impl DerefMut for Elements {
    fn deref_mut(&mut self) -> &mut Vec<Value> {
        &mut self.0
    }
}

impl fmt::Debug for Elements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> IntoIterator for &'a FrostArray {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl FromIterator<Value> for FrostArray {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
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
        Vec::from(slice).into()
    }
}

impl From<Vec<Value>> for FrostArray {
    fn from(vec: Vec<Value>) -> Self {
        Self {
            inner: Arc::new(Elements(vec)),
        }
    }
}

impl PartialEq for FrostArray {
    fn eq(&self, other: &Self) -> bool {
        equality::arrays_equal(self, other)
    }
}

impl Eq for FrostArray {}

impl Default for FrostArray {
    fn default() -> Self {
        Self::empty()
    }
}

impl FrostArray {
    /// Creates an empty FrostArray.
    pub fn empty() -> Self {
        Vec::new().into()
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
        self.resolve_index(index).map(|i| &self.inner[i])
    }

    /// The position Frost-semantic `index` names, as [`frost_get`](Self::frost_get)
    /// reads it, or None when it is out of bounds.
    pub(crate) fn resolve_index(&self, index: i64) -> Option<usize> {
        let len = self.inner.len() as i64;
        let resolved = if index < 0 { index + len } else { index };
        if resolved < 0 || resolved >= len {
            None
        } else {
            Some(resolved as usize)
        }
    }

    /// Converts this array into a Value.
    pub fn into_value(self) -> Value {
        Value::from(self)
    }

    /// Extract a mutable array when not shared, or return the FrostArray as-is.
    /// Zero-copy in the `Ok` case; the fallible counterpart of
    /// [`into_vec`](Self::into_vec).
    pub fn try_into_vec(self) -> Result<Vec<Value>, FrostArray> {
        match Arc::try_unwrap(self.inner) {
            Ok(elements) => Ok(elements.into_vec()),
            Err(arc) => Err(FrostArray { inner: arc }),
        }
    }

    /// Extract a Vec from a FrostArray.
    /// Zero-copy when possible, but quietly copies when not.
    pub fn into_vec(self) -> Vec<Value> {
        Arc::unwrap_or_clone(self.inner).into_vec()
    }
}
