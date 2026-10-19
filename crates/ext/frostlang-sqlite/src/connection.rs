//! Connections as scripts see them: connection bundles, and the factories that
//! make them.

use std::cell::RefCell;
use std::fmt;
use std::ops::{Deref, DerefMut};

use enumset::{EnumSet, EnumSetType};
use frostlang::native::{Bundle, Optional};
use frostlang::{FrostError, FrostString, NativeCtx, Value, frost_bundle, frost_closure};
use parking_lot::ReentrantMutex;
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::{Batch, Connection};

use crate::error::{closed_error, in_use_error, nul_error, sqlite_error};

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
    connection: Held,
    methods: EnumSet<Method>,
}

impl ScriptConnection {
    /// Exposes `connection`.
    pub fn new(connection: Connection) -> Self {
        Self::holding(Held::Owned(connection))
    }

    /// Exposes `connection`, taken from a pool.
    ///
    /// A pooled connection keeps the state its earlier users left on it, such
    /// as pragmas, temporary tables, and attached databases. A factory serving
    /// scripts it trusts less than those users must reset that state, or not
    /// share the pool. A transaction a script leaves open is rolled back
    /// before the connection returns to its pool.
    pub fn pooled(connection: impl DerefMut<Target = Connection> + Send + 'static) -> Self {
        Self::holding(Held::Pooled(Box::new(connection)))
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

    fn holding(connection: Held) -> Self {
        Self {
            connection,
            methods: EnumSet::all(),
        }
    }

    /// The connection bundle.
    pub(crate) fn into_bundle(self) -> Value {
        let state = Db {
            connection: ReentrantMutex::new(RefCell::new(Some(self.connection))),
        };
        self.methods
            .iter()
            .fold(Bundle::new(state), |bundle, method| {
                bundle.with(match method {
                    Method::Script => Db::script_method,
                    Method::Close => Db::close_method,
                })
            })
            .build()
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
        factory(argument.0).map(ScriptConnection::into_bundle)
    })
}

/// A connection as a [`ScriptConnection`] holds it.
enum Held {
    Owned(Connection),
    Pooled(Box<dyn DerefMut<Target = Connection> + Send>),
}

impl Deref for Held {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        match self {
            Self::Owned(connection) => connection,
            Self::Pooled(connection) => connection,
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // SQLite rolls back an owned connection's open transaction as it closes
        // it, but a pooled one stays open for its next user, who must not
        // inherit the transaction or the locks it holds. A failed rollback
        // leaves nothing better to do.
        if let Self::Pooled(connection) = self
            && !connection.is_autocommit()
        {
            let _ = connection.execute_batch("ROLLBACK");
        }
    }
}

/// A connection bundle's state: its connection, until closed.
struct Db {
    // Reentrant, since a method running a statement may call back into Frost,
    // which may use the same connection on the same thread. The `RefCell` lets
    // `close` take the connection, and tell when a call further up the stack
    // is still using it.
    connection: ReentrantMutex<RefCell<Option<Held>>>,
}

impl Db {
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
        if sql.contains('\0') {
            return Err(nul_error(ctx.name()));
        }
        self.with_connection(ctx, |connection| {
            let before = connection.total_changes();
            run_to_completion(connection, &sql).map_err(|err| sqlite_error(ctx.name(), &err))?;
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
