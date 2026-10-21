//! What the methods running SQL do, once the call's SQL has passed the
//! extension's checks and the host's filter.

use std::sync::atomic::{AtomicU64, Ordering};

use frostlang::native::{Function, Structured};
use frostlang::{FrostError, NativeCtx, Value};
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::{Batch, Connection, Statement};

use crate::error::{statement_text_error, transaction_statement_error};
use crate::statement::{bind, column_keys, is_transaction_statement, prepare_one, row_value};

/// Why a call running SQL failed.
pub(crate) enum Failure {
    /// SQLite's failure, for the call to word.
    Sqlite(rusqlite::Error),
    /// A failure already worded as the call's error.
    Frost(FrostError),
}

impl From<rusqlite::Error> for Failure {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

impl From<FrostError> for Failure {
    fn from(err: FrostError) -> Self {
        Self::Frost(err)
    }
}

/// A call running SQL on its connection.
pub(crate) struct Run<'r, 'c> {
    connection: &'r Connection,
    /// The call's SQL, as checked and filtered.
    sql: &'r str,
    ctx: &'r mut NativeCtx<'c>,
    denials: &'r AtomicU64,
    /// The count of denials when SQLite last began running for the call.
    baseline: u64,
    /// How the error for a transaction statement ends, saying what to do
    /// instead.
    transaction_hint: &'static str,
}

impl<'r, 'c> Run<'r, 'c> {
    /// The call `ctx` makes, running `sql` on `connection`, whose authorizer
    /// counts its denials in `denials`; a transaction statement fails ending
    /// with `transaction_hint`.
    pub(crate) fn new(
        connection: &'r Connection,
        sql: &'r str,
        ctx: &'r mut NativeCtx<'c>,
        denials: &'r AtomicU64,
        transaction_hint: &'static str,
    ) -> Self {
        Self {
            connection,
            sql,
            ctx,
            denials,
            baseline: denials.load(Ordering::Relaxed),
            transaction_hint,
        }
    }

    /// Fails if `sql`'s first statement is a transaction statement, which
    /// only the transaction methods run.
    fn forbid_transaction_statement(&self, sql: &str) -> Result<(), Failure> {
        if is_transaction_statement(sql) {
            return Err(transaction_statement_error(self.name(), self.transaction_hint).into());
        }
        Ok(())
    }

    /// The name of the function making the call.
    fn name(&self) -> &str {
        self.ctx.name()
    }

    /// Whether the connection's authorizer has denied an action since SQLite
    /// last began running for the call.
    pub(crate) fn denied(&self) -> bool {
        self.denials.load(Ordering::Relaxed) != self.baseline
    }

    /// The call's one statement, with `bindings` bound.
    fn prepare(&self, bindings: Option<Structured>) -> Result<Statement<'r>, Failure> {
        self.forbid_transaction_statement(self.sql)?;
        let mut statement = prepare_one(self.connection, self.sql, self.name())?;
        bind(&mut statement, bindings, self.name())?;
        Ok(statement)
    }

    /// Calls `function` back with `args`.
    fn invoke(
        &mut self,
        function: &Function,
        args: impl IntoIterator<Item = Value>,
    ) -> Result<Value, Failure> {
        let result = self.ctx.invoke(function, args);
        // Any denial meanwhile was some other call's, which `function` made
        // and may have caught.
        self.baseline = self.denials.load(Ordering::Relaxed);
        Ok(result?)
    }
}

/// `db.script`: runs every statement of the call's SQL, and counts the rows
/// they changed.
pub(crate) fn script(run: &mut Run<'_, '_>) -> Result<i64, Failure> {
    let connection = run.connection;
    let before = connection.total_changes();
    let mut statements = Batch::new(connection, run.sql);
    while let Some(mut statement) = statements.next()? {
        // Expanding changes only parameters, never the first word.
        let text = statement
            .expanded_sql()
            .ok_or_else(|| statement_text_error(run.name()))?;
        run.forbid_transaction_statement(&text)?;
        step_to_end(&mut statement)?;
    }
    Ok(changes_since(connection, before))
}

/// How many rows `connection` has changed since its count of changes was
/// `before`.
pub(crate) fn changes_since(connection: &Connection, before: u64) -> i64 {
    // The count can be SQLite's 32-bit one, sign-extended, which wraps every
    // 2^32 changes. No one call changes nearly that many rows, so the low 32
    // bits of the difference are exact.
    let changed = (connection.total_changes() as u32).wrapping_sub(before as u32);
    i64::from(changed)
}

/// `db.exec`: runs the call's statement, and counts the rows it changed.
pub(crate) fn exec(run: &mut Run<'_, '_>, bindings: Option<Structured>) -> Result<i64, Failure> {
    let connection = run.connection;
    let mut statement = run.prepare(bindings)?;
    let before = connection.total_changes();
    step_to_end(&mut statement)?;
    // SQLite's count of changes is the last INSERT's, UPDATE's, or DELETE's,
    // kept through any other statement. Only such a statement changing rows
    // changes the total.
    if connection.total_changes() == before {
        Ok(0)
    } else {
        Ok(connection.changes() as i64)
    }
}

/// `db.query`: the rows of the call's statement.
pub(crate) fn query(run: &mut Run<'_, '_>, bindings: Option<Structured>) -> Result<Value, Failure> {
    let mut statement = run.prepare(bindings)?;
    let keys = column_keys(&statement, run.name())?;
    statement
        .raw_query()
        .and_then(|row| row_value(row, &keys, run.name()))
        .collect()
}

/// `db.fold`: the rows of the call's statement, folded by `function`.
pub(crate) fn fold(
    run: &mut Run<'_, '_>,
    bindings: Option<Structured>,
    initial: Value,
    function: &Function,
) -> Result<Value, Failure> {
    let mut statement = run.prepare(bindings)?;
    let keys = column_keys(&statement, run.name())?;
    if !statement.readonly() {
        // A statement that writes holds its own transaction until it ends, so
        // one `function` began meanwhile could never commit. It runs in full
        // first: SQLite makes its changes before returning any row anyway.
        let rows: Vec<Value> = statement
            .raw_query()
            .and_then(|row| row_value(row, &keys, run.name()))
            .collect::<Result<_, Failure>>()?;
        return rows.into_iter().try_fold(initial, |accumulator, row| {
            run.invoke(function, [accumulator, row])
        });
    }
    let mut rows = statement.raw_query();
    let mut accumulator = initial;
    while let Some(row) = rows.next()? {
        let row = row_value(row, &keys, run.name())?;
        accumulator = run.invoke(function, [accumulator, row])?;
    }
    Ok(accumulator)
}

/// Steps `statement` through every row it returns, so that it runs in full.
fn step_to_end(statement: &mut Statement<'_>) -> rusqlite::Result<()> {
    let mut rows = statement.raw_query();
    while rows.next()?.is_some() {}
    Ok(())
}
