//! Mutable reference cells: Frost's only built-in mutable state.

use std::mem;

use crate::native::{MutableBundle, Optional};
use crate::{FrostError, Value, frost_fn, frost_mutable_bundle};

fn forbid_cycle(value: &Value) -> Result<(), FrostError> {
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
        Value::Array(arr) => arr.iter().try_for_each(forbid_cycle),
        Value::Map(map) => map.values().try_for_each(forbid_cycle),
        _ => Ok(()),
    }
}

#[frost_fn(name = "mutable_cell")]
pub(super) fn mutable_cell(
    #[frost(unnamed)] initial: Optional<Value>,
) -> Result<Value, FrostError> {
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
