//! Assertions and value inspection.

use crate::native::Optional;
use crate::{FrostError, Value, frost_fn};

/// `assert(condition, error?)`: return `condition` when it is truthy, else raise.
/// A String `error` is raised as `Failed assertion: {error}`, any other value as
/// is, and no `error` as `Failed assertion`.
#[frost_fn]
pub(super) fn assert(
    #[frost(unnamed)] condition: Value,
    #[frost(unnamed)] error: Optional<Value>,
) -> Result<Value, FrostError> {
    if condition.is_truthy() {
        return Ok(condition);
    }
    Err(match error.0 {
        Some(Value::String(message)) => {
            FrostError::from_string(format!("Failed assertion: {message}"))
        }
        Some(error) => FrostError::from_value(error),
        None => FrostError::from_static("Failed assertion"),
    })
}

#[frost_fn]
pub(super) fn debug_dump(#[frost(unnamed)] value: Value) -> String {
    value.to_debug_string()
}
