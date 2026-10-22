//! Cancelling a run while it uses a connection: between rows and statements
//! on every connection, and within a statement on a cancellable one.
//!
//! Most tests cancel from within the statement itself, through a host SQL
//! function, so that only what is under test can stop it. A cancellation that
//! goes unnoticed leaves a statement running forever, so each run goes through
//! [`within_deadline`].

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{
    AbortReason, CancelToken, HostComponent, Importer, ImporterBuilder, RunErrorKind, Value, Vm,
    frost_closure,
};
use frostlang_sqlite::rusqlite::Connection;
use frostlang_sqlite::rusqlite::functions::FunctionFlags;
use frostlang_sqlite::{ScriptConnection, connection_factory, sqlite_error};

use crate::common::{
    Pooled, Scratch, app_component, frost, host_importer, memory_connection, value,
};

/// How long a run that should be cancelled may take before the test fails.
/// Generous: it only bounds a hang, and a passing run takes milliseconds.
const DEADLINE: Duration = Duration::from_secs(30);

/// An endless sequence of rows, as a common table expression `c (x)`.
const ENDLESS: &str = "WITH RECURSIVE c (x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c)";

/// A statement that never returns from its first step, cancelling the run as
/// it begins: only a progress handler can stop it.
fn runaway() -> String {
    format!("{ENDLESS} SELECT count(*) FROM c WHERE cancel_now()")
}

/// A thousand rows, each calling `cancel_now()` as it is read.
const CANCELLING_ROWS: &str = "SELECT cancel_now() AS c FROM (WITH RECURSIVE n (x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 1000) SELECT x FROM n)";

/// How a run ended: its value, or how it failed.
#[derive(Debug, PartialEq)]
enum Ended {
    Finished(Value),
    Raised(String),
    Aborted(AbortReason),
}

/// Runs `source` with `importer`, cancellable by `token`, on a thread of its
/// own, failing the test if it does not end within [`DEADLINE`].
fn within_deadline(source: &str, importer: Arc<Importer>, token: &CancelToken) -> Ended {
    let closure = compile_program("main.frst", source, CompilerOptions::new())
        .unwrap_or_else(|errors| panic!("the script compiles:\n{}", errors.render()))
        .code
        .into_closure()
        .expect("the script captures nothing");
    let vm = Vm::factory()
        .with_importer(importer)
        .build(closure)
        .with_cancel_token(token.clone());
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let ended = match vm.run() {
            Ok(result) => Ended::Finished(result.tail().clone()),
            Err(failure) => match failure.kind() {
                RunErrorKind::Aborted(reason) => Ended::Aborted(reason),
                RunErrorKind::Raised => Ended::Raised(failure.into_error().message().into_owned()),
            },
        };
        sender.send(ended).expect("the test is waiting");
    });
    match receiver.recv_timeout(DEADLINE) {
        Ok(ended) => ended,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("the run did not end within {DEADLINE:?}: the cancellation went unnoticed")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the run panicked"),
    }
}

/// An importer of the app component holding `entries`.
fn app(entries: Vec<(&str, Value)>) -> Arc<Importer> {
    host_importer(app_component(entries))
}

/// How a connection a factory makes is set up.
#[derive(Clone)]
struct Setup {
    /// Whether to opt the connection in to cancellation.
    cancellable: bool,
    /// The token the host SQL function `cancel_now()` cancels, counting its
    /// calls in `calls`.
    cancels: CancelToken,
    calls: Arc<AtomicUsize>,
    /// The database file, else an in-memory database.
    path: Option<PathBuf>,
}

impl Setup {
    /// A cancellable in-memory connection, `cancel_now()` cancelling `token`.
    fn cancellable(token: &CancelToken) -> Self {
        Self {
            cancellable: true,
            cancels: token.clone(),
            calls: Arc::default(),
            path: None,
        }
    }

    /// The same, but not opted in to cancellation.
    fn not_cancellable(token: &CancelToken) -> Self {
        Self {
            cancellable: false,
            ..Self::cancellable(token)
        }
    }

    /// The same, on the database file at `path`.
    fn at(self, path: &Path) -> Self {
        Self {
            path: Some(path.to_owned()),
            ..self
        }
    }

    /// The connection, with `cancel_now()`, unexposed yet.
    fn connection(&self) -> Result<Connection, frostlang::FrostError> {
        let connection = match &self.path {
            Some(path) => {
                Connection::open(path).map_err(|err| sqlite_error("app.open_db", &err))?
            }
            None => memory_connection()?,
        };
        let (token, calls) = (self.cancels.clone(), Arc::clone(&self.calls));
        connection
            .create_scalar_function("cancel_now", 0, FunctionFlags::SQLITE_UTF8, move |_| {
                calls.fetch_add(1, Ordering::Relaxed);
                token.cancel();
                Ok(1)
            })
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        Ok(connection)
    }

    /// `connection` exposed as this setup says.
    fn expose(&self, exposed: ScriptConnection) -> ScriptConnection {
        if self.cancellable {
            exposed.with_cancellation()
        } else {
            exposed
        }
    }

    /// A factory of such connections.
    fn factory(&self) -> Value {
        let setup = self.clone();
        connection_factory("app.open_db", move |_| {
            Ok(setup.expose(ScriptConnection::new(setup.connection()?)))
        })
    }

    /// How many times `cancel_now()` has been called.
    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

/// Runs `body` with `db` from a factory as `setup` says, cancellable by the
/// token `cancel_now()` cancels.
fn run_with(setup: &Setup, body: &str) -> Ended {
    let source = format!(
        r"
        def db = import('app').open_db()
        {body}
        "
    );
    within_deadline(
        &source,
        app(vec![("open_db", setup.factory())]),
        &setup.cancels,
    )
}

/// Creates the table `t (x)` in a new database file under `scratch`, and
/// returns its path.
fn database_with_table(scratch: &Scratch) -> PathBuf {
    let path = scratch.path("app.db");
    Connection::open(&path)
        .and_then(|connection| connection.execute_batch("CREATE TABLE t (x)"))
        .expect("the database is set up");
    path
}

/// The values of `t` in the database file at `path`.
fn rows_in(path: &Path) -> Vec<i64> {
    let connection = Connection::open(path).expect("the database opens");
    let mut statement = connection
        .prepare("SELECT x FROM t ORDER BY x")
        .expect("the query prepares");
    statement
        .query_map([], |row| row.get(0))
        .expect("the query runs")
        .collect::<Result<_, _>>()
        .expect("each row reads")
}

// --- Within a statement, on a cancellable connection ---

#[test]
fn every_method_running_sql_stops_within_a_step() {
    let runaway = runaway();
    for call in [
        format!("db.exec('{runaway}')"),
        format!("db.query('{runaway}')"),
        format!("db.fold('{runaway}', null, fn acc, row -> acc)"),
        format!("db.script('SELECT 1; {runaway}')"),
        format!("db.transaction(fn tx -> tx.exec('{runaway}'))"),
        format!("db.transaction(fn tx -> tx.transaction(fn inner -> inner.query('{runaway}')))"),
    ] {
        // Ending at all shows it: the statement goes on calling `cancel_now()`
        // until the progress handler next checks.
        let setup = Setup::cancellable(&CancelToken::new());
        assert_eq!(
            run_with(&setup, &call),
            Ended::Aborted(AbortReason::Cancelled),
            "{call}"
        );
    }
}

#[test]
fn a_statement_stops_when_cancelled_from_another_thread() {
    // The statement tells the host it has begun, from within it, and the host
    // cancels from another thread, as a host would.
    let token = CancelToken::new();
    let (sender, heard) = mpsc::channel();
    let factory = connection_factory("app.open_db", move |_| {
        let connection = memory_connection()?;
        let sender = Mutex::new(sender.clone());
        connection
            .create_scalar_function("started", 0, FunctionFlags::SQLITE_UTF8, move |_| {
                let _ = sender.lock().expect("no test panicked").send(());
                Ok(1)
            })
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        Ok(ScriptConnection::new(connection).with_cancellation())
    });
    let canceller = {
        let token = token.clone();
        thread::spawn(move || {
            heard.recv().expect("the statement begins");
            token.cancel();
        })
    };
    let source = format!(
        "import('app').open_db().query('{ENDLESS} SELECT count(*) FROM c WHERE started()')"
    );
    assert_eq!(
        within_deadline(&source, app(vec![("open_db", factory)]), &token),
        Ended::Aborted(AbortReason::Cancelled)
    );
    canceller.join().expect("the canceller ends");
}

#[test]
fn the_extensions_own_connections_are_cancellable() {
    // A script cannot give SQL a function, so the host cancels from another
    // thread once the script is about to begin the statement. A cancellation
    // landing in the few instructions before it begins would stop the run
    // before the statement, and the test would pass without the handler; the
    // pause makes that all but impossible.
    let scratch = Scratch::new();
    for open in [
        "sqlite.open_memory()".to_owned(),
        format!("sqlite.open({})", scratch.literal("cancelled.db")),
    ] {
        let token = CancelToken::new();
        let (sender, heard) = mpsc::channel();
        let sender = Mutex::new(sender);
        let started = frost_closure!("started", move || {
            let _ = sender.lock().expect("no test panicked").send(());
        });
        let importer = ImporterBuilder::new()
            .with_extension(frostlang_sqlite::extension())
            .expect("a lone extension is accepted")
            .with_component(app_component(vec![("started", started)]))
            .expect("a lone component is accepted")
            .build();
        let canceller = {
            let token = token.clone();
            thread::spawn(move || {
                heard.recv().expect("the script reaches the statement");
                thread::sleep(Duration::from_millis(10));
                token.cancel();
            })
        };
        let source = format!(
            r"
            def sqlite = import('ext.sqlite')
            def db = {open}
            import('app').started()
            db.query('{ENDLESS} SELECT count(*) FROM c')
            "
        );
        assert_eq!(
            within_deadline(&source, importer, &token),
            Ended::Aborted(AbortReason::Cancelled),
            "{open}"
        );
        canceller.join().expect("the canceller ends");
    }
}

#[test]
fn a_nested_call_leaves_the_outer_statement_cancellable() {
    // The fold's first row comes at once, and its callback makes a call of its
    // own on the connection. The second row runs away, cancelling as it
    // begins: only the outer statement's watch, restored after the nested
    // call, can stop it.
    let setup = Setup::cancellable(&CancelToken::new());
    let body = format!(
        r"
        db.fold('SELECT CASE WHEN n = 2 THEN ({ENDLESS} SELECT count(*) FROM c WHERE cancel_now()) ELSE n END AS v FROM (SELECT 1 AS n UNION ALL SELECT 2)', null, fn acc, row -> {{
            db.exec('SELECT 1')
        }})
        "
    );
    assert_eq!(
        run_with(&setup, &body),
        Ended::Aborted(AbortReason::Cancelled)
    );
}

#[test]
fn an_interrupted_write_rolls_its_transaction_back() {
    // A write interrupted within a savepoint makes SQLite roll back the whole
    // transaction. Pooled, so that the connection stays open after the run,
    // as closing would roll back any transaction of itself.
    let scratch = Scratch::new();
    let path = database_with_table(&scratch);
    let setup = Setup::cancellable(&CancelToken::new()).at(&path);
    let released = Arc::new(Mutex::new(None));
    let factory = {
        let (setup, released) = (setup.clone(), Arc::clone(&released));
        connection_factory("app.open_db", move |_| {
            let released = Arc::clone(&released);
            let pooled = Pooled::new(setup.connection()?, move |connection| {
                *released.lock().expect("no test panicked") = Some(connection.is_autocommit());
            });
            Ok(setup.expose(ScriptConnection::pooled(pooled)))
        })
    };
    let source = format!(
        r"
        import('app').open_db().transaction(fn tx -> {{
            tx.exec('INSERT INTO t VALUES (1)')
            tx.transaction(fn inner -> {{
                inner.exec('INSERT INTO t VALUES (2)')
                inner.exec('INSERT INTO t {ENDLESS} SELECT x FROM c WHERE cancel_now()')
            }})
        }})
        "
    );
    assert_eq!(
        within_deadline(&source, app(vec![("open_db", factory)]), &setup.cancels),
        Ended::Aborted(AbortReason::Cancelled)
    );
    assert_eq!(
        *released.lock().expect("no test panicked"),
        Some(true),
        "released with no transaction open"
    );
    assert_eq!(
        rows_in(&path),
        Vec::<i64>::new(),
        "every insert rolled back"
    );
}

#[test]
fn a_cancelled_script_keeps_the_statements_that_ran() {
    // Outside a transaction each statement commits on its own, as SQL has
    // it; only a transaction makes a script atomic.
    let scratch = Scratch::new();
    let path = database_with_table(&scratch);
    let setup = Setup::cancellable(&CancelToken::new()).at(&path);
    let body = format!("db.script('INSERT INTO t VALUES (1); {}')", runaway());
    assert_eq!(
        run_with(&setup, &body),
        Ended::Aborted(AbortReason::Cancelled)
    );
    assert_eq!(rows_in(&path), [1]);
}

#[test]
fn a_shared_cancellable_connection_serves_an_uncancelled_run_after_a_cancelled_one() {
    // Two runs on two threads share one bundle. The first is cancelled within
    // its statement; the second, waiting for the connection meanwhile or
    // calling after, runs as if nothing happened.
    let first = CancelToken::new();
    let (sender, heard) = mpsc::channel();
    let factory = connection_factory("app.open_db", move |_| {
        let connection = memory_connection()?;
        let sender = Mutex::new(sender.clone());
        connection
            .create_scalar_function("started", 0, FunctionFlags::SQLITE_UTF8, move |_| {
                let _ = sender.lock().expect("no test panicked").send(());
                Ok(1)
            })
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        Ok(ScriptConnection::new(connection).with_cancellation())
    });
    let db = within_deadline(
        "import('app').open_db()",
        app(vec![("open_db", factory)]),
        &CancelToken::new(),
    );
    let Ended::Finished(db) = db else {
        panic!("the bundle is made, not {db:?}");
    };
    let shared = || {
        host_importer(
            HostComponent::builder("shared")
                .value("db", db.clone())
                .build(),
        )
    };

    let first_run = {
        let (shared, first) = (shared(), first.clone());
        thread::spawn(move || {
            within_deadline(
                &format!(
                    "import('shared').db.query('{ENDLESS} SELECT count(*) FROM c WHERE started()')"
                ),
                shared,
                &first,
            )
        })
    };
    heard.recv().expect("the first run's statement begins");
    let second_run = {
        let shared = shared();
        thread::spawn(move || {
            within_deadline(
                "import('shared').db.query('SELECT 1 AS x')",
                shared,
                &CancelToken::new(),
            )
        })
    };
    first.cancel();
    assert_eq!(
        first_run.join().expect("the first run ends"),
        Ended::Aborted(AbortReason::Cancelled)
    );
    assert_eq!(
        second_run.join().expect("the second run ends"),
        Ended::Finished(frost("[{x: 1}]"))
    );
}

// --- The connection's own progress handler ---

/// A connection with a progress handler of the host's own, counting its calls
/// in `calls`, and answering `interrupts`.
fn with_own_handler(
    calls: &Arc<AtomicUsize>,
    interrupts: bool,
) -> Result<Connection, frostlang::FrostError> {
    let connection = memory_connection()?;
    let calls = Arc::clone(calls);
    connection
        .progress_handler(
            1,
            Some(move || {
                calls.fetch_add(1, Ordering::Relaxed);
                interrupts
            }),
        )
        .map_err(|err| sqlite_error("app.open_db", &err))?;
    Ok(connection)
}

/// Counts to a thousand on a connection from `app.open_db`.
const COUNTING: &str = r"
    import('app').open_db().query('WITH RECURSIVE c (x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000) SELECT count(*) AS n FROM c')
";

#[test]
fn a_connection_not_opted_in_keeps_its_own_progress_handler() {
    let calls = Arc::new(AtomicUsize::new(0));
    let factory = {
        let calls = Arc::clone(&calls);
        connection_factory("app.open_db", move |_| {
            Ok(ScriptConnection::new(with_own_handler(&calls, false)?))
        })
    };
    let ended = within_deadline(
        COUNTING,
        app(vec![("open_db", factory)]),
        &CancelToken::new(),
    );
    assert_eq!(ended, Ended::Finished(frost("[{n: 1000}]")));
    assert!(
        calls.load(Ordering::Relaxed) > 0,
        "the host's own handler ran"
    );
}

#[test]
fn a_pooled_connection_not_opted_in_keeps_its_own_progress_handler_after_release() {
    let calls = Arc::new(AtomicUsize::new(0));
    let after_release = Arc::new(Mutex::new(None));
    let factory = {
        let (calls, after_release) = (Arc::clone(&calls), Arc::clone(&after_release));
        connection_factory("app.open_db", move |_| {
            let (calls, after_release) = (Arc::clone(&calls), Arc::clone(&after_release));
            let connection = with_own_handler(&calls, false)?;
            let pooled = Pooled::new(connection, move |connection| {
                let before = calls.load(Ordering::Relaxed);
                let _: i64 = connection
                    .query_row(
                        "WITH RECURSIVE c (x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 100) SELECT count(*) FROM c",
                        [],
                        |row| row.get(0),
                    )
                    .expect("the pool's own query runs");
                *after_release.lock().expect("no test panicked") =
                    Some(calls.load(Ordering::Relaxed) > before);
            });
            Ok(ScriptConnection::pooled(pooled))
        })
    };
    let ended = within_deadline(
        COUNTING,
        app(vec![("open_db", factory)]),
        &CancelToken::new(),
    );
    assert_eq!(ended, Ended::Finished(frost("[{n: 1000}]")));
    assert_eq!(
        *after_release.lock().expect("no test panicked"),
        Some(true),
        "the host's handler ran on the released connection"
    );
}

#[test]
fn opting_in_replaces_the_connections_own_progress_handler() {
    let calls = Arc::new(AtomicUsize::new(0));
    let factory = {
        let calls = Arc::clone(&calls);
        connection_factory("app.open_db", move |_| {
            Ok(ScriptConnection::new(with_own_handler(&calls, false)?).with_cancellation())
        })
    };
    let ended = within_deadline(
        COUNTING,
        app(vec![("open_db", factory)]),
        &CancelToken::new(),
    );
    assert_eq!(ended, Ended::Finished(frost("[{n: 1000}]")));
    assert_eq!(
        calls.load(Ordering::Relaxed),
        0,
        "the host's own handler is gone"
    );
}

#[test]
fn a_hosts_own_interrupt_is_an_ordinary_error() {
    // Not the run's cancellation, so the run goes on, and may catch it.
    let calls = Arc::new(AtomicUsize::new(0));
    let factory = {
        let calls = Arc::clone(&calls);
        connection_factory("app.open_db", move |_| {
            Ok(ScriptConnection::new(with_own_handler(&calls, true)?))
        })
    };
    let source = r"
        def db = import('app').open_db()
        try_call(db.query, ['WITH RECURSIVE c (x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000) SELECT count(*) FROM c']).error
    ";
    assert_eq!(
        within_deadline(source, app(vec![("open_db", factory)]), &CancelToken::new()),
        Ended::Finished(value(
            "'Function db.query failed: interrupted (SQLITE_INTERRUPT)'"
        ))
    );
}

// --- Between rows and statements, on any connection ---

#[test]
fn every_method_running_sql_stops_at_the_next_row_once_cancelled() {
    for call in [
        format!("db.exec('{CANCELLING_ROWS}')"),
        format!("db.query('{CANCELLING_ROWS}')"),
        // The callback's call back stops it as well as the fold's own check.
        format!("db.fold('{CANCELLING_ROWS}', null, fn acc, row -> acc)"),
        format!("db.script('{CANCELLING_ROWS}')"),
    ] {
        let setup = Setup::not_cancellable(&CancelToken::new());
        assert_eq!(
            run_with(&setup, &call),
            Ended::Aborted(AbortReason::Cancelled),
            "{call}"
        );
        assert_eq!(setup.calls(), 1, "{call}: no row after the first was read");
    }
}

#[test]
fn script_stops_before_its_next_statement_once_cancelled() {
    // The first statement returns no row, so only the check before the second
    // statement can stop it.
    let setup = Setup::not_cancellable(&CancelToken::new());
    assert_eq!(
        run_with(
            &setup,
            "db.script('SELECT 1 WHERE cancel_now() = 0; SELECT cancel_now()')"
        ),
        Ended::Aborted(AbortReason::Cancelled)
    );
    assert_eq!(setup.calls(), 1);
}

#[test]
fn a_call_returning_no_row_once_cancelled_ends_the_run_cancelled() {
    let setup = Setup::not_cancellable(&CancelToken::new());
    assert_eq!(
        run_with(&setup, "db.query('SELECT 1 WHERE cancel_now() = 0')"),
        Ended::Aborted(AbortReason::Cancelled)
    );
}

#[test]
fn a_run_not_cancelled_reads_every_row() {
    // `cancel_now()` cancels a token the run does not hold.
    let setup = Setup::not_cancellable(&CancelToken::new());
    let source = format!(
        r"
        def db = import('app').open_db()
        db.query('{CANCELLING_ROWS}') @ len()
        "
    );
    assert_eq!(
        within_deadline(
            &source,
            app(vec![("open_db", setup.factory())]),
            &CancelToken::new()
        ),
        Ended::Finished(Value::Int(1000))
    );
    assert_eq!(setup.calls(), 1000);
}
