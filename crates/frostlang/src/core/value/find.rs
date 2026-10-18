//! Searching a value for a value of some kind, however deeply nested, within a
//! bounded native stack.

use std::collections::HashSet;
use std::marker::PhantomData;
use std::{ptr, slice};

use crate::core::{Value, value_map};

/// The first value within `value`, itself included, that is `wanted`, searching depth
/// first, left to right.
///
/// A container held in many places is looked into the first time it is met only, so
/// a value sharing its contents is searched in time linear in the contents of its
/// distinct containers.
pub(crate) fn find(value: &Value, wanted: impl Fn(&Value) -> bool) -> Option<&Value> {
    let mut search = Search {
        wanted,
        looked_into: HashSet::new(),
        pending: Vec::new(),
    };
    if search.visit(value) {
        return Some(value);
    }
    while let Some(contents) = search.pending.last_mut() {
        match contents.next() {
            Some(found) if search.visit(found) => return Some(found),
            Some(_) => {}
            None => {
                search.pending.pop();
            }
        }
    }
    None
}

/// A depth-first search in progress.
struct Search<'a, W> {
    wanted: W,
    /// The containers looked into so far.
    looked_into: HashSet<Identity<'a>>,
    /// The contents still to search, innermost last.
    pending: Vec<Contents<'a>>,
}

impl<'a, W: Fn(&Value) -> bool> Search<'a, W> {
    /// Whether `value` is wanted. If not, and it is a container met for the first
    /// time, sets its contents aside to search.
    fn visit(&mut self, value: &'a Value) -> bool {
        if (self.wanted)(value) {
            return true;
        }
        match value {
            Value::Array(array) if self.looked_into.insert(Identity::of(&*array.inner)) => {
                self.pending.push(Contents::Elements(array.iter()));
            }
            Value::Map(map) if self.looked_into.insert(Identity::of(&*map.inner)) => {
                self.pending.push(Contents::Values(map.values()));
            }
            _ => {}
        }
        false
    }
}

/// Which value something is, by its address.
#[derive(PartialEq, Eq, Hash)]
struct Identity<'a>(*const (), PhantomData<&'a ()>);

impl<'a> Identity<'a> {
    fn of<T>(value: &'a T) -> Self {
        Identity(ptr::from_ref(value).cast(), PhantomData)
    }
}

/// The elements of an Array, or the values of a Map.
enum Contents<'a> {
    Elements(slice::Iter<'a, Value>),
    Values(value_map::Values<'a>),
}

impl<'a> Iterator for Contents<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<&'a Value> {
        match self {
            Contents::Elements(elements) => elements.next(),
            Contents::Values(values) => values.next(),
        }
    }
}
