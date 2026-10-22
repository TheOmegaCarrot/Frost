//! How a host exposes connections to scripts: [`ScriptConnection`], its
//! [`Method`]s, and the factories that make them.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use enumset::{EnumSet, EnumSetType};
use frostlang::native::Optional;
use frostlang::{FrostError, Value, frost_closure};
use rusqlite::Connection;

use crate::Authorizer;
use crate::bundle::{Held, Shared, connection_bundle};
use crate::error::sqlite_error;

/// A method of a connection bundle, the value through which a script uses a
/// database.
///
/// [`ScriptConnection::with_methods`] chooses a bundle's methods as an
/// [`EnumSet`] of these, which the enum's operators build:
///
/// ```
/// use frostlang_sqlite::{EnumSet, Method};
///
/// let only = Method::Script | Method::Close;
/// let all_but = EnumSet::all() - Method::Close;
/// ```
///
/// Each method's documentation describes it as scripts call it, on a bundle
/// `db`. Any call on a closed connection is an error.
///
/// A call running SQL stops between rows, and between statements, once its
/// run is aborting, as when cancelled; see
/// [`ScriptConnection::with_cancellation`] to stop it within a statement too.
///
/// # Statements
///
/// `exec`, `query`, and `fold` each run the one statement of their `sql`,
/// which comments and semicolons may surround. SQL holding no statement, or
/// more than one, is an error; [`Method::Script`] runs several.
///
/// Scripts make transactions only with [`Method::Transaction`]: SQL holding a
/// transaction statement, such as `BEGIN`, `COMMIT`, or `SAVEPOINT`, is an
/// error in every method.
///
/// A statement changing rows makes all its changes as it begins, as SQLite
/// runs it. A call failing later, on a row it cannot read or in a `fold`'s
/// `function`, leaves those changes made, and outside a transaction,
/// committed.
///
/// # Bindings
///
/// The optional `bindings` of `exec`, `query`, and `fold` give the values of
/// the statement's parameters:
///
/// - An Array binds its values to the parameters in order, and must hold one
///   value for each.
/// - A Map binds each parameter named with `:`, `@`, or `$` to the value keyed
///   by that name less its prefix: `:id`, `@id`, and `$id` all take the value
///   of `id`. Every parameter needs its key and every key its parameter, and
///   keys are Strings. A Map cannot bind a parameter without such a name, such
///   as `?` or `?1`.
///
/// Leaving `bindings` out binds no values.
///
/// | Frost value | SQLite value |
/// |---|---|
/// | Null | NULL |
/// | Bool | INTEGER, `1` or `0` |
/// | Int | INTEGER |
/// | Float | REAL |
/// | String | TEXT |
/// | Bytes | BLOB |
///
/// An infinite [`SpecialFloat`](frostlang::SpecialFloat) binds as REAL too.
/// Any other value, NaN among them, is an error.
///
/// # Rows
///
/// A row is a Map from each column's name to its value. Columns sharing a name
/// leave the last one's value, so a script names columns apart with `AS`.
///
/// | SQLite value | Frost value |
/// |---|---|
/// | NULL | Null |
/// | INTEGER | Int |
/// | REAL | Float, or a [`SpecialFloat`](frostlang::SpecialFloat) if infinite |
/// | TEXT | String |
/// | BLOB | Bytes |
///
/// TEXT that is not UTF-8 is an error, and so is a column name that is not.
/// For a column name, the error comes of a panic in `rusqlite` that the
/// extension catches: the panic hook still runs, by default printing the panic
/// to stderr, and a build with `panic = "abort"` aborts.
#[derive(Debug, Hash, PartialOrd, Ord, EnumSetType)]
#[non_exhaustive]
pub enum Method {
    /// `db.script(sql)` runs the statements of `sql`, separated by
    /// semicolons, and returns how many rows they changed in all, counting
    /// rows changed by triggers and foreign key actions. It takes no bindings.
    /// Outside a transaction, each statement commits on its own, so a
    /// statement failing leaves those before it committed.
    Script,
    /// `db.exec(sql, bindings?)` runs a statement and returns how many rows
    /// it changed, as SQLite's
    /// [`sqlite3_changes`](https://www.sqlite.org/c3ref/changes.html) counts
    /// them: the rows an INSERT, UPDATE, or DELETE changes itself, not those
    /// its triggers or foreign key actions change, and `0` for any other
    /// statement. Dropping a table that other tables' foreign keys reference
    /// first deletes its rows, which count.
    Exec,
    /// `db.query(sql, bindings?)` runs a statement and returns its rows, as
    /// an Array.
    Query,
    /// `db.fold(sql, bindings?, initial, function)` runs a statement and
    /// folds its rows: it calls `function(accumulator, row)` for each row in
    /// turn, with `initial` as the first accumulator and each call's result
    /// as the next, and returns the last result, or `initial` for no rows.
    ///
    /// A statement that only reads is read row by row, as `function` takes
    /// each, so the fold holds one row at a time. A statement that changes
    /// rows and returns them, with SQLite's
    /// [`RETURNING`](https://www.sqlite.org/lang_returning.html) clause, runs
    /// in full before `function` takes its first row, so that `function` may
    /// begin a transaction; the fold then holds every row.
    ///
    /// `function` may use the connection meanwhile, though not close it.
    /// Whether a change `function` makes to the rows being read shows in the
    /// rows still to come is undefined, as SQLite leaves it. A transaction
    /// `function` rolls back having changed the schema ends the read, and the
    /// fold fails. An error from `function` ends the statement, and is the
    /// call's error.
    Fold,
    /// `db.last_insert_rowid()` returns the rowid of the row last inserted
    /// on the connection, as SQLite's
    /// [`sqlite3_last_insert_rowid`](https://www.sqlite.org/c3ref/last_insert_rowid.html)
    /// gives it, or `0` if none has been.
    LastInsertRowid,
    /// `db.transaction(function)` calls `function(tx)` in a transaction,
    /// committed once `function` returns, and rolled back if `function` or
    /// the commit fails. It returns as `try_call` does: `{ok: true, value:
    /// rows}`, with `rows` how many rows changed meanwhile as `script` counts
    /// them, rows an inner transaction rolled back included; or `{ok: false,
    /// error, trace}`. A script ignoring the result ignores a failure too.
    /// What `function` returns is dropped.
    ///
    /// A transaction takes the database's write lock as it begins, so that
    /// its first write cannot fail to take it. It cannot begin on a
    /// connection with `PRAGMA query_only` set.
    ///
    /// `tx` has the connection's methods but `close`, run within the
    /// transaction. `tx.transaction(function)` works alike, as a savepoint
    /// within the open transaction: failing, it rolls back only its own work,
    /// and the outer transaction goes on.
    ///
    /// While a transaction is open, `db` cannot be used, and of the open
    /// transactions only the innermost one's `tx`. A `tx` whose transaction
    /// has ended cannot be used either, nor can one SQLite itself rolled back,
    /// as on an `OR ROLLBACK` conflict, which ends every savepoint within the
    /// transaction too. A call that cannot be used so fails, as one on a
    /// closed connection does. And no `tx` leaves its transaction: an error
    /// that may hold a Function, as [`Value::may_hold_function`] decides, is
    /// returned as one saying so.
    ///
    /// Without this method, a connection's scripts make no transactions:
    /// each statement commits on its own.
    ///
    /// [`Value::may_hold_function`]: frostlang::Value::may_hold_function
    Transaction,
    /// `db.close()` closes the connection, as described on
    /// [`ScriptConnection`].
    Close,
}

/// A connection as scripts see it: the connection, and how its bundle exposes
/// it.
///
/// A host makes one in the factory it gives [`connection_factory`].
///
/// A connection stays open while a script holds any function of its bundle.
/// It is released when a script closes it with `db.close()`, or once no script
/// holds any of those functions: an owned connection is closed, and a pooled
/// one returned to its pool.
///
/// A connection serves one call at a time. A call from another thread waits
/// while one runs: a `fold` or `transaction` until its function returns.
pub struct ScriptConnection {
    connection: Source,
    methods: EnumSet<Method>,
    authorizer: Option<Authorizer>,
    filter: Option<SqlFilter>,
    cancellable: bool,
}

/// A host's SQL filter; see [`ScriptConnection::with_sql_filter`].
pub(crate) type SqlFilter = Box<dyn Fn(&str) -> Result<(), FrostError> + Send + Sync>;

impl ScriptConnection {
    /// Exposes `connection`.
    pub fn new(connection: Connection) -> Self {
        Self::holding(Source::Owned(connection))
    }

    /// Exposes `connection`, taken from a pool.
    ///
    /// A pooled connection keeps the state its earlier users left on it, such
    /// as pragmas, temporary tables, attached databases, and the rowid
    /// [`Method::LastInsertRowid`] gives. A factory serving scripts it trusts
    /// less than those users must reset that state, or not share the pool.
    /// Scripts leave no transaction open: each ends with its
    /// [`Method::Transaction`] call, and the connection rolls back any other
    /// as it returns to its pool.
    pub fn pooled(connection: impl DerefMut<Target = Connection> + Send + 'static) -> Self {
        Self::holding(Source::Pooled(Box::new(connection)))
    }

    /// Exposes only `methods`, rather than every [`Method`], the default.
    ///
    /// Methods choose how scripts reach the connection, not what their SQL
    /// may do: `query` and `fold` run any statement, writes included. An
    /// [`Authorizer`] or a read-only connection limits the SQL itself.
    ///
    /// A set built from `EnumSet::all()`, such as `EnumSet::all() -
    /// Method::Close`, takes in methods that later versions add. A set listing
    /// its methods, such as `Method::Script | Method::Close`, never does.
    #[must_use]
    pub fn with_methods(self, methods: EnumSet<Method>) -> Self {
        Self { methods, ..self }
    }

    /// Checks every statement the connection's scripts run with `authorizer`,
    /// replacing any authorizer given before.
    ///
    /// SQLite gives a connection one authorizer, so this replaces any the
    /// connection had, for good: a pooled connection returns to its pool with
    /// no authorizer at all. Without this, the extension leaves the
    /// connection's authorizer as it was, in use and after. Such an authorizer
    /// must allow beginning, ending, and saving transactions, which only
    /// [`Method::Transaction`] does, as an [`Authorizer`] always does: denied,
    /// transactions fail, and a rollback denied leaves one open.
    #[must_use]
    pub fn with_authorizer(self, authorizer: Authorizer) -> Self {
        Self {
            authorizer: Some(authorizer),
            ..self
        }
    }

    /// Passes the SQL of every call a script makes to `filter`, exactly as the
    /// script gave it, before SQLite sees it, replacing any filter given
    /// before. An error from `filter` is the call's error.
    ///
    /// A call the extension refuses before reading its SQL, as on a closed
    /// connection, never reaches the filter. Nor does the SQL that SQLite runs
    /// for a statement, such as a trigger's or a view's, which only an
    /// [`Authorizer`] judges.
    ///
    /// ```
    /// use frostlang::FrostError;
    /// use frostlang_sqlite::ScriptConnection;
    /// use frostlang_sqlite::rusqlite::Connection;
    ///
    /// # fn main() -> Result<(), frostlang_sqlite::rusqlite::Error> {
    /// let connection = ScriptConnection::new(Connection::open_in_memory()?).with_sql_filter(|sql| {
    ///     if sql.len() > 10_000 {
    ///         Err(FrostError::from_static("SQL is limited to 10000 bytes"))
    ///     } else {
    ///         Ok(())
    ///     }
    /// });
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn with_sql_filter(
        self,
        filter: impl Fn(&str) -> Result<(), FrostError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            filter: Some(Box::new(filter)),
            ..self
        }
    }

    /// Lets a run's cancellation interrupt a statement the run is running on
    /// the connection.
    ///
    /// Without this, a call stops only between rows and statements (see
    /// [`Method`]), so a cancelled run waits for a long statement, such as an
    /// accidental cartesian join, to return a row or end. With it, the
    /// statement stops shortly after the run's
    /// [`CancelToken`](frostlang::CancelToken) is cancelled, and the run fails
    /// as cancelled. A statement waiting for another connection's lock still
    /// waits out SQLite's busy timeout. Fuel bounds no statement either way:
    /// SQLite's work spends none.
    ///
    /// This installs a SQLite progress handler that checks the run's token as
    /// SQLite works. SQLite gives a connection one progress handler, so this
    /// replaces any the connection had, for good: a pooled connection returns
    /// to its pool with none. Without this, the extension leaves the
    /// connection's progress handler as it was.
    #[must_use]
    pub fn with_cancellation(self) -> Self {
        Self {
            cancellable: true,
            ..self
        }
    }

    fn holding(connection: Source) -> Self {
        Self {
            connection,
            methods: EnumSet::all(),
            authorizer: None,
            filter: None,
            cancellable: false,
        }
    }

    /// The connection bundle, failing as `function` if SQLite refuses to
    /// install the authorizer or progress handler.
    pub(crate) fn into_bundle(self, function: &str) -> Result<Value, FrostError> {
        // Held first, so that a failure to install one handler still removes
        // any installed before it.
        let mut held = Held::new(self.connection);
        let denials = Arc::new(AtomicU64::new(0));
        if let Some(authorizer) = self.authorizer {
            held.authorize(authorizer, &denials)
                .map_err(|err| sqlite_error(function, &err))?;
        }
        if self.cancellable {
            held.watch_cancellation()
                .map_err(|err| sqlite_error(function, &err))?;
        }
        let shared = Shared::new(held, self.filter, denials, self.methods);
        Ok(connection_bundle(shared))
    }
}

impl From<Connection> for ScriptConnection {
    fn from(connection: Connection) -> Self {
        Self::new(connection)
    }
}

impl fmt::Debug for ScriptConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptConnection")
            .field("methods", &self.methods)
            .field("authorizer", &self.authorizer)
            .field("filters_sql", &self.filter.is_some())
            .field("cancellable", &self.cancellable)
            .finish_non_exhaustive()
    }
}

/// The Frost function, named `name`, that makes a connection bundle each time
/// it is called, from the [`ScriptConnection`] that `factory` makes.
///
/// This is how a host gives scripts the databases it chooses, configured as it
/// chooses, without the `ext.sqlite` extension. Placed in a
/// [`HostComponent`](frostlang::HostComponent), the function gives each call a
/// connection of its own:
///
/// ```
/// use frostlang::HostComponent;
/// use frostlang_sqlite::rusqlite::{Connection, OpenFlags};
/// use frostlang_sqlite::{EnumSet, Method, ScriptConnection, connection_factory, sqlite_error};
///
/// let open_db = connection_factory("app.open_db", |_| {
///     let connection = Connection::open_with_flags("app.db", OpenFlags::SQLITE_OPEN_READ_ONLY)
///         .map_err(|err| sqlite_error("app.open_db", &err))?;
///     Ok(ScriptConnection::new(connection).with_methods(EnumSet::all() - Method::Close))
/// });
/// let app = HostComponent::builder("app")
///     .value("open_db", open_db)
///     .build();
/// ```
///
/// A script calling `import('app').open_db()` then gets a read-only
/// connection it cannot close.
///
/// The function takes one optional argument, of any type, which `factory`
/// receives: a host may let scripts choose among databases by name, say. A
/// factory taking no argument can reject one. An error from `factory` is the
/// call's error.
///
/// The extension configures nothing on the connections `factory` makes. Each
/// `:memory:` connection is a database of its own, so separate calls do not
/// share an in-memory database.
pub fn connection_factory<F>(name: &'static str, factory: F) -> Value
where
    F: Fn(Option<Value>) -> Result<ScriptConnection, FrostError> + Send + Sync + 'static,
{
    frost_closure!(name, move |argument: Optional<Value>| {
        factory(argument.0)?.into_bundle(name)
    })
}

/// Where a [`ScriptConnection`]'s connection comes from.
pub(crate) enum Source {
    Owned(Connection),
    Pooled(Box<dyn DerefMut<Target = Connection> + Send>),
}

impl Deref for Source {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        match self {
            Self::Owned(connection) => connection,
            Self::Pooled(connection) => connection,
        }
    }
}
