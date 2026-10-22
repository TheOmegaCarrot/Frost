//! `transaction`: running a script's callback in a transaction, or within one
//! in a savepoint.

use std::sync::Arc;

use frostlang::native::{Function, caught};
use frostlang::{FrostError, FrostResult, NativeCtx, Value};

use crate::bundle::{Held, Owner, Shared, transaction_bundle};
use crate::error::{escaping_error, sqlite_error, uncommittable_error};
use crate::run::changes_since;

/// Runs `function` in a transaction, for the function `ctx` runs, through
/// `owner`: a transaction of its own if `owner` is the connection, else a
/// savepoint within `owner`'s.
///
/// Returns the outcome as `try_call` does: the rows changed meanwhile, as
/// SQLite counts them, or the error the transaction failed with. A call that
/// may not use the connection fails, as any other method's does.
pub(crate) fn run(
    shared: &Arc<Shared>,
    ctx: &mut NativeCtx<'_>,
    owner: Owner,
    function: &Function,
) -> FrostResult {
    let name = ctx.name();
    shared.with_connection(ctx, owner, |held, ctx| {
        let outcome = in_transaction(shared, held, ctx, owner, function);
        Ok(caught(outcome.map_err(|err| confined(name, err))))
    })
}

/// Runs `function` in a transaction on `held`, as [`run`] does, returning the
/// rows changed meanwhile.
fn in_transaction(
    shared: &Arc<Shared>,
    held: &Held,
    ctx: &mut NativeCtx<'_>,
    owner: Owner,
    function: &Function,
) -> FrostResult {
    let name = ctx.name();
    let sql = Sql::new(owner);
    held.execute_batch(sql.begin)
        .map_err(|err| sqlite_error(name, &err))?;
    let begun = Begun {
        held,
        sql: &sql,
        committed: false,
    };
    let before = held.total_changes();
    let usable = held.transactions.open();
    ctx.invoke(function, [transaction_bundle(shared, usable.id)])?;
    // The transaction's `tx` is spent once its callback returns.
    drop(usable);
    // SQLite rolled the transaction back itself; see `Shared::with_connection`.
    if held.is_autocommit() {
        return Err(uncommittable_error(name));
    }
    held.execute_batch(sql.commit)
        .map_err(|err| sqlite_error(name, &err))?;
    begun.commit();
    Ok(Value::Int(changes_since(held, before)))
}

/// The SQL beginning, committing, and rolling back a transaction.
struct Sql {
    begin: &'static str,
    commit: &'static str,
    roll_back: &'static str,
}

impl Sql {
    /// The SQL of a transaction made through `owner`.
    fn new(owner: Owner) -> Self {
        match owner {
            // Taking the write lock as it begins, a transaction cannot fail to
            // take it on its first write, which SQLite's busy timeout could
            // not wait out.
            Owner::Connection => Self {
                begin: "BEGIN IMMEDIATE",
                commit: "COMMIT",
                roll_back: "ROLLBACK",
            },
            // SQLite takes a savepoint's name to mean the innermost of that
            // name, so one name serves every level.
            Owner::Transaction(_) => Self {
                begin: "SAVEPOINT frost",
                commit: "RELEASE frost",
                roll_back: "ROLLBACK TO frost; RELEASE frost",
            },
        }
    }
}

/// A begun transaction, which rolls back as this is dropped unless it
/// [committed](Self::commit), however its callback ended, panicking included.
struct Begun<'t> {
    held: &'t Held,
    sql: &'t Sql,
    committed: bool,
}

impl Begun<'_> {
    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for Begun<'_> {
    fn drop(&mut self) {
        // SQLite may have rolled the whole transaction back already. A
        // rollback failing leaves nothing better to do: a pooled connection
        // rolls back any transaction still open as it is released.
        if !self.committed && !self.held.is_autocommit() {
            let _ = self
                .held
                .uninterrupted(|connection| connection.execute_batch(self.sql.roll_back));
        }
    }
}

/// `err`, unless it holds, or may hold, a Function, which could reach the
/// transaction's spent `tx`: then an error saying so, for `function`, with
/// `err`'s message.
fn confined(function: &str, err: FrostError) -> FrostError {
    if err.clone().into_value().may_hold_function() {
        escaping_error(function, &err.message())
    } else {
        err
    }
}
