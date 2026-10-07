use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use itertools::Itertools;

use crate::core::Value;

/// A runtime error produced by Frost code.
///
/// Catchable by Frost's `try_call`. As the error propagates through call frames the VM
/// accumulates the functions it passes through, readable via [`backtrace`](Self::backtrace).
#[derive(Clone, Debug)]
pub struct FrostError {
    payload: ErrorPayload,
    // Crate-internal: the VM appends frames as the error unwinds. External callers get
    // read-only access via `backtrace()`.
    pub(crate) backtrace: Vec<BacktraceFrame>,
}

/// One function a [`FrostError`] passed through as it propagated.
/// [`Display`](fmt::Display) shows its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacktraceFrame {
    name: String,
    origin: Option<Arc<str>>,
}

impl BacktraceFrame {
    pub(crate) fn new(name: String, origin: Option<Arc<str>>) -> Self {
        Self { name, origin }
    }

    /// The function's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The file the function was compiled from, by base name;
    /// see [`CompiledFunction::origin`](crate::CompiledFunction::origin).
    pub fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
    }
}

impl fmt::Display for BacktraceFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// An error's payload: a message (the common case) *xor* an arbitrary thrown value.
#[derive(Clone, Debug)]
enum ErrorPayload {
    /// A message: what a Rust-side error, or a Frost `error("...")`, produces.
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

    /// The accumulated call frames, outermost last.
    pub fn backtrace(&self) -> &[BacktraceFrame] {
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

    /// The error as a person reads it: as [`Display`](fmt::Display) shows it,
    /// then a line `  in <frame>` for each frame of its
    /// [`backtrace`](Self::backtrace), with no trailing newline.
    /// When the frames come from more than one file, each frame with an
    /// [`origin`](BacktraceFrame::origin) also names it: `  in <frame> (<origin>)`.
    pub fn with_backtrace(&self) -> WithBacktrace<'_> {
        WithBacktrace(self)
    }
}

/// Shows the [`message`](FrostError::message) alone, with no label such as
/// `Error:`; adding one is the presenter's choice.
impl fmt::Display for FrostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

/// A [`FrostError`] shown with its backtrace; see
/// [`FrostError::with_backtrace`].
#[derive(Debug, Clone, Copy)]
pub struct WithBacktrace<'a>(&'a FrostError);

impl fmt::Display for WithBacktrace<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;
        let frames = self.0.backtrace();
        let mixed = !frames.iter().filter_map(BacktraceFrame::origin).all_equal();
        frames.iter().try_for_each(|frame| match frame.origin() {
            Some(origin) if mixed => write!(f, "\n  in {frame} ({origin})"),
            _ => write!(f, "\n  in {frame}"),
        })
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

/// The result of a Frost operation: a [`Value`], or the [`FrostError`] it raised.
pub type FrostResult = Result<Value, FrostError>;
