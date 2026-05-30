use std::fmt;

/// A runtime error produced by Frost code.
///
/// Catchable by Frost's `try_call`. The VM accumulates function names
/// in `backtrace` as the error propagates through call frames.
#[derive(Clone, Debug)]
pub struct FrostError {
    pub message: String,
    pub backtrace: Vec<String>,
}

impl FrostError {
    /// Creates a new error with the given message and an empty backtrace.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            backtrace: Vec::new(),
        }
    }

    /// Pushes a function name onto the backtrace as the error propagates.
    pub fn with_frame(mut self, name: impl Into<String>) -> Self {
        self.backtrace.push(name.into());
        self
    }
}

impl fmt::Display for FrostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

impl std::error::Error for FrostError {}

impl From<String> for FrostError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for FrostError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

use crate::core::Value;

pub type FrostResult = Result<Value, FrostError>;
