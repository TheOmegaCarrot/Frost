//! Connections as scripts see them: connection bundles, and the factories that
//! make them.

use std::cell::RefCell;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use enumset::{EnumSet, EnumSetType};
use frostlang::native::{Bundle, Function, Optional, Structured};
use frostlang::{
    FrostError, FrostResult, FrostString, NativeCtx, Value, frost_bundle, frost_closure,
};
use parking_lot::ReentrantMutex;
use rusqlite::Connection;
use rusqlite::hooks::{AuthContext, Authorization};

use crate::Authorizer;
use crate::error::{
    closed_error, denied_error, in_use_error, is_function_denial, nul_error, sqlite_error,
};
use crate::run::{self, Failure, Run};

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
/// # Statements
///
/// `exec`, `query`, and `fold` each run the one statement of their `sql`,
/// which comments and semicolons may surround. SQL holding no statement, or
/// more than one, is an error; [`Method::Script`] runs several.
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
    /// It reads each row only as `function` takes it, so it holds one row at
    /// a time. `function` may use the connection meanwhile, though not close
    /// it. Whether a change `function` makes to the rows being read shows in
    /// the rows still to come is undefined, as SQLite leaves it. An error
    /// from `function` ends the statement, and is the call's error.
    Fold,
    /// `db.last_insert_rowid()` returns the rowid of the row last inserted
    /// on the connection, as SQLite's
    /// [`sqlite3_last_insert_rowid`](https://www.sqlite.org/c3ref/last_insert_rowid.html)
    /// gives it, or `0` if none has been.
    LastInsertRowid,
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
pub struct ScriptConnection {
    connection: Source,
    methods: EnumSet<Method>,
    authorizer: Option<Authorizer>,
    filter: Option<SqlFilter>,
}

/// A host's SQL filter; see [`ScriptConnection::with_sql_filter`].
type SqlFilter = Box<dyn Fn(&str) -> Result<(), FrostError> + Send + Sync>;

impl ScriptConnection {
    /// Exposes `connection`.
    pub fn new(connection: Connection) -> Self {
        Self::holding(Source::Owned(connection))
    }

    /// Exposes `connection`, taken from a pool.
    ///
    /// A pooled connection keeps the state its earlier users left on it, such
    /// as pragmas, temporary tables, attached databases, and the rowid
    /// [`Method::LastInsertRowid`] gives. A factory serving
    /// scripts it trusts less than those users must reset that state, or not
    /// share the pool. A transaction a script leaves open is rolled back
    /// before the connection returns to its pool.
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
    /// connection's authorizer as it was, in use and after.
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
    /// A call failing the extension's own checks, as on a closed connection,
    /// never reaches the filter. Nor does the SQL that SQLite runs for a
    /// statement, such as a trigger's or a view's, which only an
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

    fn holding(connection: Source) -> Self {
        Self {
            connection,
            methods: EnumSet::all(),
            authorizer: None,
            filter: None,
        }
    }

    /// The connection bundle, failing as `function` if SQLite refuses to
    /// install the authorizer.
    pub(crate) fn into_bundle(self, function: &str) -> Result<Value, FrostError> {
        let denials = Arc::new(AtomicU64::new(0));
        let authorized = match self.authorizer {
            Some(authorizer) => {
                let counted = Arc::clone(&denials);
                self.connection
                    .authorizer(Some(move |ctx: AuthContext<'_>| {
                        let judgement = authorizer.judge(ctx);
                        if judgement == Authorization::Deny {
                            counted.fetch_add(1, Ordering::Relaxed);
                        }
                        judgement
                    }))
                    .map_err(|err| sqlite_error(function, &err))?;
                true
            }
            None => false,
        };
        let state = Db {
            connection: ReentrantMutex::new(RefCell::new(Some(Held {
                source: self.connection,
                authorized,
            }))),
            filter: self.filter,
            denials,
        };
        let bundle = self
            .methods
            .iter()
            .fold(Bundle::new(state), |bundle, method| {
                bundle.with(match method {
                    Method::Script => Db::script_method,
                    Method::Exec => Db::exec_method,
                    Method::Query => Db::query_method,
                    Method::Fold => Db::fold_method,
                    Method::LastInsertRowid => Db::last_insert_rowid_method,
                    Method::Close => Db::close_method,
                })
            })
            .build();
        Ok(bundle)
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
enum Source {
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

/// A connection as its bundle holds it.
struct Held {
    source: Source,
    /// Whether the extension installed the connection's authorizer.
    authorized: bool,
}

impl Deref for Held {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        &self.source
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // An owned connection closes, and with it go its authorizer and any
        // open transaction, which SQLite rolls back. A pooled one stays open
        // for its next user, who must inherit neither. The authorizer goes
        // first, lest it deny the rollback. A failure leaves nothing better to
        // do.
        let Source::Pooled(connection) = &self.source else {
            return;
        };
        if self.authorized {
            let _ = connection.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        }
        if !connection.is_autocommit() {
            let _ = connection.execute_batch("ROLLBACK");
        }
    }
}

/// A connection bundle's state: its connection, until closed, and its SQL
/// filter.
struct Db {
    // Reentrant, since a method running a statement may call back into Frost,
    // which may use the same connection on the same thread. The `RefCell` lets
    // `close` take the connection, and tell when a call further up the stack
    // is still using it.
    connection: ReentrantMutex<RefCell<Option<Held>>>,
    filter: Option<SqlFilter>,
    /// How many actions the connection's authorizer has denied.
    denials: Arc<AtomicU64>,
}

impl Db {
    /// Runs `operation` for the function `ctx` runs, once `sql` passes the
    /// extension's checks and the host's filter.
    fn with_sql<T>(
        &self,
        ctx: &mut NativeCtx<'_>,
        sql: &str,
        operation: impl FnOnce(&mut Run<'_, '_>) -> Result<T, Failure>,
    ) -> Result<T, FrostError> {
        let function = ctx.name();
        if sql.contains('\0') {
            return Err(nul_error(function));
        }
        self.with_connection(function, |connection| {
            if let Some(filter) = &self.filter {
                filter(sql)?;
            }
            let mut run = Run::new(connection, sql, ctx, &self.denials);
            operation(&mut run).map_err(|failure| match failure {
                Failure::Frost(err) => err,
                // SQLite words a denied function under another code than
                // `SQLITE_AUTH`, naming it. Its wording alone could be any
                // failure's, so only a denial while SQLite ran confirms it.
                Failure::Sqlite(err) if run.denied() && is_function_denial(&err) => {
                    denied_error(function)
                }
                Failure::Sqlite(err) => sqlite_error(function, &err),
            })
        })
    }

    /// Runs `operation` on the connection, for `function`.
    fn with_connection<T>(
        &self,
        function: &str,
        operation: impl FnOnce(&Connection) -> Result<T, FrostError>,
    ) -> Result<T, FrostError> {
        let lock = self.connection.lock();
        // Only `close` borrows mutably, and only to take the connection out,
        // so no mutable borrow is live here. Every method must keep it so: a
        // mutable borrow held across a call back into Frost would make a call
        // reentering the bundle panic.
        let connection = lock.borrow();
        let connection = connection
            .as_deref()
            .ok_or_else(|| closed_error(function))?;
        operation(connection)
    }
}

#[frost_bundle(prefix = "db", group = script_method)]
impl Db {
    fn script(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        sql: FrostString,
    ) -> Result<i64, FrostError> {
        self.with_sql(ctx, &sql, run::script)
    }
}

#[frost_bundle(prefix = "db", group = exec_method)]
impl Db {
    fn exec(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        sql: FrostString,
        bindings: Optional<Structured>,
    ) -> Result<i64, FrostError> {
        self.with_sql(ctx, &sql, |run| run::exec(run, bindings.0))
    }
}

#[frost_bundle(prefix = "db", group = query_method)]
impl Db {
    fn query(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        sql: FrostString,
        bindings: Optional<Structured>,
    ) -> FrostResult {
        self.with_sql(ctx, &sql, |run| run::query(run, bindings.0))
    }
}

#[frost_bundle(prefix = "db", group = fold_method)]
impl Db {
    fn fold(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        sql: FrostString,
        bindings: Optional<Structured>,
        initial: Value,
        #[frost(unnamed)] function: Function,
    ) -> FrostResult {
        self.with_sql(ctx, &sql, |run| {
            run::fold(run, bindings.0, initial, &function)
        })
    }
}

#[frost_bundle(prefix = "db", group = last_insert_rowid_method)]
impl Db {
    fn last_insert_rowid(&self, #[frost(ctx)] ctx: &mut NativeCtx<'_>) -> Result<i64, FrostError> {
        self.with_connection(ctx.name(), |connection| Ok(connection.last_insert_rowid()))
    }
}

#[frost_bundle(prefix = "db", group = close_method)]
impl Db {
    fn close(&self, #[frost(ctx)] ctx: &mut NativeCtx<'_>) -> Result<(), FrostError> {
        let connection = {
            let lock = self.connection.lock();
            let mut connection = lock
                .try_borrow_mut()
                .map_err(|_| in_use_error(ctx.name()))?;
            connection.take()
        };
        // Dropped only now the lock and borrow are released: releasing the
        // connection may run code that reaches this bundle, which then finds
        // it closed.
        match connection {
            Some(connection) => {
                drop(connection);
                Ok(())
            }
            None => Err(closed_error(ctx.name())),
        }
    }
}
