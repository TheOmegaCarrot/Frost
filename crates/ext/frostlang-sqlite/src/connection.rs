//! Connections as scripts see them: connection bundles, and the factories that
//! make them.

use std::cell::RefCell;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use enumset::{EnumSet, EnumSetType};
use frostlang::native::{Bundle, Optional};
use frostlang::{FrostError, FrostString, NativeCtx, Value, frost_bundle, frost_closure};
use parking_lot::ReentrantMutex;
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::hooks::{AuthContext, Authorization};
use rusqlite::{Batch, Connection};

use crate::Authorizer;
use crate::error::{
    closed_error, denied_error, in_use_error, is_function_denial, nul_error, sqlite_error,
};

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
#[derive(Debug, Hash, PartialOrd, Ord, EnumSetType)]
#[non_exhaustive]
pub enum Method {
    /// `db.script(sql)` runs the statements of `sql`, separated by
    /// semicolons, and returns how many rows they changed in all, counting
    /// rows changed by triggers and foreign key actions. It takes no bindings.
    /// Outside a transaction, each statement commits on its own, so a
    /// statement failing leaves those before it committed.
    Script,
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
    /// as pragmas, temporary tables, and attached databases. A factory serving
    /// scripts it trusts less than those users must reset that state, or not
    /// share the pool. A transaction a script leaves open is rolled back
    /// before the connection returns to its pool.
    pub fn pooled(connection: impl DerefMut<Target = Connection> + Send + 'static) -> Self {
        Self::holding(Source::Pooled(Box::new(connection)))
    }

    /// Exposes only `methods`, rather than every [`Method`], the default.
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
    /// Runs `operation` on the connection with `sql`, for the function `ctx`
    /// runs, once `sql` passes the extension's checks and the host's filter.
    fn with_sql<T>(
        &self,
        ctx: &NativeCtx<'_>,
        sql: &str,
        operation: impl FnOnce(&Connection, &str) -> rusqlite::Result<T>,
    ) -> Result<T, FrostError> {
        if sql.contains('\0') {
            return Err(nul_error(ctx.name()));
        }
        self.with_connection(ctx, |connection| {
            if let Some(filter) = &self.filter {
                filter(sql)?;
            }
            let denials = self.denials.load(Ordering::Relaxed);
            operation(connection, sql).map_err(|err| {
                // SQLite words a denied function under another code than
                // `SQLITE_AUTH`, naming it. Its wording alone could be any
                // failure's, so only a denial during this call confirms it.
                if self.denials.load(Ordering::Relaxed) != denials && is_function_denial(&err) {
                    denied_error(ctx.name())
                } else {
                    sqlite_error(ctx.name(), &err)
                }
            })
        })
    }

    /// Runs `operation` on the connection, for the function `ctx` runs.
    fn with_connection<T>(
        &self,
        ctx: &NativeCtx<'_>,
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
            .ok_or_else(|| closed_error(ctx.name()))?;
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
        self.with_sql(ctx, &sql, |connection, sql| {
            let before = connection.total_changes();
            run_to_completion(connection, sql)?;
            Ok(changes_since(connection, before))
        })
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

/// Runs each statement of `sql` in turn, stepping each through every row it
/// returns, so that each runs in full.
fn run_to_completion(connection: &Connection, sql: &str) -> rusqlite::Result<()> {
    let mut statements = Batch::new(connection, sql);
    while let Some(mut statement) = statements.next()? {
        let mut rows = statement.raw_query();
        while rows.next()?.is_some() {}
    }
    Ok(())
}

/// How many rows `connection` has changed since its count of changes was
/// `before`.
fn changes_since(connection: &Connection, before: u64) -> i64 {
    // The count can be SQLite's 32-bit one, sign-extended, which wraps every
    // 2^32 changes. No one call changes nearly that many rows, so the low 32
    // bits of the difference are exact.
    let changed = (connection.total_changes() as u32).wrapping_sub(before as u32);
    i64::from(changed)
}
