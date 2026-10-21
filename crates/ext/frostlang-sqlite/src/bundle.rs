//! The bundles through which scripts use a connection: the connection's own,
//! `db`, and each open transaction's, `tx`.

use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use enumset::EnumSet;
use frostlang::native::{Bundle, Function, Optional, Structured};
use frostlang::{FrostError, FrostResult, FrostString, NativeCtx, Value, frost_bundle};
use parking_lot::ReentrantMutex;
use rusqlite::Connection;
use rusqlite::hooks::{AuthContext, Authorization};

use crate::Method;
use crate::connection::{Source, SqlFilter};
use crate::error::{
    closed_error, denied_error, in_use_error, inner_transaction_error, is_function_denial,
    nul_error, open_transaction_error, rolled_back_error, sqlite_error, transaction_ended_error,
};
use crate::run::{self, Failure, Run};
use crate::transaction;

/// What every bundle over one connection shares: the connection, until
/// closed, and how scripts may use it.
pub(crate) struct Shared {
    // Reentrant, since a method running a statement may call back into Frost,
    // which may use the same connection on the same thread. The `RefCell` lets
    // `close` take the connection, and tell when a call further up the stack
    // is still using it.
    connection: ReentrantMutex<RefCell<Option<Held>>>,
    filter: Option<SqlFilter>,
    /// How many actions the connection's authorizer has denied.
    denials: Arc<AtomicU64>,
    methods: EnumSet<Method>,
}

/// The bundle a call comes through.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Owner {
    /// The connection's own, `db`.
    Connection,
    /// The `tx` of the transaction with this id.
    Transaction(u64),
}

impl Shared {
    /// The state of a connection bundle over `source`, whose authorizer, if
    /// the extension installed it (`authorized`), counts its denials in
    /// `denials`.
    pub(crate) fn new(
        source: Source,
        authorized: bool,
        filter: Option<SqlFilter>,
        denials: Arc<AtomicU64>,
        methods: EnumSet<Method>,
    ) -> Arc<Self> {
        Arc::new(Self {
            connection: ReentrantMutex::new(RefCell::new(Some(Held {
                source,
                authorized,
                transactions: Transactions::default(),
            }))),
            filter,
            denials,
            methods,
        })
    }

    /// Runs `operation` for the function `ctx` runs, through `owner`, once
    /// `sql` passes the extension's checks and the host's filter.
    fn with_sql<T>(
        &self,
        ctx: &mut NativeCtx<'_>,
        owner: Owner,
        sql: &str,
        operation: impl FnOnce(&mut Run<'_, '_>) -> Result<T, Failure>,
    ) -> Result<T, FrostError> {
        let function = ctx.name();
        if sql.contains('\0') {
            return Err(nul_error(function));
        }
        self.with_connection(function, owner, |connection| {
            if let Some(filter) = &self.filter {
                filter(sql)?;
            }
            let mut run = Run::new(
                connection,
                sql,
                ctx,
                &self.denials,
                self.transaction_hint(owner),
            );
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

    /// What a call through `owner` running a transaction statement should do
    /// instead, as its error suggests.
    fn transaction_hint(&self, owner: Owner) -> &'static str {
        match owner {
            Owner::Connection if self.methods.contains(Method::Transaction) => {
                "; use db.transaction"
            }
            Owner::Connection => "",
            Owner::Transaction(_) => {
                "; tx.transaction makes savepoints, and a transaction ends as its function returns"
            }
        }
    }

    /// Runs `operation` on the connection, for `function`, through `owner`.
    pub(crate) fn with_connection<T>(
        &self,
        function: &str,
        owner: Owner,
        operation: impl FnOnce(&Held) -> Result<T, FrostError>,
    ) -> Result<T, FrostError> {
        let lock = self.connection.lock();
        // Only `close` borrows mutably, and only to take the connection out,
        // so no mutable borrow is live here. Every method must keep it so: a
        // mutable borrow held across a call back into Frost would make a call
        // reentering the bundle panic.
        let held = lock.borrow();
        let held = held.as_ref().ok_or_else(|| closed_error(function))?;
        held.transactions.admit(function, owner)?;
        // Scripts cannot end a transaction, but SQLite itself may, on an `OR
        // ROLLBACK` conflict, say, ending every savepoint within it. The
        // transaction's `tx`s are then spent, and `transaction` cannot commit.
        if owner != Owner::Connection && held.is_autocommit() {
            return Err(rolled_back_error(function));
        }
        operation(held)
    }
}

/// A connection as its bundles hold it.
pub(crate) struct Held {
    source: Source,
    /// Whether the extension installed the connection's authorizer.
    authorized: bool,
    pub(crate) transactions: Transactions,
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
        // for its next user, who must inherit neither. A script's transaction
        // ends within its `transaction` call, so one still open was left by a
        // failed rollback, or by the host. A failure leaves nothing better to
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

/// The transactions open on a connection, innermost last. Only the innermost
/// one's `tx` may be used, and `db` only while none is open.
#[derive(Default)]
pub(crate) struct Transactions {
    open: RefCell<Vec<u64>>,
    next_id: Cell<u64>,
}

impl Transactions {
    /// Whether `function`, called through `owner`, may use the connection.
    fn admit(&self, function: &str, owner: Owner) -> Result<(), FrostError> {
        let open = self.open.borrow();
        match owner {
            Owner::Connection if open.is_empty() => Ok(()),
            Owner::Connection => Err(open_transaction_error(function)),
            Owner::Transaction(id) if open.last() == Some(&id) => Ok(()),
            Owner::Transaction(id) if open.contains(&id) => Err(inner_transaction_error(function)),
            Owner::Transaction(_) => Err(transaction_ended_error(function)),
        }
    }

    /// Makes a new transaction's `tx` usable, innermost, until the returned
    /// guard is dropped.
    pub(crate) fn open(&self) -> Usable<'_> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.open.borrow_mut().push(id);
        Usable {
            transactions: self,
            id,
        }
    }
}

/// A transaction whose `tx` is usable until this is dropped.
pub(crate) struct Usable<'t> {
    transactions: &'t Transactions,
    pub(crate) id: u64,
}

impl Drop for Usable<'_> {
    fn drop(&mut self) {
        let ended = self.transactions.open.borrow_mut().pop();
        debug_assert_eq!(ended, Some(self.id), "transactions end innermost first");
    }
}

/// The state of a connection's own bundle, `db`.
struct Db {
    shared: Arc<Shared>,
}

impl Db {
    fn owner(&self) -> Owner {
        Owner::Connection
    }
}

/// The state of an open transaction's bundle, `tx`.
struct Tx {
    shared: Arc<Shared>,
    id: u64,
}

impl Tx {
    fn owner(&self) -> Owner {
        Owner::Transaction(self.id)
    }
}

/// The connection's own bundle, `db`, with the methods `shared` allows.
pub(crate) fn connection_bundle(shared: Arc<Shared>) -> Value {
    let methods = shared.methods;
    methods
        .iter()
        .fold(Bundle::new(Db { shared }), |bundle, method| match method {
            Method::Script => bundle.with(Db::script_method),
            Method::Exec => bundle.with(Db::exec_method),
            Method::Query => bundle.with(Db::query_method),
            Method::Fold => bundle.with(Db::fold_method),
            Method::LastInsertRowid => bundle.with(Db::last_insert_rowid_method),
            Method::Transaction => bundle.with(Db::transaction_method),
            Method::Close => bundle.with(Db::close_method),
        })
        .build()
}

/// The bundle, `tx`, of the open transaction `id` on `shared`'s connection,
/// with the methods `shared` allows but `close`.
pub(crate) fn transaction_bundle(shared: &Arc<Shared>, id: u64) -> Value {
    let state = Tx {
        shared: Arc::clone(shared),
        id,
    };
    shared
        .methods
        .iter()
        .fold(Bundle::new(state), |bundle, method| match method {
            Method::Script => bundle.with(Tx::script_method),
            Method::Exec => bundle.with(Tx::exec_method),
            Method::Query => bundle.with(Tx::query_method),
            Method::Fold => bundle.with(Tx::fold_method),
            Method::LastInsertRowid => bundle.with(Tx::last_insert_rowid_method),
            Method::Transaction => bundle.with(Tx::transaction_method),
            Method::Close => bundle,
        })
        .build()
}

/// The methods `db` and `tx` share, for the bundle state `$state`, whose
/// functions are named with `$prefix`: each calls through the state's owner.
macro_rules! shared_methods {
    ($state:ident, $prefix:literal) => {
        #[frost_bundle(prefix = $prefix, group = script_method)]
        impl $state {
            fn script(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
                sql: FrostString,
            ) -> Result<i64, FrostError> {
                self.shared.with_sql(ctx, self.owner(), &sql, run::script)
            }
        }

        #[frost_bundle(prefix = $prefix, group = exec_method)]
        impl $state {
            fn exec(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
                sql: FrostString,
                bindings: Optional<Structured>,
            ) -> Result<i64, FrostError> {
                self.shared
                    .with_sql(ctx, self.owner(), &sql, |run| run::exec(run, bindings.0))
            }
        }

        #[frost_bundle(prefix = $prefix, group = query_method)]
        impl $state {
            fn query(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
                sql: FrostString,
                bindings: Optional<Structured>,
            ) -> FrostResult {
                self.shared
                    .with_sql(ctx, self.owner(), &sql, |run| run::query(run, bindings.0))
            }
        }

        #[frost_bundle(prefix = $prefix, group = fold_method)]
        impl $state {
            fn fold(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
                sql: FrostString,
                bindings: Optional<Structured>,
                initial: Value,
                #[frost(unnamed)] function: Function,
            ) -> FrostResult {
                self.shared.with_sql(ctx, self.owner(), &sql, |run| {
                    run::fold(run, bindings.0, initial, &function)
                })
            }
        }

        #[frost_bundle(prefix = $prefix, group = last_insert_rowid_method)]
        impl $state {
            fn last_insert_rowid(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
            ) -> Result<i64, FrostError> {
                self.shared
                    .with_connection(
                        ctx.name(),
                        self.owner(),
                        |held| Ok(held.last_insert_rowid()),
                    )
            }
        }

        #[frost_bundle(prefix = $prefix, group = transaction_method)]
        impl $state {
            fn transaction(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
                #[frost(unnamed)] function: Function,
            ) -> FrostResult {
                transaction::run(&self.shared, ctx, self.owner(), &function)
            }
        }
    };
}

shared_methods!(Db, "db");
shared_methods!(Tx, "tx");

#[frost_bundle(prefix = "db", group = close_method)]
impl Db {
    fn close(&self, #[frost(ctx)] ctx: &mut NativeCtx<'_>) -> Result<(), FrostError> {
        let held = {
            let lock = self.shared.connection.lock();
            let mut held = lock
                .try_borrow_mut()
                .map_err(|_| in_use_error(ctx.name()))?;
            held.take()
        };
        // Dropped only now the lock and borrow are released: releasing the
        // connection may run code that reaches this bundle, which then finds
        // it closed.
        match held {
            Some(held) => {
                drop(held);
                Ok(())
            }
            None => Err(closed_error(ctx.name())),
        }
    }
}
