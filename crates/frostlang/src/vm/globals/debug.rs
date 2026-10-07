//! Assertions and value inspection.

use crate::{Arity, FrostError, Value};

/// `assert(condition, error?)`: return `condition` when it is truthy, else raise.
/// A String `error` is raised as `Failed assertion: {error}`, any other value as
/// is, and no `error` as `Failed assertion`.
pub(super) fn assert_global() -> Value {
    Value::native("assert", Arity::Between(1, 2), |_, args| {
        if args[0].is_truthy() {
            return Ok(args[0].take());
        }
        Err(match args.get_mut(1).map(Value::take) {
            Some(Value::String(message)) => {
                FrostError::from_string(format!("Failed assertion: {message}"))
            }
            Some(error) => FrostError::from_value(error),
            None => FrostError::from_static("Failed assertion"),
        })
    })
}

pub(super) fn debug_dump_global() -> Value {
    Value::native("debug_dump", Arity::Exact(1), |_, args| {
        Ok(Value::from(args[0].to_debug_string()))
    })
}
