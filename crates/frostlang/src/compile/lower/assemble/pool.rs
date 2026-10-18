//! The pools assembly places a function's constants in.
//!
//! When deduplicating, constants equal by Frost's `==` share one entry: equal values
//! are indistinguishable, so either may stand for the other.

/// A constant pool under construction.
pub(super) struct Pool<T> {
    entries: Vec<T>,
    deduplicate: bool,
}

impl<T: Clone + PartialEq> Pool<T> {
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
            && let Some(index) = self.entries.iter().position(|entry| entry == constant)
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
