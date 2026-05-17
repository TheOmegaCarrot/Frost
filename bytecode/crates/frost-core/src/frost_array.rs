use std::ops::Index;
use std::sync::Arc;

use crate::value::{FrostArray, Value};

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

impl Default for FrostArray {
    fn default() -> Self {
        Self::empty()
    }
}

impl FrostArray {
    pub fn new(elems: &[Value]) -> Self {
        Self {
            inner: Arc::from(elems),
        }
    }

    pub fn empty() -> Self {
        Self {
            inner: Arc::from([]),
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Value> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

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
}
