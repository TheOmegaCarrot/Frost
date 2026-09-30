//! The pools assembly places a function's constants in.
//!
//! When deduplicating, constants that are the same share one entry. Same is
//! Frost's `==`, except that a Float's sign of zero counts: `0.0 == -0.0`, but
//! they print differently, so a constant holding one never shares with a
//! constant holding the other.

use frost_runtime::{MapKey, Value};

/// A constant pool under construction.
pub(super) struct Pool<T> {
    entries: Vec<T>,
    deduplicate: bool,
}

impl<T: Constant> Pool<T> {
    pub(super) fn new(deduplicate: bool) -> Self {
        Self {
            entries: Vec::new(),
            deduplicate,
        }
    }

    /// The index of an entry holding `constant`: one already added when
    /// deduplicating, otherwise a new one.
    pub(super) fn add(&mut self, constant: &T) -> usize {
        // A function's pool is small, so a scan beats hashing each constant.
        if self.deduplicate
            && let Some(index) = self
                .entries
                .iter()
                .position(|entry| entry.is_same(constant))
        {
            return index;
        }
        self.entries.push(constant.clone());
        self.entries.len() - 1
    }

    pub(super) fn into_entries(self) -> Vec<T> {
        self.entries
    }
}

/// A constant that can share a pool entry with another it is the same as.
pub(super) trait Constant: Clone {
    fn is_same(&self, other: &Self) -> bool;
}

impl Constant for Value {
    fn is_same(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Float(a), Value::Float(b)) => a.get().to_bits() == b.get().to_bits(),
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a.is_same(b))
            }
            (Value::Map(a), Value::Map(b)) => {
                a.len() == b.len()
                    && a.iter().all(|(key, value)| {
                        b.get_key_value(key)
                            .is_some_and(|(other_key, other_value)| {
                                key.is_same(other_key) && value.is_same(other_value)
                            })
                    })
            }
            _ => self == other,
        }
    }
}

impl Constant for MapKey {
    fn is_same(&self, other: &Self) -> bool {
        match (self, other) {
            (MapKey::Float(a), MapKey::Float(b)) => a.get().to_bits() == b.get().to_bits(),
            _ => self == other,
        }
    }
}
