//! The errors the extension reports.

use std::fmt::Display;

use frostlang::FrostError;
use rusqlite::ffi;

/// The Frost error for the function `function` failing with `err`, worded as
/// the extension's own, for a host's
/// [`connection_factory`](crate::connection_factory) to report a connection it
/// failed to open, as its example does.
///
/// Where SQLite reported the failure, the error gives SQLite's message, then
/// the name of its result code, as in `Function app.open_db failed: unable to
/// open database file: app.db (SQLITE_CANTOPEN)`. Otherwise it gives
/// `rusqlite`'s description. An authorizer's denial, which SQLite reports as
/// `SQLITE_AUTH`, is worded `not authorized (SQLITE_AUTH)` whatever SQLite's
/// message, as an [`Authorizer`](crate::Authorizer)'s are.
pub fn sqlite_error(function: &str, err: &rusqlite::Error) -> FrostError {
    FrostError::from_string(format!("Function {function} failed: {}", describe(err)))
}

/// The error for `function` failing on an action the connection's authorizer
/// denied.
pub(crate) fn denied_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} failed: {}",
        with_code(NOT_AUTHORIZED, ffi::SQLITE_AUTH)
    ))
}

/// How every denial is worded, saying no more, so as not to help a script map
/// out an authorizer's rules.
const NOT_AUTHORIZED: &str = "not authorized";

/// Whether `err` is worded as SQLite words a statement using a function the
/// authorizer denied, which it reports as `SQLITE_ERROR`.
pub(crate) fn is_function_denial(err: &rusqlite::Error) -> bool {
    // Every variant SQLite's failure may arrive as displays its message first.
    err.to_string()
        .starts_with("not authorized to use function: ")
}

/// The error for `function` used on a closed connection.
pub(crate) fn closed_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires an open connection, but it is closed"
    ))
}

/// The error for `function` closing a connection that a call further up the
/// stack is still using.
pub(crate) fn in_use_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} cannot close the connection while it is in use"
    ))
}

/// The error for `function` given SQL containing a NUL character, which SQLite
/// would take as the end of the SQL, silently ignoring the rest.
pub(crate) fn nul_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires SQL without NUL characters"
    ))
}

/// The error for `function` given a path containing a NUL character, which no
/// file's path can contain.
pub(crate) fn nul_path_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires a path without NUL characters"
    ))
}

/// The error for `function` given SQL holding no statement.
pub(crate) fn no_statement_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires SQL of exactly one statement, but it has none"
    ))
}

/// The error for `function` given SQL holding more than one statement.
pub(crate) fn statements_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires SQL of exactly one statement, but it has more than one"
    ))
}

/// The error for `function` given `got` bindings for SQL of `expected`
/// parameters.
pub(crate) fn binding_count_error(function: &str, expected: usize, got: usize) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires {}, but got {got}",
        bindings(expected)
    ))
}

/// The error for `function` given no bindings for SQL of `expected`
/// parameters.
pub(crate) fn no_bindings_error(function: &str, expected: usize) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires {}, but none were given",
        bindings(expected)
    ))
}

/// `count` bindings, in words.
fn bindings(count: usize) -> String {
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} binding{plural}")
}

/// The error for `function` given a Map of bindings for SQL with a parameter
/// not named with a prefix.
pub(crate) fn nameless_parameter_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires an Array of bindings, as the SQL has a parameter not named with :, @, or $"
    ))
}

/// The error for `function` given no binding named `name`, for `parameter`.
pub(crate) fn missing_binding_error(function: &str, name: &str, parameter: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires a binding named '{name}', for parameter {parameter}"
    ))
}

/// The error for `function` given a binding named `name`, which no
/// parameter has.
pub(crate) fn unknown_binding_error(function: &str, name: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} got a binding named '{name}', but the SQL has no parameter of that name"
    ))
}

/// The error for `function` given a Map of bindings with a key of type
/// `kind`, not String.
pub(crate) fn binding_key_type_error(function: &str, kind: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires String keys in a Map of bindings, got {kind}"
    ))
}

/// The error for `function` given a value of type `kind` to bind to
/// `parameter`.
pub(crate) fn binding_type_error(
    function: &str,
    parameter: impl Display,
    kind: &str,
) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires Null, Bool, Int, Float, String, or Bytes for parameter {parameter}, got {kind}"
    ))
}

/// The error for `function` given NaN to bind to `parameter`.
pub(crate) fn nan_binding_error(function: &str, parameter: impl Display) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} cannot bind NaN to parameter {parameter}"
    ))
}

/// The error for `function` reading TEXT that is not UTF-8 from `column`.
pub(crate) fn invalid_text_error(function: &str, column: impl Display) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires TEXT to be UTF-8, but column '{column}' holds TEXT that is not"
    ))
}

/// The error for `function` reading a column, at `position` counting from 1,
/// whose name is not UTF-8.
pub(crate) fn invalid_column_name_error(function: &str, position: usize) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires column names to be UTF-8, but column {position} has a name that is not"
    ))
}

/// `err` as [`sqlite_error`] words it.
fn describe(err: &rusqlite::Error) -> String {
    let message = match err {
        rusqlite::Error::SqliteFailure(failure, message) => match message {
            Some(message) => message.clone(),
            None => ffi::code_to_str(failure.extended_code).to_string(),
        },
        // A failure to prepare SQL, as builds with rusqlite's `modern_sqlite`
        // report it. Its own wording repeats the whole SQL, so it is worded as
        // other builds report it. `bundled` implies `modern_sqlite`; a build
        // with only the latter keeps rusqlite's wording.
        #[cfg(feature = "bundled")]
        rusqlite::Error::SqlInputError { msg, .. } => msg.clone(),
        other => other.to_string(),
    };
    // Every error SQLite reported has its result code as its source, whatever
    // the variant, and whichever variants the build has.
    let code = std::error::Error::source(err)
        .and_then(|source| source.downcast_ref::<ffi::Error>())
        .map(|failure| failure.extended_code);
    match code {
        // SQLite's own wording of a denial can name what was denied.
        Some(code) if code & 0xff == ffi::SQLITE_AUTH => with_code(NOT_AUTHORIZED, code),
        Some(code) => with_code(&message, code),
        None => message,
    }
}

/// `message`, then the name of the result code `code`.
fn with_code(message: &str, code: i32) -> String {
    match primary_code_name(code) {
        Some(name) => format!("{message} ({name})"),
        None => format!("{message} (SQLite result code {code})"),
    }
}

/// The name of the primary result code within `code`, as SQLite's C API
/// names it, if it is one.
fn primary_code_name(code: i32) -> Option<&'static str> {
    macro_rules! names {
        ($($name:ident),* $(,)?) => {
            match code & 0xff {
                $(ffi::$name => Some(stringify!($name)),)*
                _ => None,
            }
        };
    }
    names!(
        SQLITE_ERROR,
        SQLITE_INTERNAL,
        SQLITE_PERM,
        SQLITE_ABORT,
        SQLITE_BUSY,
        SQLITE_LOCKED,
        SQLITE_NOMEM,
        SQLITE_READONLY,
        SQLITE_INTERRUPT,
        SQLITE_IOERR,
        SQLITE_CORRUPT,
        SQLITE_NOTFOUND,
        SQLITE_FULL,
        SQLITE_CANTOPEN,
        SQLITE_PROTOCOL,
        SQLITE_EMPTY,
        SQLITE_SCHEMA,
        SQLITE_TOOBIG,
        SQLITE_CONSTRAINT,
        SQLITE_MISMATCH,
        SQLITE_MISUSE,
        SQLITE_NOLFS,
        SQLITE_AUTH,
        SQLITE_FORMAT,
        SQLITE_RANGE,
        SQLITE_NOTADB,
        SQLITE_NOTICE,
        SQLITE_WARNING,
    )
}
