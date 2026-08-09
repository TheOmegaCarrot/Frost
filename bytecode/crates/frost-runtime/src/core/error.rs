use std::borrow::Cow;
use std::fmt;

use crate::core::Value;

/// A runtime error produced by Frost code.
///
/// Catchable by Frost's `try_call`. As the error propagates through call frames the VM
/// accumulates function names, readable via [`backtrace`](Self::backtrace).
#[derive(Clone, Debug)]
pub struct FrostError {
    payload: ErrorPayload,
    // Crate-internal: the VM appends frames as the error unwinds. External callers get
    // read-only access via `backtrace()`.
    pub(crate) backtrace: Vec<String>,
}

/// An error's payload: a UTF-8 message (the common case) *xor* an arbitrary thrown value.
#[derive(Clone, Debug)]
enum ErrorPayload {
    /// A UTF-8 message: what a Rust-side error, or a Frost `error("...")`, produces.
    /// A `Cow` so the common static-literal case (`"Division by zero"`) needs no allocation.
    Message(Cow<'static, str>),
    /// An arbitrary thrown Frost value.
    Value(Value),
}

impl FrostError {
    /// Creates an error from a static message, stored without allocating.
    pub fn from_static(message: &'static str) -> Self {
        Self {
            payload: ErrorPayload::Message(Cow::Borrowed(message)),
            backtrace: Vec::new(),
        }
    }

    /// Creates an error from a dynamically-built message (e.g. from `format!`).
    pub fn from_string(message: impl Into<String>) -> Self {
        Self {
            payload: ErrorPayload::Message(Cow::Owned(message.into())),
            backtrace: Vec::new(),
        }
    }

    /// Creates an error from an arbitrary thrown Frost value.
    pub fn from_value(value: Value) -> Self {
        let payload = match value {
            Value::String(text) => ErrorPayload::Message(Cow::Owned(text.to_string())),
            other => ErrorPayload::Value(other),
        };
        Self {
            payload,
            backtrace: Vec::new(),
        }
    }

    /// The accumulated call-frame names, outermost last.
    pub fn backtrace(&self) -> &[String] {
        &self.backtrace
    }

    /// The error's message, for display. Borrows a plain message; renders a value payload.
    pub fn message(&self) -> Cow<'_, str> {
        match &self.payload {
            ErrorPayload::Message(text) => Cow::Borrowed(text.as_ref()),
            ErrorPayload::Value(value) => Cow::Owned(value.to_frost_string()),
        }
    }

    /// The payload as a Frost value: what `try_call` hands the catcher. A plain message
    /// becomes a `String`.
    pub fn into_value(self) -> Value {
        match self.payload {
            ErrorPayload::Message(text) => Value::from(text.as_ref()),
            ErrorPayload::Value(value) => value,
        }
    }
}

impl fmt::Display for FrostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.message())
    }
}

impl std::error::Error for FrostError {}

impl From<String> for FrostError {
    fn from(message: String) -> Self {
        Self::from_string(message)
    }
}

impl From<&'static str> for FrostError {
    fn from(message: &'static str) -> Self {
        Self::from_static(message)
    }
}

pub type FrostResult = Result<Value, FrostError>;
