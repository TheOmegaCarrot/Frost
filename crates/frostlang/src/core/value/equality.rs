use std::iter::Zip;
use std::slice;
use std::sync::Arc;

use crate::core::{FrostArray, Value, ValueMap, value_map};

/// How many levels deep `==` recurses before it compares the levels below in a
/// loop. Recursing is faster, so the many shallow values a script compares are
/// compared by it alone.
const MAX_RECURSION: usize = 32;

/// Frost's `==`, which never fails.
///
/// Values of different variants are never equal, so an `Int` never equals a
/// `Float`. Data compares by content: an `Array` or `Map` element by element. A
/// Function, native or closure, equals only itself: each evaluation of a lambda
/// expression makes a new Function, unequal to any other, even one with the
/// same code and captures. An `Opaque` compares as
/// [`FrostOpaque::equals`](crate::FrostOpaque::equals) describes.
///
/// Which values are the same Function or Opaque handle is decided by the
/// source, never by compiler options: no optimization merges two into one or
/// splits one into several.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        equal(self, other, MAX_RECURSION)
    }
}

impl Eq for Value {}

/// Whether two Arrays are equal, as by Frost's `==`.
pub(crate) fn arrays_equal(l: &FrostArray, r: &FrostArray) -> bool {
    decide(arrays_equal_shallowly(l, r), MAX_RECURSION)
}

/// Whether two Maps are equal, as by Frost's `==`.
pub(crate) fn maps_equal(l: &ValueMap, r: &ValueMap) -> bool {
    decide(maps_equal_shallowly(l, r), MAX_RECURSION)
}

/// Whether `l == r`, recursing at most `depth` levels deeper.
#[inline] // An optimized build otherwise leaves it out of line, measurably slowing `==`.
fn equal(l: &Value, r: &Value, depth: usize) -> bool {
    decide(equal_shallowly(l, r), depth)
}

/// Whether values are equal, given how `equality` leaves them, recursing at most `depth`
/// levels deeper.
#[inline] // See `equal`.
fn decide(equality: Shallow, depth: usize) -> bool {
    match equality {
        Shallow::Decided(decided) => decided,
        Shallow::Containers(containers) => contents_equal(containers, depth),
    }
}

/// Whether the contents of `containers` are equal, recursing at most `depth` levels
/// deeper.
// Apart from `equal`, so that it stays small enough to inline where scalars are compared.
fn contents_equal(containers: Containers, depth: usize) -> bool {
    let Some(below) = depth.checked_sub(1) else {
        return equal_iteratively(containers);
    };
    let pair_equal = |(l, r)| equal(l, r, below);
    // Each kind of container in its own loop, which measures faster than one loop
    // through `Children`.
    match containers {
        Containers::Arrays(l, r) => l.iter().zip(r.iter()).all(pair_equal),
        Containers::Maps(l, r) => l.values().zip(r.values()).all(pair_equal),
    }
}

/// Whether the contents of `containers` are equal, compared in a loop.
fn equal_iteratively(containers: Containers) -> bool {
    let mut pending = vec![containers.children()];
    while let Some(children) = pending.last_mut() {
        match children.next() {
            None => {
                pending.pop();
            }
            Some((l, r)) => match equal_shallowly(l, r) {
                Shallow::Decided(false) => return false,
                Shallow::Decided(true) => {}
                Shallow::Containers(containers) => pending.push(containers.children()),
            },
        }
    }
    true
}

/// What comparing two values decides without looking into their contents.
enum Shallow<'a> {
    Decided(bool),
    /// They are equal if their contents are.
    Containers(Containers<'a>),
}

/// Two Arrays of the same length, or two Maps with the same keys.
enum Containers<'a> {
    Arrays(&'a FrostArray, &'a FrostArray),
    Maps(&'a ValueMap, &'a ValueMap),
}

impl<'a> Containers<'a> {
    /// The pairs of their contents, which are equal if each pair is.
    fn children(self) -> Children<'a> {
        match self {
            Containers::Arrays(l, r) => Children::Elements(l.iter().zip(r.iter())),
            // Maps iterate in key order, so with the same keys, their values pair up
            // by key.
            Containers::Maps(l, r) => Children::Values(Box::new(l.values().zip(r.values()))),
        }
    }
}

/// Pairs of elements of two Arrays, or of values of two Maps at the same keys.
enum Children<'a> {
    Elements(Zip<slice::Iter<'a, Value>, slice::Iter<'a, Value>>),
    // Boxed, as two Map iterators are several times the size of two slice iterators,
    // and a deep comparison holds one `Children` per level.
    Values(Box<Zip<value_map::Values<'a>, value_map::Values<'a>>>),
}

impl<'a> Iterator for Children<'a> {
    type Item = (&'a Value, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Children::Elements(pairs) => pairs.next(),
            Children::Values(pairs) => pairs.next(),
        }
    }
}

/// What comparing `l` and `r` decides without looking into their contents.
fn equal_shallowly<'a>(l: &'a Value, r: &'a Value) -> Shallow<'a> {
    let decided = match (l, r) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Int(l), Value::Int(r)) => l == r,
        (Value::Float(l), Value::Float(r)) => l == r,
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Bytes(l), Value::Bytes(r)) => l == r,
        (Value::Array(l), Value::Array(r)) => return arrays_equal_shallowly(l, r),
        (Value::Map(l), Value::Map(r)) => return maps_equal_shallowly(l, r),
        (Value::NativeFunction(l), Value::NativeFunction(r)) => Arc::ptr_eq(l, r),
        (Value::Closure(l), Value::Closure(r)) => Arc::ptr_eq(l, r),
        (Value::Opaque(l), Value::Opaque(r)) => l.frost_eq(r),
        _ => false,
    };
    Shallow::Decided(decided)
}

fn arrays_equal_shallowly<'a>(l: &'a FrostArray, r: &'a FrostArray) -> Shallow<'a> {
    if Arc::ptr_eq(&l.inner, &r.inner) {
        Shallow::Decided(true)
    } else if l.len() != r.len() {
        Shallow::Decided(false)
    } else {
        Shallow::Containers(Containers::Arrays(l, r))
    }
}

fn maps_equal_shallowly<'a>(l: &'a ValueMap, r: &'a ValueMap) -> Shallow<'a> {
    if std::ptr::eq(l, r) {
        Shallow::Decided(true)
    } else if l.len() != r.len() || !l.keys().eq(r.keys()) {
        Shallow::Decided(false)
    } else {
        Shallow::Containers(Containers::Maps(l, r))
    }
}
