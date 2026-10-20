//! One statement's preparation and bindings, and its rows as Frost values.

use std::collections::BTreeSet;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};

use frostlang::native::Structured;
use frostlang::{MapKey, SpecialFloat, Value, ValueMap};
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::types::{ToSqlOutput, ValueRef};
use rusqlite::{Batch, Connection, Row, Statement};

use crate::error::{
    binding_count_error, binding_key_type_error, binding_type_error, invalid_column_name_error,
    invalid_text_error, missing_binding_error, nameless_parameter_error, nan_binding_error,
    no_bindings_error, no_statement_error, statements_error, unknown_binding_error,
};
use crate::run::Failure;

/// The one statement of `sql`, failing as `function` if `sql` holds none or
/// more than one.
pub(crate) fn prepare_one<'c>(
    connection: &'c Connection,
    sql: &str,
    function: &str,
) -> Result<Statement<'c>, Failure> {
    let mut statements = Batch::new(connection, sql);
    let Some(statement) = statements.next()? else {
        return Err(no_statement_error(function).into());
    };
    // Anything after it but comments and semicolons is another statement,
    // whether or not it would prepare.
    match statements.next() {
        Ok(None) => Ok(statement),
        Ok(Some(_)) | Err(_) => Err(statements_error(function).into()),
    }
}

/// A parameter of a statement, as errors name it.
#[derive(Clone, Copy)]
enum Parameter<'a> {
    /// The parameter at a position, counting from 1.
    Position(usize),
    /// A named parameter, prefix and all.
    Named(&'a str),
}

impl fmt::Display for Parameter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Position(position) => write!(f, "{position}"),
            Self::Named(name) => f.write_str(name),
        }
    }
}

/// Binds `bindings` to the parameters of `statement`, failing as `function`.
pub(crate) fn bind(
    statement: &mut Statement<'_>,
    bindings: Option<Structured>,
    function: &str,
) -> Result<(), Failure> {
    match bindings {
        None => match statement.parameter_count() {
            0 => Ok(()),
            expected => Err(no_bindings_error(function, expected).into()),
        },
        Some(Structured::Array(bindings)) => {
            bind_positional(statement, bindings.as_slice(), function)
        }
        Some(Structured::Map(bindings)) => bind_named(statement, &bindings, function),
    }
}

/// Binds `bindings` to the parameters of `statement` in order.
fn bind_positional(
    statement: &mut Statement<'_>,
    bindings: &[Value],
    function: &str,
) -> Result<(), Failure> {
    let expected = statement.parameter_count();
    if bindings.len() != expected {
        return Err(binding_count_error(function, expected, bindings.len()).into());
    }
    for (index, value) in (1..).zip(bindings) {
        let value = sql_value(value, Parameter::Position(index), function)?;
        statement.raw_bind_parameter(index, value)?;
    }
    Ok(())
}

/// Binds each parameter of `statement` to the binding of its name, less its
/// prefix, so that `:id`, `@id`, and `$id` all take the binding `id`.
fn bind_named(
    statement: &mut Statement<'_>,
    bindings: &ValueMap,
    function: &str,
) -> Result<(), Failure> {
    let mut used = BTreeSet::new();
    for index in 1..=statement.parameter_count() {
        let parameter = statement
            .parameter_name(index)
            .filter(|parameter| parameter.starts_with([':', '@', '$']))
            .ok_or_else(|| nameless_parameter_error(function))?
            .to_owned();
        let name = &parameter[1..];
        let value = bindings
            .get(&MapKey::from(name))
            .ok_or_else(|| missing_binding_error(function, name, &parameter))?;
        let value = sql_value(value, Parameter::Named(&parameter), function)?;
        statement.raw_bind_parameter(index, value)?;
        used.insert(name.to_owned());
    }
    for key in bindings.keys() {
        match key {
            MapKey::String(name) if used.contains(&**name) => {}
            MapKey::String(name) => return Err(unknown_binding_error(function, name).into()),
            other => {
                let kind = Value::from(other.clone()).type_name();
                return Err(binding_key_type_error(function, kind).into());
            }
        }
    }
    Ok(())
}

/// `value` as SQLite binds it to `parameter`, failing as `function`.
fn sql_value<'v>(
    value: &'v Value,
    parameter: Parameter<'_>,
    function: &str,
) -> Result<ToSqlOutput<'v>, Failure> {
    let value = match value {
        Value::Null => ValueRef::Null,
        Value::Bool(boolean) => ValueRef::Integer(i64::from(*boolean)),
        Value::Int(int) => ValueRef::Integer(*int),
        Value::Float(float) => ValueRef::Real(float.get()),
        Value::String(text) => ValueRef::Text(text.as_bytes()),
        Value::Bytes(bytes) => ValueRef::Blob(bytes),
        other => match other.downcast_opaque::<SpecialFloat>() {
            // SQLite would store NaN as NULL.
            Some(special) if special.get().is_nan() => {
                return Err(nan_binding_error(function, parameter).into());
            }
            Some(special) => ValueRef::Real(special.get()),
            None => return Err(binding_type_error(function, parameter, other.type_name()).into()),
        },
    };
    Ok(ToSqlOutput::Borrowed(value))
}

/// The keys of the row Maps `statement` yields: its column names, failing as
/// `function`.
pub(crate) fn column_keys(
    statement: &Statement<'_>,
    function: &str,
) -> Result<Vec<MapKey>, Failure> {
    (0..statement.column_count())
        .map(|index| {
            // TODO: rusqlite panics on a column name that is not UTF-8,
            // rather than failing. Catch the panic until it fails instead.
            let name = panic::catch_unwind(AssertUnwindSafe(|| statement.column_name(index)))
                .map_err(|_| invalid_column_name_error(function, index + 1))??;
            Ok(MapKey::from(name))
        })
        .collect()
}

/// `row` as a Map of its columns keyed by `keys`, failing as `function`.
///
/// Where columns share a name, the last one's value stands.
pub(crate) fn row_value(row: &Row<'_>, keys: &[MapKey], function: &str) -> Result<Value, Failure> {
    keys.iter()
        .enumerate()
        .map(|(index, key)| {
            let value = match row.get_ref(index)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(int) => Value::Int(int),
                ValueRef::Real(float) => Value::from_f64(float),
                ValueRef::Text(text) => std::str::from_utf8(text)
                    .map(Value::from)
                    .map_err(|_| invalid_text_error(function, key))?,
                ValueRef::Blob(bytes) => Value::from(bytes),
            };
            Ok((key.clone(), value))
        })
        .collect()
}
