//! Mutable reference cells: Frost's only built-in mutable state.

use std::collections::HashSet;
use std::marker::PhantomData;
use std::{mem, ptr, slice};

use crate::core::value_map;
use crate::native::{MutableBundle, Optional};
use crate::{FrostError, Value, frost_fn, frost_mutable_bundle};

/// Rejects `value` if it holds, at any depth, what could refer back to the cell.
fn forbid_cycle(value: &Value) -> Result<(), FrostError> {
    // The containers looked into so far: one held in many places is looked into once.
    let mut seen = HashSet::new();
    let mut pending = Vec::new();
    forbid_cycle_shallowly(value, &mut seen, &mut pending)?;
    while let Some(contents) = pending.last_mut() {
        match contents.next() {
            Some(value) => forbid_cycle_shallowly(value, &mut seen, &mut pending)?,
            None => {
                pending.pop();
            }
        }
    }
    Ok(())
}

/// Rejects `value` itself if it could refer back to the cell, or, the first time a
/// container is seen, sets its contents aside on `pending` to look into.
fn forbid_cycle_shallowly<'a>(
    value: &'a Value,
    seen: &mut HashSet<Identity<'a>>,
    pending: &mut Vec<Contents<'a>>,
) -> Result<(), FrostError> {
    match value {
        Value::NativeFunction(_) | Value::Closure(_) => Err(FrostError::from_static(
            "A mutable cell may not store a Function value",
        )),
        // We cannot see inside an Opaque, so one that might hold a Value (and through
        // it, this cell) is rejected. One with no drop glue provably holds none: it
        // cannot own the `Arc` behind a Value.
        Value::Opaque(o) if o.needs_drop() => Err(format!(
            "A mutable cell may not store an Opaque {}, which could refer back to the cell",
            o.type_name()
        )
        .into()),
        Value::Array(array) => {
            if seen.insert(Identity::of(&*array.inner)) {
                pending.push(Contents::Elements(array.iter()));
            }
            Ok(())
        }
        Value::Map(map) => {
            if seen.insert(Identity::of(&*map.inner)) {
                pending.push(Contents::Values(map.values()));
            }
            Ok(())
        }
        _ => Ok(()),
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

#[frost_fn]
pub(super) fn mutable_cell(initial: Optional<Value>) -> Result<Value, FrostError> {
    let initial = initial.0.unwrap_or(Value::Null);
    forbid_cycle(&initial)?;
    Ok(MutableBundle::new(Cell(initial))
        .with(Cell::frost_functions)
        .build())
}

/// What a mutable cell holds.
struct Cell(Value);

#[frost_mutable_bundle(prefix = "mutable_cell")]
impl Cell {
    fn get(&self) -> Value {
        self.0.clone()
    }

    fn exchange(&mut self, new_value: Value) -> Result<Value, FrostError> {
        forbid_cycle(&new_value)?;
        Ok(mem::replace(&mut self.0, new_value))
    }
}
