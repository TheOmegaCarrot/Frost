use std::cmp::Ordering;
use std::iter::Zip;
use std::slice;
use std::sync::Arc;

use crate::core::{FrostArray, FrostError, Value};

/// How many levels deep ordering recurses before it compares the levels below in a
/// loop. Recursing is faster, so the many shallow values a script orders are ordered
/// by it alone.
const MAX_RECURSION: usize = 32;

impl Value {
    /// Frost ordered comparison, backing `<`, `<=`, `>`, and `>=`.
    /// Returns the [`Ordering`] of two values, or a type error when they are not orderable.
    ///
    /// Arrays order lexicographically: element pairs that are equal (by `==`, or
    /// as an `Int` and `Float` of the same value) are passed over, and the first
    /// remaining pair decides, failing only if it cannot be ordered. With no such
    /// pair, the shorter Array orders first. So two equal Arrays order as equal,
    /// whatever they hold.
    #[inline] // An optimized build otherwise leaves it out of line, measurably slowing `<`.
    pub fn compare(&self, rhs: &Value) -> Result<Ordering, FrostError> {
        match compare_shallowly(self, rhs) {
            Ok(Shallow::Decided(ordering)) => Ok(ordering),
            Ok(Shallow::Arrays(l, r)) => compare_arrays(l, r, MAX_RECURSION),
            Err(Unorderable) => Err(unorderable(self, rhs)),
        }
    }
}

/// The ordering of two Arrays, recursing at most `depth` levels deeper.
fn compare_arrays(l: &FrostArray, r: &FrostArray, depth: usize) -> Result<Ordering, FrostError> {
    let Some(below) = depth.checked_sub(1) else {
        return compare_arrays_iteratively(l, r);
    };
    for (x, y) in l.iter().zip(r.iter()) {
        let ordering = match compare_elements_shallowly(x, y)? {
            Shallow::Decided(ordering) => ordering,
            Shallow::Arrays(x, y) => compare_arrays(x, y, below)?,
        };
        if ordering.is_ne() {
            return Ok(ordering);
        }
    }
    Ok(l.len().cmp(&r.len()))
}

/// The ordering of two Arrays, compared in a loop.
fn compare_arrays_iteratively(l: &FrostArray, r: &FrostArray) -> Result<Ordering, FrostError> {
    let mut pending = vec![Elements::of(l, r)];
    while let Some(elements) = pending.last_mut() {
        match elements.pairs.next() {
            None => {
                let lengths = elements.lengths;
                if lengths.is_ne() {
                    return Ok(lengths);
                }
                pending.pop();
            }
            Some((x, y)) => match compare_elements_shallowly(x, y)? {
                Shallow::Decided(Ordering::Equal) => {}
                Shallow::Decided(ordering) => return Ok(ordering),
                Shallow::Arrays(x, y) => pending.push(Elements::of(x, y)),
            },
        }
    }
    Ok(Ordering::Equal)
}

/// The pairs of two Arrays' elements, and how the Arrays order if every pair is equal.
struct Elements<'a> {
    pairs: Zip<slice::Iter<'a, Value>, slice::Iter<'a, Value>>,
    lengths: Ordering,
}

impl<'a> Elements<'a> {
    fn of(l: &'a FrostArray, r: &'a FrostArray) -> Self {
        Elements {
            pairs: l.iter().zip(r.iter()),
            lengths: l.len().cmp(&r.len()),
        }
    }
}

/// What comparing two values decides without looking into their contents.
enum Shallow<'a> {
    Decided(Ordering),
    /// They order as their elements do.
    Arrays(&'a FrostArray, &'a FrostArray),
}

/// Two values that cannot be ordered.
struct Unorderable;

/// What comparing elements `x` and `y` of two Arrays decides without looking into their
/// contents. Unlike two values compared on their own, two equal elements order as
/// equal, though they may not be orderable.
fn compare_elements_shallowly<'a>(x: &'a Value, y: &'a Value) -> Result<Shallow<'a>, FrostError> {
    match compare_shallowly(x, y) {
        Ok(shallow) => Ok(shallow),
        Err(Unorderable) if x == y => Ok(Shallow::Decided(Ordering::Equal)),
        Err(Unorderable) => Err(unorderable(x, y)),
    }
}

/// What comparing `l` and `r` decides without looking into their contents.
#[inline] // See `Value::compare`.
fn compare_shallowly<'a>(l: &'a Value, r: &'a Value) -> Result<Shallow<'a>, Unorderable> {
    let ordering = match (l, r) {
        (Value::Int(l), Value::Int(r)) => l.cmp(r),
        (Value::Float(l), Value::Float(r)) => l.cmp(r),
        (Value::Int(l), Value::Float(r)) => r.cmp_int(*l).reverse(),
        (Value::Float(l), Value::Int(r)) => l.cmp_int(*r),
        (Value::String(l), Value::String(r)) => l.cmp(r),
        (Value::Bytes(l), Value::Bytes(r)) => l.cmp(r),
        // Two Arrays order as their elements do, which passes over equal elements in
        // turn. Comparing them for equality first would walk everything below again at
        // every level: time quadratic in the depth.
        (Value::Array(l), Value::Array(r)) => {
            if Arc::ptr_eq(&l.inner, &r.inner) {
                Ordering::Equal
            } else {
                return Ok(Shallow::Arrays(l, r));
            }
        }
        _ => return Err(Unorderable),
    };
    Ok(Shallow::Decided(ordering))
}

/// The error for `l` and `r`, which cannot be ordered.
#[cold]
fn unorderable(l: &Value, r: &Value) -> FrostError {
    // Same-type values that have no ordering (Bool, Null, Map, Function, ...) are
    // unorderable, not mismatched; naming the one type reads better than "X and X".
    if l.type_name() == r.type_name() {
        format!("Type {} is not orderable", l.type_name()).into()
    } else {
        format!(
            "Cannot compare incompatible types: {} and {}",
            l.type_name(),
            r.type_name()
        )
        .into()
    }
}
