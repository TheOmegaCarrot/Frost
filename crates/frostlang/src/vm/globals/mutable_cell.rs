//! Mutable reference cells: Frost's only built-in mutable state.

use std::mem;

use crate::native::MutableBundle;
use crate::{Arity, FrostError, Value, frost_mutable_bundle};

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

pub(super) fn mutable_cell_global() -> Value {
    Value::native("mutable_cell", Arity::Between(0, 1), |_, args| {
        // steal the initial value if present, else default to Null.
        let initial = args.first_mut().map_or(Value::Null, Value::take);
        forbid_cycle(&initial)?;
        Ok(Cell::frost_functions(MutableBundle::new(Cell(initial))).build())
    })
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
