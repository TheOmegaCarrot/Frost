use std::fmt;

#[derive(Clone, Debug)]
pub enum FrostError {
    /// An error caused by user code, which user code can recover from.
    Recoverable(String),

    /// An error caused by user code, which user code cannot recover from.
    /// (Provable bug in user code)
    Unrecoverable(String),

    /// Internal invariant violated. This means there's a bug in the interpreter.
    Internal(String),
}

impl fmt::Display for FrostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrostError::Recoverable(msg) => write!(f, "Error: {msg}"),
            FrostError::Unrecoverable(msg) => write!(f, "Error: {msg}"),
            FrostError::Internal(msg) => write!(f, "INTERNAL ERROR: {msg}"),
        }
    }
}

impl std::error::Error for FrostError {}

use crate::Value;

pub type FrostResult = Result<Value, FrostError>;
