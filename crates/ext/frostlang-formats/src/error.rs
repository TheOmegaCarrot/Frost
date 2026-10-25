//! The errors of encoding and decoding, shared by every format.

use std::fmt::Display;

use frostlang::FrostError;

/// The error a script sees from the function named `function`, which failed
/// with `error`, an [`EncodeError`] or [`DecodeError`] reading as a predicate.
#[cfg_attr(not(feature = "json"), allow(dead_code))]
pub(crate) fn function_error(function: &str, error: impl Display) -> FrostError {
    FrostError::from_string(format!("Function {function} {error}"))
}

/// Why a value could not be encoded.
///
/// Its [`Display`](std::fmt::Display) reads as a predicate, such as
/// ``cannot encode Bytes in JSON (at `a.b[2]`)``.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EncodeError {
    /// The value holds something the format has no form for.
    #[error("cannot encode {what} in {format}{}", at(path))]
    Unrepresentable {
        /// The format's name, such as `JSON`.
        format: &'static str,
        /// What has no form, such as `Bytes` or `the Int key 1`.
        what: String,
        /// Where it is within the value, as Frost would index it, such as
        /// `a.b[2]`; empty for the value itself.
        path: String,
    },
    /// Arrays and Maps nest deeper than the format allows.
    #[error("cannot encode Arrays and Maps nested more than {limit} deep in {format}")]
    TooDeep {
        /// The format's name, such as `JSON`.
        format: &'static str,
        /// The deepest nesting the format allows.
        limit: usize,
    },
}

/// ` (at `path`)`, or nothing for an empty path.
fn at(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!(" (at `{path}`)")
    }
}

/// Why input could not be decoded: it is malformed, or holds something Frost
/// cannot, such as a Map with a key given twice.
///
/// Its [`Display`](std::fmt::Display) reads as a predicate, such as
/// `cannot decode JSON: duplicate key "a" at line 1 column 12`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("cannot decode {format}: {message}")]
pub struct DecodeError {
    format: &'static str,
    message: String,
}

impl DecodeError {
    #[cfg_attr(not(feature = "json"), allow(dead_code))]
    pub(crate) fn new(format: &'static str, message: impl ToString) -> Self {
        Self {
            format,
            message: message.to_string(),
        }
    }

    /// The format's name, such as `JSON`.
    pub fn format(&self) -> &'static str {
        self.format
    }

    /// What is wrong with the input, and where, as the format's parser
    /// describes it.
    pub fn message(&self) -> &str {
        &self.message
    }
}
