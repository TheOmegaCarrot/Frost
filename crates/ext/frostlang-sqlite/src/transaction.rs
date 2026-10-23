//! `transaction`: running a script's callback in a transaction, or within one
//! in a savepoint.

use std::sync::Arc;

use frostlang::native::{Function, caught};
use frostlang::{FrostError, FrostResult, NativeCtx, Value};

use crate::bundle::{ConnectionState, OpenConnection, Scope, transaction_bundle};
use crate::error::{escaping_error, sqlite_error, uncommittable_error};
use crate::run::changes_since;

/// Runs `function` in a transaction, for the function `ctx` runs, in `scope`:
/// a transaction of its own if `scope` is autocommit, else a savepoint within
/// `scope`'s transaction.
///
/// Returns the outcome as `try_call` does: the rows changed meanwhile, as
/// SQLite counts them, or the error the transaction failed with. A call that
/// may not use the connection fails, as any other method's does.
pub(crate) fn run(
    state: &Arc<ConnectionState>,
    ctx: &mut NativeCtx<'_>,
    scope: Scope,
    function: &Function,
) -> FrostResult {
    let name = ctx.name();
    state.with_connection(ctx, scope, |connection, ctx| {
        let outcome = in_transaction(state, connection, ctx, scope, function);
        Ok(caught(
            outcome.map_err(|err| confine_to_transaction(name, err)),
        ))
    })
}

/// Runs `function` in a transaction on `connection`, as [`run`] does,
/// returning the rows changed meanwhile.
fn in_transaction(
    state: &Arc<ConnectionState>,
    connection: &OpenConnection,
    ctx: &mut NativeCtx<'_>,
    scope: Scope,
    function: &Function,
) -> FrostResult {
    let name = ctx.name();
    let sql = TransactionSql::new(scope);
    connection
        .execute_batch(sql.begin)
        .map_err(|err| sqlite_error(name, &err))?;
    let guard = RollbackGuard {
        connection,
        sql: &sql,
        committed: false,
    };
    let before = connection.total_changes();
    let innermost = connection.transactions.push();
    ctx.invoke(function, [transaction_bundle(state, innermost.id)])?;
    // The transaction's `tx` is spent once its callback returns.
    drop(innermost);
    // SQLite rolled the transaction back itself; see
    // `ConnectionState::with_connection`.
    if connection.is_autocommit() {
        return Err(uncommittable_error(name));
    }
    connection
        .execute_batch(sql.commit)
        .map_err(|err| sqlite_error(name, &err))?;
    guard.commit();
    Ok(Value::Int(changes_since(connection, before)))
}

/// The SQL beginning, committing, and rolling back a transaction.
struct TransactionSql {
    begin: &'static str,
    commit: &'static str,
    roll_back: &'static str,
}

impl TransactionSql {
    /// The SQL of a transaction made in `scope`.
    fn new(scope: Scope) -> Self {
        match scope {
            // Taking the write lock as it begins, a transaction cannot fail to
            // take it on its first write, which SQLite's busy timeout could
            // not wait out.
            Scope::Autocommit => Self {
                begin: "BEGIN IMMEDIATE",
                commit: "COMMIT",
                roll_back: "ROLLBACK",
            },
            // SQLite takes a savepoint's name to mean the innermost of that
            // name, so one name serves every level.
            Scope::Transaction(_) => Self {
                begin: "SAVEPOINT frost",
                commit: "RELEASE frost",
                roll_back: "ROLLBACK TO frost; RELEASE frost",
            },
        }
    }
}

/// Rolls a begun transaction back as this is dropped, unless it
/// [committed](Self::commit), however its callback ended, panicking included.
struct RollbackGuard<'t> {
    connection: &'t OpenConnection,
    sql: &'t TransactionSql,
    committed: bool,
}

impl RollbackGuard<'_> {
    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for RollbackGuard<'_> {
    fn drop(&mut self) {
        // SQLite may have rolled the whole transaction back already. A
        // rollback failing leaves nothing better to do: a pooled connection
        // rolls back any transaction still open as it is released.
        if !self.committed && !self.connection.is_autocommit() {
            let _ = self
                .connection
                .uninterrupted(|connection| connection.execute_batch(self.sql.roll_back));
        }
    }
}

/// `err`, unless it holds, or may hold, a Function, which could reach the
/// transaction's spent `tx`: then an error saying so, for `function`, with
/// `err`'s message.
fn confine_to_transaction(function: &str, err: FrostError) -> FrostError {
    if err.clone().into_value().may_hold_function() {
        escaping_error(function, &err.message())
    } else {
        err
    }
}
