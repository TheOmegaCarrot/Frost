//! Mutable reference cells: Frost's only built-in mutable state.

use std::sync::{Arc, Mutex};

use crate::{Arity, FrostError, Value};

fn forbid_cycle(value: &Value) -> Result<(), FrostError> {
    match value {
        Value::NativeFunction(_) | Value::Closure(_) => Err(FrostError::from_static(
            "A mutable cell may not store a Function value",
        )),
        // Opaque wraps an arbitrary host value that could itself hold a cycle, and we
        // cannot see inside it: reject it wholesale so the guarantee stays sound.
        Value::Opaque(_) => Err(FrostError::from_static(
            "A mutable cell may not store an Opaque value",
        )),
        Value::Array(arr) => arr.iter().try_for_each(forbid_cycle),
        Value::Map(map) => map.values().try_for_each(forbid_cycle),
        _ => Ok(()),
    }
}

pub(super) fn mutable_cell_global() -> Value {
    Value::native("mutable_cell", Arity::Between(0, 1), |_, args| {
        // steal the initial value if present, else default to Null.
        let initial = args.first_mut().map(Value::take).unwrap_or(Value::Null);
        forbid_cycle(&initial)?;

        let get_cell = Arc::new(Mutex::new(initial));
        let exchange_cell = get_cell.clone();

        Ok(Value::map([
            (
                "get",
                Value::native("mutable_cell.get", Arity::Exact(0), move |_, _| {
                    Ok(get_cell.lock().unwrap().clone())
                }),
            ),
            (
                "exchange",
                Value::native("mutable_cell.exchange", Arity::Exact(1), move |_, args| {
                    let new_value = args[0].take();
                    forbid_cycle(&new_value)?;
                    Ok(std::mem::replace(
                        &mut *exchange_cell.lock().unwrap(),
                        new_value,
                    ))
                }),
            ),
        ]))
    })
}
