//! The functions of `ext.sqlite` that open a database: its own connection
//! factory.

use frostlang::{FrostResult, FrostString, NativeCtx, frost_fn};
use rusqlite::{Connection, OpenFlags};

use crate::error::nul_path_error;
use crate::{ScriptConnection, sqlite_error};

/// The flags of every connection the extension opens, beyond its access.
///
/// - `NO_MUTEX`: the connection's bundle holds its own lock, so SQLite need
///   not hold one too.
/// - `URI`: a path may be a `file:` URI, whichever way the library's build
///   sets the default. A URI cannot loosen the access the flags give.
const COMMON: OpenFlags = OpenFlags::SQLITE_OPEN_NO_MUTEX.union(OpenFlags::SQLITE_OPEN_URI);

/// Reading and writing, creating the database if it does not exist.
const READ_WRITE: OpenFlags = OpenFlags::SQLITE_OPEN_READ_WRITE
    .union(OpenFlags::SQLITE_OPEN_CREATE)
    .union(COMMON);

#[frost_fn]
pub(crate) fn open(#[frost(ctx)] ctx: &mut NativeCtx<'_>, path: FrostString) -> FrostResult {
    connection_bundle(ctx, &path, READ_WRITE)
}

#[frost_fn]
pub(crate) fn open_readonly(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    path: FrostString,
) -> FrostResult {
    connection_bundle(ctx, &path, OpenFlags::SQLITE_OPEN_READ_ONLY.union(COMMON))
}

#[frost_fn]
pub(crate) fn open_memory(#[frost(ctx)] ctx: &mut NativeCtx<'_>) -> FrostResult {
    connection_bundle(ctx, ":memory:", READ_WRITE)
}

/// The bundle over the database at `path`, opened with `flags`, enforcing
/// foreign keys, and cancellable, for the function `ctx` runs.
fn connection_bundle(ctx: &NativeCtx<'_>, path: &str, flags: OpenFlags) -> FrostResult {
    if path.contains('\0') {
        return Err(nul_path_error(ctx.name()));
    }
    let fail = |err| sqlite_error(ctx.name(), &err);
    let connection = Connection::open_with_flags(path, flags).map_err(fail)?;
    // SQLite reads nothing on opening, so a file that is not a database would
    // otherwise open, and fail only on first use. A new file, or an empty
    // one, is an empty database.
    connection
        .pragma_query_value(None, "schema_version", |_| Ok(()))
        .map_err(fail)?;
    // Whatever the library's default, as its build sets it.
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(fail)?;
    ScriptConnection::new(connection)
        .with_cancellation()
        .into_bundle(ctx.name())
}
