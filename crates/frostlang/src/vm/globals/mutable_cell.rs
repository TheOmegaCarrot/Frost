//! Mutable reference cells: Frost's only built-in mutable state.

use std::mem;

use crate::core::find;
use crate::native::{MutableBundle, Optional};
use crate::{FrostError, Value, frost_fn, frost_mutable_bundle};

/// Rejects `value` if it holds, at any depth, what could refer back to the cell.
fn forbid_cycle(value: &Value) -> Result<(), FrostError> {
    match find::find(value, could_refer_back) {
        None => Ok(()),
        Some(Value::NativeFunction(_) | Value::Closure(_)) => Err(FrostError::from_static(
            "A mutable cell may not store a Function value",
        )),
        Some(Value::Opaque(o)) => Err(format!(
            "A mutable cell may not store an Opaque {}, which could refer back to the cell",
            o.type_name()
        )
        .into()),
        Some(found) => unreachable!("`could_refer_back` accepts {}", found.type_name()),
    }
}

/// Whether `value` itself could refer back to the cell.
fn could_refer_back(value: &Value) -> bool {
    match value {
        Value::NativeFunction(_) | Value::Closure(_) => true,
        // We cannot see inside an Opaque, so one that might hold a Value (and through
        // it, this cell) is rejected. One with no drop glue provably holds none: it
        // cannot own the `Arc` behind a Value.
        Value::Opaque(o) => o.needs_drop(),
        _ => false,
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
