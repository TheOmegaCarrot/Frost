//! The bundles through which scripts use a connection: the connection's own,
//! `db`, and each open transaction's, `tx`.

use std::cell::{Cell, RefCell};
use std::mem;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use enumset::EnumSet;
use frostlang::native::{Bundle, Function, Optional, Structured};
use frostlang::{
    CancelToken, FrostError, FrostResult, FrostString, NativeCtx, Value, frost_bundle,
};
use parking_lot::{Mutex, ReentrantMutex};
use rusqlite::Connection;
use rusqlite::hooks::{AuthContext, Authorization};

use crate::connection::{Source, SqlFilter};
use crate::error::{
    closed_error, denied_error, in_use_error, inner_transaction_error, is_function_denial,
    nul_error, open_transaction_error, rolled_back_error, sqlite_error, transaction_ended_error,
};
use crate::run::{self, SqlCall, SqlCallError};
use crate::transaction;
use crate::{Authorizer, Method};

/// What every bundle over one connection shares: the connection, until
/// closed, and how scripts may use it.
pub(crate) struct ConnectionState {
    // Reentrant, since a method running a statement may call back into Frost,
    // which may use the same connection on the same thread. The `RefCell` lets
    // `close` take the connection, and tell when a call further up the stack
    // is still using it.
    connection: ReentrantMutex<RefCell<Option<OpenConnection>>>,
    filter: Option<SqlFilter>,
    /// How many actions the connection's authorizer has denied.
    denials: Arc<AtomicU64>,
    methods: EnumSet<Method>,
}

/// What a call may use the connection for, as the bundle it comes through
/// gives it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Autocommit mode, through `db`.
    Autocommit,
    /// The transaction with this id, through its `tx`.
    Transaction(TransactionId),
}

/// The id of a transaction opened on a connection, unique to the connection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TransactionId(u64);

impl ConnectionState {
    /// The state of a connection bundle over `connection`, whose authorizer,
    /// if the extension installed it, counts its denials in `denials`.
    pub(crate) fn new(
        connection: OpenConnection,
        filter: Option<SqlFilter>,
        denials: Arc<AtomicU64>,
        methods: EnumSet<Method>,
    ) -> Arc<Self> {
        Arc::new(Self {
            connection: ReentrantMutex::new(RefCell::new(Some(connection))),
            filter,
            denials,
            methods,
        })
    }

    /// Runs `operation` for the function `ctx` runs, in `scope`, once `sql`
    /// passes the extension's checks and the host's filter.
    fn with_sql<T>(
        &self,
        ctx: &mut NativeCtx<'_>,
        scope: Scope,
        sql: &str,
        operation: impl FnOnce(&mut SqlCall<'_, '_>) -> Result<T, SqlCallError>,
    ) -> Result<T, FrostError> {
        let function = ctx.name();
        if sql.contains('\0') {
            return Err(nul_error(function));
        }
        self.with_connection(ctx, scope, |connection, ctx| {
            if let Some(filter) = &self.filter {
                filter(sql)?;
            }
            let mut call = SqlCall::new(
                connection,
                sql,
                ctx,
                &self.denials,
                self.transaction_hint(scope),
            );
            operation(&mut call).map_err(|err| match err {
                SqlCallError::Frost(err) => err,
                // SQLite words a denied function under another code than
                // `SQLITE_AUTH`, naming it. Its wording alone could be any
                // failure's, so only a denial while SQLite ran confirms it.
                SqlCallError::Sqlite(err) if call.denied() && is_function_denial(&err) => {
                    denied_error(function)
                }
                SqlCallError::Sqlite(err) => sqlite_error(function, &err),
            })
        })
    }

    /// What a call in `scope` running a transaction statement should do
    /// instead, as its error suggests.
    fn transaction_hint(&self, scope: Scope) -> &'static str {
        match scope {
            Scope::Autocommit if self.methods.contains(Method::Transaction) => {
                "; use db.transaction"
            }
            Scope::Autocommit => "",
            Scope::Transaction(_) => {
                "; tx.transaction makes savepoints, and a transaction ends as its function returns"
            }
        }
    }

    /// Runs `operation` on the connection, for the function `ctx` runs, in
    /// `scope`.
    pub(crate) fn with_connection<'c, T>(
        &self,
        ctx: &mut NativeCtx<'c>,
        scope: Scope,
        operation: impl FnOnce(&OpenConnection, &mut NativeCtx<'c>) -> Result<T, FrostError>,
    ) -> Result<T, FrostError> {
        let function = ctx.name();
        let lock = self.connection.lock();
        // Only `close` borrows mutably, and only to take the connection out,
        // so no mutable borrow is live here. Every method must keep it so: a
        // mutable borrow held across a call back into Frost would make a call
        // reentering the bundle panic.
        let connection = lock.borrow();
        let connection = connection.as_ref().ok_or_else(|| closed_error(function))?;
        connection.transactions.check_scope(function, scope)?;
        // Scripts cannot end a transaction, but SQLite itself may, on an `OR
        // ROLLBACK` conflict, say, ending every savepoint within it. The
        // transaction's `tx`s are then spent, and `transaction` cannot commit.
        if scope != Scope::Autocommit && connection.is_autocommit() {
            return Err(rolled_back_error(function));
        }
        let _caller_token = connection
            .caller_token
            .as_ref()
            .map(|caller_token| caller_token.set(ctx.cancel_token().cloned()));
        operation(connection, ctx)
    }
}

/// About how many SQLite instructions run between checks of the calling run's
/// token on a cancellable connection: often enough to stop within
/// milliseconds, rarely enough to cost little.
const PROGRESS_INTERVAL: i32 = 1000;

/// A connection open to scripts, with the handlers the extension installed on
/// it and its open transactions.
pub(crate) struct OpenConnection {
    source: Source,
    /// Whether the extension installed the connection's authorizer.
    authorized: bool,
    /// What the connection's progress handler checks, if the extension
    /// installed it.
    caller_token: Option<CallerToken>,
    pub(crate) transactions: TransactionStack,
}

impl OpenConnection {
    /// `source`, with none of the extension's handlers installed yet.
    pub(crate) fn new(source: Source) -> Self {
        Self {
            source,
            authorized: false,
            caller_token: None,
            transactions: TransactionStack::default(),
        }
    }

    /// Installs `authorizer`, counting its denials in `denials`.
    pub(crate) fn authorize(
        &mut self,
        authorizer: Authorizer,
        denials: &Arc<AtomicU64>,
    ) -> rusqlite::Result<()> {
        let counted = Arc::clone(denials);
        self.source.authorizer(Some(move |ctx: AuthContext<'_>| {
            let judgement = authorizer.judge(ctx);
            if judgement == Authorization::Deny {
                counted.fetch_add(1, Ordering::Relaxed);
            }
            judgement
        }))?;
        self.authorized = true;
        Ok(())
    }

    /// Installs a progress handler interrupting a statement once the calling
    /// run is cancelled.
    pub(crate) fn interrupt_on_cancel(&mut self) -> rusqlite::Result<()> {
        let caller_token = CallerToken::default();
        let checked = caller_token.clone();
        self.source
            .progress_handler(PROGRESS_INTERVAL, Some(move || checked.is_cancelled()))?;
        self.caller_token = Some(caller_token);
        Ok(())
    }

    /// Runs `operation` with no run's cancellation interrupting it, as a
    /// rollback must not be.
    pub(crate) fn uninterrupted<T>(&self, operation: impl FnOnce(&Connection) -> T) -> T {
        let _no_caller_token = self
            .caller_token
            .as_ref()
            .map(|caller_token| caller_token.set(None));
        operation(self)
    }
}

/// The token of the run calling on a cancellable connection, which the
/// connection's progress handler checks.
#[derive(Clone, Default)]
struct CallerToken(Arc<Mutex<Option<CancelToken>>>);

impl CallerToken {
    /// Whether the calling run is cancelled.
    fn is_cancelled(&self) -> bool {
        self.0
            .lock()
            .as_ref()
            .is_some_and(CancelToken::is_cancelled)
    }

    /// Sets `token` until the returned guard is dropped, then the token set
    /// before, so that a call within another leaves the outer call's token
    /// set as it returns.
    fn set(&self, token: Option<CancelToken>) -> CallerTokenGuard<'_> {
        let previous = mem::replace(&mut *self.0.lock(), token);
        CallerTokenGuard {
            caller_token: self,
            previous,
        }
    }
}

/// Restores the token a [`CallerToken`] held before, as this is dropped.
struct CallerTokenGuard<'t> {
    caller_token: &'t CallerToken,
    previous: Option<CancelToken>,
}

impl Drop for CallerTokenGuard<'_> {
    fn drop(&mut self) {
        *self.caller_token.0.lock() = self.previous.take();
    }
}

impl Deref for OpenConnection {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        &self.source
    }
}

impl Drop for OpenConnection {
    fn drop(&mut self) {
        // An owned connection closes, and with it go its handlers and any
        // open transaction, which SQLite rolls back. A pooled one stays open
        // for its next user, who must inherit none. A script's transaction
        // ends within its `transaction` call, so one still open was left by a
        // failed rollback, or by the host. A failure leaves nothing better to
        // do.
        let Source::Pooled(connection) = &self.source else {
            return;
        };
        if self.authorized {
            let _ = connection.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        }
        if self.caller_token.is_some() {
            let _ = connection.progress_handler(0, None::<fn() -> bool>);
        }
        if !connection.is_autocommit() {
            let _ = connection.execute_batch("ROLLBACK");
        }
    }
}

/// The transactions open on a connection, innermost last. Only the innermost
/// one's `tx` may be used, and `db` only while none is open.
#[derive(Default)]
pub(crate) struct TransactionStack {
    open: RefCell<Vec<TransactionId>>,
    next_id: Cell<TransactionId>,
}

impl TransactionStack {
    /// Whether `function`, called in `scope`, may use the connection.
    fn check_scope(&self, function: &str, scope: Scope) -> Result<(), FrostError> {
        let open = self.open.borrow();
        match scope {
            Scope::Autocommit if open.is_empty() => Ok(()),
            Scope::Autocommit => Err(open_transaction_error(function)),
            Scope::Transaction(id) if open.last() == Some(&id) => Ok(()),
            Scope::Transaction(id) if open.contains(&id) => Err(inner_transaction_error(function)),
            Scope::Transaction(_) => Err(transaction_ended_error(function)),
        }
    }

    /// Makes a new transaction's `tx` usable, innermost, until the returned
    /// guard is dropped.
    pub(crate) fn push(&self) -> InnermostGuard<'_> {
        let id = self.next_id.get();
        self.next_id.set(TransactionId(id.0 + 1));
        self.open.borrow_mut().push(id);
        InnermostGuard {
            transactions: self,
            id,
        }
    }
}

/// Keeps a transaction innermost, so that its `tx` is usable, until this is
/// dropped.
pub(crate) struct InnermostGuard<'t> {
    transactions: &'t TransactionStack,
    pub(crate) id: TransactionId,
}

impl Drop for InnermostGuard<'_> {
    fn drop(&mut self) {
        let ended = self.transactions.open.borrow_mut().pop();
        debug_assert_eq!(ended, Some(self.id), "transactions end innermost first");
    }
}

/// The state of a connection's own bundle, `db`.
struct Db {
    state: Arc<ConnectionState>,
}

impl Db {
    fn scope(&self) -> Scope {
        Scope::Autocommit
    }
}

/// The state of an open transaction's bundle, `tx`.
struct Tx {
    state: Arc<ConnectionState>,
    id: TransactionId,
}

impl Tx {
    fn scope(&self) -> Scope {
        Scope::Transaction(self.id)
    }
}

/// The connection's own bundle, `db`, with the methods `state` allows.
pub(crate) fn connection_bundle(state: Arc<ConnectionState>) -> Value {
    let methods = state.methods;
    methods
        .iter()
        .fold(Bundle::new(Db { state }), |bundle, method| match method {
            Method::Script => bundle.with(Db::script_method),
            Method::Exec => bundle.with(Db::exec_method),
            Method::Query => bundle.with(Db::query_method),
            Method::Reduce => bundle.with(Db::reduce_method),
            Method::LastInsertRowid => bundle.with(Db::last_insert_rowid_method),
            Method::Transaction => bundle.with(Db::transaction_method),
            Method::Close => bundle.with(Db::close_method),
        })
        .build()
}

/// The bundle, `tx`, of the open transaction `id` on `state`'s connection,
/// with the methods `state` allows but `close`.
pub(crate) fn transaction_bundle(state: &Arc<ConnectionState>, id: TransactionId) -> Value {
    let tx = Tx {
        state: Arc::clone(state),
        id,
    };
    state
        .methods
        .iter()
        .fold(Bundle::new(tx), |bundle, method| match method {
            Method::Script => bundle.with(Tx::script_method),
            Method::Exec => bundle.with(Tx::exec_method),
            Method::Query => bundle.with(Tx::query_method),
            Method::Reduce => bundle.with(Tx::reduce_method),
            Method::LastInsertRowid => bundle.with(Tx::last_insert_rowid_method),
            Method::Transaction => bundle.with(Tx::transaction_method),
            Method::Close => bundle,
        })
        .build()
}

/// The methods `db` and `tx` share, for the bundle state `$state`, whose
/// functions are named with `$prefix`: each calls in the state's scope.
macro_rules! shared_methods {
    ($state:ident, $prefix:literal) => {
        #[frost_bundle(prefix = $prefix, group = script_method)]
        impl $state {
            fn script(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
                sql: FrostString,
            ) -> Result<i64, FrostError> {
                self.state.with_sql(ctx, self.scope(), &sql, run::script)
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
                self.state
                    .with_sql(ctx, self.scope(), &sql, |call| run::exec(call, bindings.0))
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
                self.state
                    .with_sql(ctx, self.scope(), &sql, |call| run::query(call, bindings.0))
            }
        }

        #[frost_bundle(prefix = $prefix, group = reduce_method)]
        impl $state {
            fn reduce(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
                sql: FrostString,
                bindings: Optional<Structured>,
                initial: Value,
                #[frost(unnamed)] function: Function,
            ) -> FrostResult {
                self.state.with_sql(ctx, self.scope(), &sql, |call| {
                    run::reduce(call, bindings.0, initial, &function)
                })
            }
        }

        #[frost_bundle(prefix = $prefix, group = last_insert_rowid_method)]
        impl $state {
            fn last_insert_rowid(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
            ) -> Result<i64, FrostError> {
                self.state
                    .with_connection(ctx, self.scope(), |connection, _| {
                        Ok(connection.last_insert_rowid())
                    })
            }
        }

        #[frost_bundle(prefix = $prefix, group = transaction_method)]
        impl $state {
            fn transaction(
                &self,
                #[frost(ctx)] ctx: &mut NativeCtx<'_>,
                #[frost(unnamed)] function: Function,
            ) -> FrostResult {
                transaction::run(&self.state, ctx, self.scope(), &function)
            }
        }
    };
}

shared_methods!(Db, "db");
shared_methods!(Tx, "tx");

#[frost_bundle(prefix = "db", group = close_method)]
impl Db {
    fn close(&self, #[frost(ctx)] ctx: &mut NativeCtx<'_>) -> Result<(), FrostError> {
        let connection = {
            let lock = self.state.connection.lock();
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
