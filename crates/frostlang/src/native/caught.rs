//! [`caught`]: a call's outcome as a value, as Frost's `try_call` reports it.

use crate::{FrostArray, FrostResult, MapKey, Value};

/// `result`, a call's outcome, as Frost's `try_call` reports one: a Map,
/// `{ok: true, value: <the result>}` on success, or `{ok: false, error,
/// trace}` on failure, with `error` the error's
/// [value](crate::FrostError::into_value) and `trace` the names of its
/// [backtrace](crate::FrostError::backtrace)'s frames.
///
/// The backtrace holds the frames the error passed through before reaching
/// the native, so an error the native makes itself has an empty trace.
///
/// A native that runs a callback and reports its outcome, rather than raising
/// its error, returns this, so that scripts handle both alike.
pub fn caught(result: FrostResult) -> Value {
    match result {
        Ok(value) => Value::from_iter([
            (MapKey::from("ok"), Value::Bool(true)),
            (MapKey::from("value"), value),
        ]),
        Err(err) => {
            let trace = err
                .backtrace()
                .iter()
                .map(|frame| Value::from(frame.name()))
                .collect::<FrostArray>();
            Value::from_iter([
                (MapKey::from("ok"), Value::Bool(false)),
                (MapKey::from("error"), err.into_value()),
                (MapKey::from("trace"), Value::Array(trace)),
            ])
        }
    }
}
