//! `db.transaction` and `tx.transaction`: running a callback in a transaction,
//! or within one in a savepoint.

use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{
    AbortReason, CancelToken, FrostOpaque, HostComponent, RunErrorKind, SpecialFloat, Value, Vm,
    frost_closure,
};
use frostlang_sqlite::rusqlite::Connection;
use frostlang_sqlite::rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use frostlang_sqlite::{EnumSet, Method, ScriptConnection, connection_factory, sqlite_error};

use crate::common::{
    NON_STRINGS, Pooled, Scratch, arity_error, assert_arity, assert_raises, assert_values, frost,
    host_importer, memory_connection, run_app, string_literal, value, with_db, with_rows,
    with_table,
};

/// How a call fails, as `try_call` would report it, without its trace.
fn failed(message: &str) -> String {
    format!("{{ok: false, error: {}}}", string_literal(message))
}

/// `transaction`'s result for a script, without its trace, which tests of
/// the trace check apart.
fn outcome(call: &str) -> String {
    format!(
        r"
        def outcome = {}
        if outcome.ok: outcome else: {{ok: false, error: outcome.error}}
        ",
        call.trim()
    )
}

/// The message of a call made through `db` while a transaction is open.
fn open_transaction(name: &str) -> String {
    format!("Function {name} cannot be used while a transaction is open; use the transaction's tx")
}

/// The message of a call made through an outer `tx` while an inner
/// transaction is open.
fn inner_transaction(name: &str) -> String {
    format!(
        "Function {name} cannot be used while a transaction within it is open; use the inner transaction's tx"
    )
}

/// The message of a call made through a `tx` whose transaction has ended.
fn ended(name: &str) -> String {
    format!("Function {name} requires an open transaction, but it has ended")
}

/// The message of a call made through a `tx` whose transaction SQLite rolled
/// back.
fn rolled_back(name: &str) -> String {
    format!("Function {name} requires an open transaction, but SQLite rolled it back")
}

/// The value of `source`, run with `app.open_db`, a factory of in-memory
/// connections exposing `methods`, and the Frost functions of `entries`.
fn with_app(source: &str, methods: EnumSet<Method>, entries: Vec<(&str, Value)>) -> Value {
    let factory = connection_factory("app.open_db", move |_| {
        Ok(ScriptConnection::from(memory_connection()?).with_methods(methods))
    });
    let mut entries = entries;
    entries.push(("open_db", factory));
    run_app(source, entries)
        .unwrap_or_else(|error| panic!("{source:?} should run, but raised: {error}"))
}

/// `app.keep(value)` and `app.kept()`, which keep a value past the script's
/// reach, as a host may.
fn keeper() -> Vec<(&'static str, Value)> {
    let store = Arc::new(Mutex::new(Value::Null));
    let kept = Arc::clone(&store);
    let keep = frost_closure!("keep", move |value| {
        *store.lock().expect("no test panicked") = value;
    });
    let kept = frost_closure!("kept", move || kept
        .lock()
        .expect("no test panicked")
        .clone());
    vec![("keep", keep), ("kept", kept)]
}

// --- Committing ---

#[test]
fn a_transaction_commits_and_returns_the_rows_it_changed() {
    let source = with_table(
        r#"
        def committed = db.transaction(fn tx -> {
            tx.exec("INSERT INTO t VALUES (1, 'a'), (2, 'b')")
            tx.exec("UPDATE t SET name = 'z' WHERE id = 1")
        })
        [committed, db.query('SELECT * FROM t ORDER BY id')]
        "#,
    );
    assert_values(&[(
        &source,
        "[{ok: true, value: 3}, [{id: 1, name: 'z'}, {id: 2, name: 'b'}]]",
    )]);
}

#[test]
fn a_transaction_returns_its_count_not_its_callbacks_result() {
    assert_values(&[
        (
            &with_db("db.transaction(fn tx -> 'ignored')"),
            "{ok: true, value: 0}",
        ),
        (
            &with_table("db.transaction(fn tx -> tx.query('SELECT * FROM t'))"),
            "{ok: true, value: 0}",
        ),
        // Nor a `tx`, which would outlive its transaction.
        (
            &with_db("db.transaction(fn tx -> tx)"),
            "{ok: true, value: 0}",
        ),
        // A native callback is a callback too.
        (&with_db("db.transaction(type)"), "{ok: true, value: 0}"),
    ]);
}

#[test]
fn a_transaction_counts_changes_as_sqlite_does() {
    // Rows that triggers change count too, as `script` counts them.
    let source = with_table(
        r"
        db.exec('CREATE TABLE log (id)')
        db.exec('CREATE TRIGGER logged AFTER INSERT ON t BEGIN INSERT INTO log VALUES (new.id); END')
        db.transaction(fn tx -> tx.exec('INSERT INTO t (name) VALUES (1), (2)'))
        ",
    );
    assert_values(&[(&source, "{ok: true, value: 4}")]);
    // So do rows an inner transaction changed and rolled back: SQLite's count
    // never goes down.
    let source = with_table(
        r"
        def outer = db.transaction(fn tx -> {
            tx.exec('INSERT INTO t (name) VALUES (1)')
            tx.transaction(fn inner -> {
                inner.exec('INSERT INTO t (name) VALUES (2), (3)')
                assert(false)
            })
        })
        [outer, db.query('SELECT count(*) AS c FROM t')]
        ",
    );
    assert_values(&[(&source, "[{ok: true, value: 3}, [{c: 1}]]")]);
}

#[test]
fn a_transaction_is_seen_by_other_connections_only_once_committed() {
    let scratch = Scratch::new();
    let path = scratch.literal("shared.db");
    let source = format!(
        r"
        def db = sqlite.open({path})
        def other = sqlite.open({path})
        db.exec('CREATE TABLE t (x)')
        def count = fn -> other.query('SELECT count(*) AS c FROM t')[0].c
        def during = mutable_cell()
        db.transaction(fn tx -> {{
            tx.exec('INSERT INTO t VALUES (1)')
            during.exchange([count(), tx.query('SELECT count(*) AS c FROM t')[0].c])
        }})
        [during.get(), count()]
        "
    );
    assert_values(&[(&source, "[[0, 1], 1]")]);
}

#[test]
fn a_transaction_takes_the_write_lock_as_it_begins() {
    // Before writing anything, the transaction keeps another connection from
    // writing, which, waiting for none, fails at once.
    let scratch = Scratch::new();
    let path = scratch.literal("locked.db");
    let source = format!(
        r"
        def db = sqlite.open({path})
        def other = sqlite.open({path})
        db.exec('CREATE TABLE t (x)')
        other.exec('PRAGMA busy_timeout = 0')
        def during = mutable_cell()
        db.transaction(fn tx -> during.exchange([
            try_call(other.exec, ['INSERT INTO t VALUES (1)']).error,
            other.query('SELECT count(*) AS c FROM t'),
        ]))
        during.get()
        "
    );
    assert_values(&[(
        &source,
        "['Function db.exec failed: database is locked (SQLITE_BUSY)', [{c: 0}]]",
    )]);
}

#[test]
fn a_transaction_cannot_begin_on_a_query_only_connection() {
    let source = with_db(&outcome(
        r"
        do {
            db.exec('PRAGMA query_only = 1')
            db.transaction(fn tx -> assert(false, 'never called'))
        }
        ",
    ));
    assert_values(&[(
        &source,
        &failed(
            "Function db.transaction failed: attempt to write a readonly database (SQLITE_READONLY)",
        ),
    )]);
}

// --- Rolling back ---

#[test]
fn a_frost_error_rolls_back_and_is_returned() {
    let source = with_table(
        r"
        def rolled_back = db.transaction(fn tx -> {
            tx.exec('INSERT INTO t (name) VALUES (1)')
            assert(false, {reason: 'refused'})
        })
        [rolled_back.ok, rolled_back.error, db.query('SELECT * FROM t')]
        ",
    );
    assert_values(&[(&source, "[false, {reason: 'refused'}, []]")]);
}

#[test]
fn a_sqlite_error_rolls_back_and_is_returned() {
    let source = with_rows(
        r"
        def outcome = db.transaction(fn tx -> {
            tx.exec('DELETE FROM t WHERE id = 1')
            tx.exec('INSERT INTO t VALUES (2, 2)')
        })
        [outcome.ok, outcome.error, outcome.trace, db.query('SELECT id FROM t ORDER BY id')]
        ",
    );
    assert_values(&[(
        &source,
        "[false, 'Function tx.exec failed: UNIQUE constraint failed: t.id (SQLITE_CONSTRAINT)', ['tx.exec', '<lambda>'], [{id: 1}, {id: 2}, {id: 3}]]",
    )]);
}

#[test]
fn a_callback_of_the_wrong_arity_fails_the_transaction() {
    for (callback, count) in [("fn -> 1", 0), ("fn a, b -> 1", 2)] {
        assert_values(&[(
            &with_db(&outcome(&format!("db.transaction({callback})"))),
            &failed(&format!(
                "Function <lambda> expects {count} arguments, but was called with 1"
            )),
        )]);
    }
}

#[test]
fn a_failed_commit_rolls_back_and_is_returned() {
    // A deferred foreign key is checked only as the transaction commits.
    let source = with_db(
        r"
        db.exec('CREATE TABLE parent (id INTEGER PRIMARY KEY)')
        db.exec('CREATE TABLE child (parent REFERENCES parent (id) DEFERRABLE INITIALLY DEFERRED)')
        def outcome = db.transaction(fn tx -> tx.exec('INSERT INTO child VALUES (1)'))
        [outcome, db.query('SELECT * FROM child'), db.exec('INSERT INTO parent VALUES (1)')]
        ",
    );
    // The error is the transaction's own, so no frame saw it pass.
    assert_values(&[(
        &source,
        "[{ok: false, error: 'Function db.transaction failed: FOREIGN KEY constraint failed (SQLITE_CONSTRAINT)', trace: []}, [], 1]",
    )]);
}

#[test]
fn a_failure_is_reported_as_try_call_reports_it() {
    let source = with_db(
        r"
        defn refuse(tx) -> assert(false, 'refused')
        [db.transaction(refuse), try_call(refuse, [null])]
        ",
    );
    let both = value(&source);
    let Value::Array(both) = both else {
        panic!("the script returns an Array, not {both:?}");
    };
    assert_eq!(
        both.as_slice()[0],
        both.as_slice()[1],
        "the same Map, trace and all"
    );
}

#[test]
fn a_connection_works_on_after_a_rollback() {
    let source = with_table(
        r"
        db.transaction(fn tx -> assert(false))
        db.transaction(fn tx -> tx.exec('NOT VALID SQL'))
        def again = db.transaction(fn tx -> tx.exec('INSERT INTO t (name) VALUES (1)'))
        [again, db.exec('INSERT INTO t (name) VALUES (2)')]
        ",
    );
    assert_values(&[(&source, "[{ok: true, value: 1}, 1]")]);
}

/// A factory of pooled connections to the database at `path`, recording in
/// `released` whether each returned to its pool with no transaction open.
fn pooled_file_factory(path: std::path::PathBuf, released: &Arc<Mutex<Option<bool>>>) -> Value {
    let released = Arc::clone(released);
    connection_factory("app.open_db", move |_| {
        let connection =
            Connection::open(&path).map_err(|err| sqlite_error("app.open_db", &err))?;
        let released = Arc::clone(&released);
        Ok(ScriptConnection::pooled(Pooled::new(
            connection,
            move |connection| {
                *released.lock().expect("no test panicked") = Some(connection.is_autocommit());
            },
        )))
    })
}

/// The count of rows in `t` of the database at `path`.
fn rows_in(path: &std::path::Path) -> i64 {
    Connection::open(path)
        .and_then(|connection| connection.query_row("SELECT count(*) FROM t", [], |row| row.get(0)))
        .expect("the database reads")
}

#[test]
fn a_run_cancelled_within_a_transaction_rolls_it_back() {
    // Pooled, so that it stays open after the run, as closing would roll
    // back any transaction of itself.
    let scratch = Scratch::new();
    let path = scratch.path("cancelled.db");
    Connection::open(&path)
        .and_then(|connection| connection.execute_batch("CREATE TABLE t (x)"))
        .expect("the database is set up");
    let released = Arc::new(Mutex::new(None));
    let token = CancelToken::new();
    let cancelling = token.clone();
    let cancel = frost_closure!("cancel", move || cancelling.cancel());
    let app = HostComponent::builder("app")
        .value("open_db", pooled_file_factory(path.clone(), &released))
        .value("cancel", cancel)
        .build();
    let source = r"
        def app = import('app')
        app.open_db().transaction(fn tx -> {
            tx.exec('INSERT INTO t VALUES (1)')
            app.cancel()
            tx.exec('INSERT INTO t VALUES (2)')
        })
    ";
    let closure = compile_program("main.frst", source, CompilerOptions::new())
        .expect("the script compiles")
        .code
        .into_closure()
        .expect("the script captures nothing");
    let run = Vm::factory()
        .with_importer(host_importer(app))
        .build(closure)
        .with_cancel_token(token)
        .run();
    let Err(failed) = run else {
        panic!("the run should be cancelled");
    };
    assert_eq!(failed.kind(), RunErrorKind::Aborted(AbortReason::Cancelled));
    drop(failed);
    assert_eq!(
        *released.lock().expect("no test panicked"),
        Some(true),
        "released with no transaction open"
    );
    assert_eq!(rows_in(&path), 0, "the transaction was rolled back");
}

#[test]
fn a_host_function_panicking_within_a_transaction_rolls_it_back() {
    let scratch = Scratch::new();
    let path = scratch.path("panicked.db");
    Connection::open(&path)
        .and_then(|connection| connection.execute_batch("CREATE TABLE t (x)"))
        .expect("the database is set up");
    let released = Arc::new(Mutex::new(None));
    let factory = pooled_file_factory(path.clone(), &released);
    let explode = frost_closure!("explode", || -> i64 {
        panic!("a host function with a bug")
    });
    let source = r"
        def app = import('app')
        app.open_db().transaction(fn tx -> {
            tx.exec('INSERT INTO t VALUES (1)')
            app.explode()
        })
    ";
    let panicked = panic::catch_unwind(AssertUnwindSafe(|| {
        run_app(source, vec![("open_db", factory), ("explode", explode)])
    }));
    assert!(panicked.is_err(), "the host function panicked");
    assert_eq!(
        *released.lock().expect("no test panicked"),
        Some(true),
        "released with no transaction open"
    );
    assert_eq!(rows_in(&path), 0, "the transaction was rolled back");
}

// --- SQLite rolling a transaction back itself ---

#[test]
fn a_transaction_sqlite_rolls_back_cannot_be_used_or_committed() {
    // An `OR ROLLBACK` conflict ends the whole transaction. Caught, it leaves
    // the `tx` spent, and the transaction failing as it ends.
    let source = with_db(
        r"
        db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY)')
        db.exec('INSERT INTO t VALUES (1)')
        def attempts = mutable_cell()
        def outcome = db.transaction(fn tx -> {
            tx.exec('INSERT INTO t VALUES (2)')
            def conflict = try_call(tx.exec, ['INSERT OR ROLLBACK INTO t VALUES (1)'])
            def after = try_call(tx.exec, ['INSERT INTO t VALUES (3)'])
            attempts.exchange([conflict.error, after.error])
        })
        [outcome, attempts.get(), db.query('SELECT id FROM t')]
        ",
    );
    assert_values(&[(
        &source,
        &format!(
            "[{}, ['Function tx.exec failed: UNIQUE constraint failed: t.id (SQLITE_CONSTRAINT)', {}], [{{id: 1}}]]",
            "{ok: false, error: 'Function db.transaction cannot commit the transaction, as SQLite rolled it back', trace: []}",
            string_literal(&rolled_back("tx.exec")),
        ),
    )]);
}

#[test]
fn a_rollback_sqlite_makes_within_a_savepoint_ends_every_level() {
    // A trigger's `RAISE(ROLLBACK)` ends the whole transaction, not only the
    // savepoint it was raised in.
    let source = with_db(
        r#"
        db.exec('CREATE TABLE t (id)')
        db.exec("CREATE TRIGGER refuse BEFORE INSERT ON t WHEN new.id = 99 BEGIN SELECT RAISE(ROLLBACK, 'refused'); END")
        def attempts = mutable_cell()
        def outcome = db.transaction(fn tx -> {
            tx.exec('INSERT INTO t VALUES (1)')
            def inner = tx.transaction(fn inner -> {
                def raised = try_call(inner.exec, ['INSERT INTO t VALUES (99)'])
                def after = try_call(inner.exec, ['INSERT INTO t VALUES (2)'])
                [raised.error, after.error]
            })
            def outer_after = try_call(tx.exec, ['INSERT INTO t VALUES (3)'])
            attempts.exchange([inner.error, outer_after.error])
        })
        [outcome.error, attempts.get(), db.query('SELECT id FROM t')]
        "#,
    );
    assert_values(&[(
        &source,
        &format!(
            "['Function db.transaction cannot commit the transaction, as SQLite rolled it back', ['Function tx.transaction cannot commit the transaction, as SQLite rolled it back', {}], []]",
            string_literal(&rolled_back("tx.exec")),
        ),
    )]);
}

#[test]
fn a_rollback_sqlite_makes_after_an_inner_transaction_ends_it_too() {
    let source = with_db(
        r"
        db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY)')
        db.exec('INSERT INTO t VALUES (1)')
        def outcome = db.transaction(fn tx -> {
            tx.transaction(fn inner -> inner.exec('INSERT INTO t VALUES (2)'))
            try_call(tx.exec, ['INSERT OR ROLLBACK INTO t VALUES (1)'])
            tx.transaction(fn inner -> inner.exec('INSERT INTO t VALUES (3)'))
        })
        [outcome.ok, outcome.error, db.query('SELECT id FROM t')]
        ",
    );
    // The second inner transaction cannot begin: the `tx` it is made through
    // is spent, so the call raises, failing the transaction.
    assert_values(&[(
        &source,
        &format!(
            "[false, {}, [{{id: 1}}]]",
            string_literal(&rolled_back("tx.transaction"))
        ),
    )]);
}

#[test]
fn a_pooled_connection_rolls_back_a_transaction_left_open_as_it_is_released() {
    // The host lends the connection with a transaction of its own open.
    let released = Arc::new(Mutex::new(None));
    let releases = Arc::clone(&released);
    let factory = connection_factory("app.open_db", move |_| {
        let connection = memory_connection()?;
        connection
            .execute_batch("CREATE TABLE t (x); BEGIN; INSERT INTO t VALUES (1)")
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        let releases = Arc::clone(&releases);
        Ok(ScriptConnection::pooled(Pooled::new(
            connection,
            move |connection| {
                let rows: i64 = connection
                    .query_row("SELECT count(*) FROM t", [], |row| row.get(0))
                    .expect("the table reads");
                *releases.lock().expect("no test panicked") =
                    Some((connection.is_autocommit(), rows));
            },
        )))
    });
    let source = r"
        def db = import('app').open_db()
        db.query('SELECT x FROM t')
    ";
    assert_eq!(
        run_app(source, vec![("open_db", factory)]).expect("the script runs"),
        frost("[{x: 1}]")
    );
    assert_eq!(
        *released.lock().expect("no test panicked"),
        Some((true, 0)),
        "released with the transaction rolled back"
    );
}

// --- The transaction's tx ---

#[test]
fn a_tx_offers_the_connections_methods_but_close() {
    assert_values(&[(
        &with_db(
            r"
            def keys_of = mutable_cell()
            db.transaction(fn tx -> keys_of.exchange(sorted(keys(tx))))
            keys_of.get()
            ",
        ),
        "['exec', 'fold', 'last_insert_rowid', 'query', 'script', 'transaction']",
    )]);
}

#[test]
fn a_tx_sees_its_own_changes() {
    let source = with_db(
        r"
        db.exec('CREATE TABLE n (id INTEGER PRIMARY KEY, x)')
        def seen = mutable_cell()
        db.transaction(fn tx -> {
            tx.script('INSERT INTO n (x) VALUES (1); INSERT INTO n (x) VALUES (2)')
            tx.exec('INSERT INTO n (x) VALUES (?)', [3])
            seen.exchange([
                tx.query('SELECT x FROM n ORDER BY id'),
                tx.fold('SELECT x FROM n', 0, fn total, row -> total + row.x),
                tx.last_insert_rowid(),
            ])
        })
        seen.get()
        ",
    );
    assert_values(&[(&source, "[[{x: 1}, {x: 2}, {x: 3}], 6, 3]")]);
}

#[test]
fn the_connection_is_unusable_while_a_transaction_is_open() {
    for (call, name) in [
        ("db.exec('SELECT 1')", "db.exec"),
        ("db.query('SELECT 1')", "db.query"),
        ("db.fold('SELECT 1', null, fn acc, row -> acc)", "db.fold"),
        ("db.script('SELECT 1')", "db.script"),
        ("db.last_insert_rowid()", "db.last_insert_rowid"),
        ("db.transaction(fn other -> null)", "db.transaction"),
    ] {
        let source = with_db(&format!(
            r"
            def attempt = mutable_cell()
            db.transaction(fn tx -> attempt.exchange(try_call(fn -> {call}).error))
            attempt.get()
            "
        ));
        assert_values(&[(&source, &string_literal(&open_transaction(name)))]);
    }
    // Uncaught, such a call fails the transaction.
    assert_values(&[
        (
            &with_db(&outcome("db.transaction(fn tx -> db.exec('SELECT 1'))")),
            &failed(&open_transaction("db.exec")),
        ),
        (
            &with_db(&outcome("db.transaction(fn tx -> db.close())")),
            &failed("Function db.close cannot close the connection while it is in use"),
        ),
    ]);
    // Once the transaction ends, the connection is free again.
    assert_values(&[(
        &with_db(
            r"
            db.transaction(fn tx -> null)
            db.exec('SELECT 1')
            ",
        ),
        "0",
    )]);
}

#[test]
fn a_transaction_on_a_closed_connection_raises() {
    assert_raises(&[(
        &with_db(
            r"
            db.close()
            db.transaction(fn tx -> null)
            ",
        ),
        "Function db.transaction requires an open connection, but it is closed",
    )]);
}

#[test]
fn an_ended_tx_cannot_be_used() {
    let source = r"
        def app = import('app')
        def db = app.open_db()
        db.exec('CREATE TABLE t (x)')
        db.transaction(fn tx -> app.keep(tx))
        def tx = app.kept()
        def attempt = fn call -> try_call(call).error
        def ended = [
            attempt(fn -> tx.exec('SELECT 1')),
            attempt(fn -> tx.query('SELECT 1')),
            attempt(fn -> tx.fold('SELECT 1', null, fn acc, row -> acc)),
            attempt(fn -> tx.script('SELECT 1')),
            attempt(fn -> tx.last_insert_rowid()),
            attempt(fn -> tx.transaction(fn inner -> null)),
        ]
        # Nor while another transaction is open.
        def during = mutable_cell()
        db.transaction(fn other -> during.exchange(attempt(fn -> tx.exec('SELECT 1'))))
        [ended, during.get()]
    ";
    let message = |name: &str| Value::from(ended(name));
    assert_eq!(
        with_app(source, EnumSet::all(), keeper()),
        Value::array([
            Value::array([
                message("tx.exec"),
                message("tx.query"),
                message("tx.fold"),
                message("tx.script"),
                message("tx.last_insert_rowid"),
                message("tx.transaction"),
            ]),
            message("tx.exec"),
        ])
    );
}

// --- Savepoints ---

#[test]
fn an_inner_transaction_commits_into_the_outer_one() {
    let source = with_table(
        r"
        def results = mutable_cell()
        def outer = db.transaction(fn tx -> {
            tx.exec('INSERT INTO t (name) VALUES (1)')
            def inner = tx.transaction(fn inner -> inner.exec('INSERT INTO t (name) VALUES (2), (3)'))
            results.exchange([inner, tx.query('SELECT count(*) AS c FROM t')[0].c])
        })
        [outer, results.get(), db.query('SELECT count(*) AS c FROM t')[0].c]
        ",
    );
    assert_values(&[(
        &source,
        "[{ok: true, value: 3}, [{ok: true, value: 2}, 3], 3]",
    )]);
}

#[test]
fn a_failing_inner_transaction_rolls_back_only_its_own_work() {
    let source = with_table(
        r"
        def results = mutable_cell()
        def outer = db.transaction(fn tx -> {
            tx.exec('INSERT INTO t (name) VALUES (1)')
            def inner = tx.transaction(fn inner -> {
                inner.exec('INSERT INTO t (name) VALUES (2)')
                assert(false, 'inner refused')
            })
            tx.exec('INSERT INTO t (name) VALUES (3)')
            results.exchange([inner.ok, inner.error])
        })
        [outer.ok, results.get(), db.query('SELECT name FROM t ORDER BY id')]
        ",
    );
    assert_values(&[(
        &source,
        "[true, [false, 'Failed assertion: inner refused'], [{name: '1'}, {name: '3'}]]",
    )]);
}

#[test]
fn an_outer_failure_rolls_back_its_committed_inner_transactions() {
    let source = with_table(
        r"
        db.transaction(fn tx -> {
            tx.transaction(fn inner -> inner.exec('INSERT INTO t (name) VALUES (1)'))
            assert(false)
        })
        db.query('SELECT * FROM t')
        ",
    );
    assert_values(&[(&source, "[]")]);
}

#[test]
fn transactions_nest_deeply() {
    // Each level inserts its depth, then nests a level; the innermost fails,
    // rolling back only its own insert.
    let source = with_db(
        r"
        db.exec('CREATE TABLE t (depth)')
        defn nest(tx, depth) -> {
            tx.exec('INSERT INTO t VALUES (?)', [depth])
            if depth == 100: assert(false) else: do {
                def inner = tx.transaction(fn inner -> nest(inner, depth + 1))
                if depth == 99: assert(not inner.ok) else: assert(inner.ok)
            }
        }
        def outer = db.transaction(fn tx -> nest(tx, 1))
        [outer, db.query('SELECT count(*) AS c, max(depth) AS deepest FROM t')]
        ",
    );
    assert_values(&[(&source, "[{ok: true, value: 100}, [{c: 99, deepest: 99}]]")]);
}

#[test]
fn rollbacks_chain_through_the_levels_that_fail() {
    // The innermost fails, then the level holding it: both roll back, and
    // the outer transaction keeps only its own work.
    let source = with_table(
        r"
        def outer = db.transaction(fn tx -> {
            tx.exec('INSERT INTO t (name) VALUES (1)')
            tx.transaction(fn middle -> {
                middle.exec('INSERT INTO t (name) VALUES (2)')
                middle.transaction(fn innermost -> {
                    innermost.exec('INSERT INTO t (name) VALUES (3)')
                    assert(false)
                })
                assert(false)
            })
        })
        [outer.ok, db.query('SELECT name FROM t')]
        ",
    );
    assert_values(&[(&source, "[true, [{name: '1'}]]")]);
}

#[test]
fn the_outer_tx_is_unusable_while_an_inner_transaction_is_open() {
    let source = with_db(
        r"
        def attempt = fn call -> try_call(call).error
        def attempts = mutable_cell()
        db.transaction(fn tx -> tx.transaction(fn inner -> attempts.exchange([
            attempt(fn -> tx.exec('SELECT 1')),
            attempt(fn -> tx.query('SELECT 1')),
            attempt(fn -> tx.fold('SELECT 1', null, fn acc, row -> acc)),
            attempt(fn -> tx.script('SELECT 1')),
            attempt(fn -> tx.last_insert_rowid()),
            attempt(fn -> tx.transaction(fn other -> null)),
            attempt(fn -> db.exec('SELECT 1')),
            inner.exec('SELECT 1'),
        ])))
        attempts.get()
        ",
    );
    let blocked = |name: &str| string_literal(&inner_transaction(name));
    assert_values(&[(
        &source,
        &format!(
            "[{}, {}, {}, {}, {}, {}, {}, 0]",
            blocked("tx.exec"),
            blocked("tx.query"),
            blocked("tx.fold"),
            blocked("tx.script"),
            blocked("tx.last_insert_rowid"),
            blocked("tx.transaction"),
            string_literal(&open_transaction("db.exec")),
        ),
    )]);
}

#[test]
fn an_ended_inner_tx_cannot_be_used_while_the_outer_is_open() {
    let source = r"
        def app = import('app')
        def db = app.open_db()
        def results = mutable_cell()
        db.transaction(fn tx -> {
            tx.transaction(fn inner -> app.keep(inner))
            def attempt = try_call(fn -> app.kept().exec('SELECT 1'))
            results.exchange([attempt.error, tx.exec('SELECT 1')])
        })
        results.get()
    ";
    assert_eq!(
        with_app(source, EnumSet::all(), keeper()),
        Value::array([Value::from(ended("tx.exec")), Value::Int(0)])
    );
}

// --- Callbacks reentering the connection, and other connections ---

#[test]
fn a_rollback_undoing_a_schema_change_ends_a_fold_in_progress() {
    // SQLite ends every read in progress as such a rollback undoes the schema
    // they read under. Rolling back data alone ends none.
    let source = with_rows(
        r"
        def ended = try_call(db.fold, ['SELECT id FROM t', null, fn acc, row -> {
            db.transaction(fn tx -> {
                tx.exec('CREATE TABLE u (x)')
                assert(false)
            })
        }])
        [ended.error, db.query('SELECT name FROM sqlite_schema')]
        ",
    );
    assert_values(&[(
        &source,
        "['Function db.fold failed: abort due to ROLLBACK (SQLITE_ABORT)', [{name: 't'}]]",
    )]);
}

#[test]
fn a_transaction_may_run_within_a_fold() {
    let source = with_rows(
        r"
        db.exec('CREATE TABLE log (id)')
        def outcomes = db.fold('SELECT id FROM t ORDER BY id', [], fn acc, row -> {
            def outcome = db.transaction(fn tx -> {
                tx.exec('INSERT INTO log VALUES (?)', [row.id])
                if row.id == 2: assert(false) else: null
            })
            acc + [outcome.ok]
        })
        [outcomes, db.query('SELECT id FROM log ORDER BY id')]
        ",
    );
    assert_values(&[(&source, "[[true, false, true], [{id: 1}, {id: 3}]]")]);
}

#[test]
fn a_transaction_may_run_within_a_fold_over_a_statement_that_writes() {
    // The statement runs in full before the first call, so neither loses the
    // other's work.
    let source = with_db(
        r"
        db.exec('CREATE TABLE t (id)')
        db.exec('CREATE TABLE log (id)')
        def outcomes = db.fold('INSERT INTO t VALUES (1), (2) RETURNING id', [], fn acc, row -> {
            def outcome = db.transaction(fn tx -> tx.exec('INSERT INTO log VALUES (?)', [row.id]))
            acc + [outcome.ok]
        })
        [outcomes, db.query('SELECT id FROM t ORDER BY id'), db.query('SELECT id FROM log ORDER BY id')]
        ",
    );
    assert_values(&[(
        &source,
        "[[true, true], [{id: 1}, {id: 2}], [{id: 1}, {id: 2}]]",
    )]);
    // So within a transaction, a savepoint.
    let source = with_db(
        r"
        db.exec('CREATE TABLE t (id)')
        def outer = db.transaction(fn tx -> {
            tx.fold('INSERT INTO t VALUES (1), (2) RETURNING id', null, fn acc, row -> {
                assert(tx.transaction(fn inner -> inner.exec('INSERT INTO t VALUES (?)', [row.id * 10])).ok)
            })
        })
        [outer, db.query('SELECT id FROM t ORDER BY id')]
        ",
    );
    assert_values(&[(
        &source,
        "[{ok: true, value: 4}, [{id: 1}, {id: 2}, {id: 10}, {id: 20}]]",
    )]);
}

#[test]
fn a_fold_within_a_transaction_may_reenter_it() {
    let source = with_rows(
        r"
        db.exec('CREATE TABLE log (id)')
        def outer = db.transaction(fn tx -> {
            tx.fold('SELECT id FROM t ORDER BY id', null, fn acc, row -> {
                tx.exec('INSERT INTO log VALUES (?)', [row.id])
                tx.transaction(fn inner -> {
                    inner.exec('INSERT INTO log VALUES (?)', [row.id * 10])
                    if row.id == 2: assert(false) else: null
                })
            })
        })
        [outer.ok, db.query('SELECT id FROM log ORDER BY id')]
        ",
    );
    assert_values(&[(
        &source,
        "[true, [{id: 1}, {id: 2}, {id: 3}, {id: 10}, {id: 30}]]",
    )]);
}

#[test]
fn transactions_on_other_connections_are_their_own() {
    let source = r"
        def db = sqlite.open_memory()
        def other = sqlite.open_memory()
        db.exec('CREATE TABLE t (x)')
        other.exec('CREATE TABLE t (x)')
        db.transaction(fn tx -> {
            tx.exec('INSERT INTO t VALUES (1)')
            other.transaction(fn other_tx -> {
                other_tx.exec('INSERT INTO t VALUES (9)')
                tx.exec('INSERT INTO t VALUES (2)')
                assert(false)
            })
            other.exec('INSERT INTO t VALUES (3)')
        })
        [db.query('SELECT x FROM t ORDER BY x'), other.query('SELECT x FROM t')]
    ";
    assert_values(&[(source, "[[{x: 1}, {x: 2}], [{x: 3}]]")]);
}

// --- No tx escapes ---

/// An Opaque owning heap data, which might hold a Value.
#[derive(Debug)]
struct Owning(#[allow(dead_code)] Vec<u8>);

impl FrostOpaque for Owning {
    fn type_name() -> &'static str {
        "Owning"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// An Opaque with no drop glue, which provably holds no Value.
#[derive(Debug)]
struct Plain(#[allow(dead_code)] u64);

impl FrostOpaque for Plain {
    fn type_name() -> &'static str {
        "Plain"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// The start of the message replacing an error that may hold a Function.
const ESCAPING: &str =
    "Function db.transaction cannot return an error that holds, or may hold, a Function: ";

#[test]
fn an_error_that_may_hold_a_function_is_not_returned() {
    for thrown in [
        "tx",
        "tx.exec",
        "[1, {deep: tx.query}]",
        "fn -> tx.exec('SELECT 1')",
        "{f: fn -> 1}",
    ] {
        let source = with_table(&format!(
            r"
            def outcome = db.transaction(fn tx -> {{
                tx.exec('INSERT INTO t (name) VALUES (1)')
                assert(false, {thrown})
            }})
            [outcome.ok, outcome.error, outcome.trace, db.query('SELECT * FROM t')]
            "
        ));
        let Value::Array(result) = value(&source) else {
            panic!("the script returns an Array");
        };
        let [ok, error, trace, rows] = result.as_slice() else {
            panic!("the script returns four values");
        };
        assert_eq!(ok, &Value::Bool(false), "{thrown}");
        let Value::String(error) = error else {
            panic!("{thrown}: the error is replaced by a message, not {error:?}");
        };
        assert!(
            error.starts_with(ESCAPING),
            "{thrown}: {error:?} says why it was replaced, then what it was"
        );
        assert_eq!(
            trace,
            &value("[]"),
            "{thrown}: the replacement has no trace"
        );
        assert_eq!(rows, &value("[]"), "{thrown}: the transaction rolled back");
    }
    // An Opaque that might hold one.
    let source = r"
        def app = import('app')
        app.open_db().transaction(fn tx -> assert(false, [app.owning()])).error
    ";
    let owning = Value::opaque(Owning(vec![1]));
    let owning = frost_closure!("owning", move || owning.clone());
    let Value::String(error) = with_app(source, EnumSet::all(), vec![("owning", owning)]) else {
        panic!("the error is replaced by a message");
    };
    assert!(error.starts_with(ESCAPING), "{error:?}");
}

#[test]
fn an_error_that_cannot_hold_a_function_is_returned_as_it_is() {
    let source = r"
        def app = import('app')
        app.open_db().transaction(fn tx -> assert(false, [app.special(), app.plain()])).error
    ";
    let special = Value::opaque(SpecialFloat::new(f64::INFINITY).expect("an infinity is special"));
    let plain = Value::opaque(Plain(1));
    let (held_special, held_plain) = (special.clone(), plain.clone());
    let entries = vec![
        (
            "special",
            frost_closure!("special", move || held_special.clone()),
        ),
        ("plain", frost_closure!("plain", move || held_plain.clone())),
    ];
    assert_eq!(
        with_app(source, EnumSet::all(), entries),
        Value::array([special, plain])
    );
}

#[test]
fn an_inner_error_that_may_hold_a_function_is_not_returned() {
    let source = with_db(
        r"
        def inner_error = mutable_cell()
        db.transaction(fn tx -> inner_error.exchange(tx.transaction(fn inner -> assert(false, inner)).error))
        inner_error.get()
        ",
    );
    let Value::String(error) = value(&source) else {
        panic!("the error is replaced by a message");
    };
    assert!(
        error.starts_with(
            "Function tx.transaction cannot return an error that holds, or may hold, a Function: "
        ),
        "{error:?}"
    );
}

// --- Trimming ---

#[test]
fn a_tx_has_the_methods_its_connection_was_trimmed_to() {
    let source = r"
        def app = import('app')
        def keys_of = mutable_cell()
        app.open_db().transaction(fn tx -> keys_of.exchange(sorted(keys(tx))))
        keys_of.get()
    ";
    for (methods, expected) in [
        (
            Method::Exec | Method::Transaction,
            "['exec', 'transaction']",
        ),
        (
            Method::Query | Method::Transaction | Method::Close,
            "['query', 'transaction']",
        ),
        (EnumSet::only(Method::Transaction), "['transaction']"),
    ] {
        assert_eq!(
            with_app(source, methods, Vec::new()),
            frost(expected),
            "{methods:?}"
        );
    }
}

#[test]
fn without_the_transaction_method_there_are_no_transactions() {
    let source = r"
        def app = import('app')
        def db = app.open_db()
        [keys(db) @ contains('transaction'), try_call(db.exec, ['BEGIN']).error]
    ";
    assert_eq!(
        with_app(source, EnumSet::all() - Method::Transaction, Vec::new()),
        frost("[false, 'Function db.exec cannot run a transaction statement']"),
        "no method to suggest"
    );
}

// --- The host's authorizer ---

/// A host's own authorizer, set on the connection with rusqlite, denying
/// transactions from beginning.
fn denying_begins(ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        AuthAction::Transaction { .. } | AuthAction::Savepoint { .. } => Authorization::Deny,
        _ => Authorization::Allow,
    }
}

#[test]
fn a_transaction_the_hosts_own_authorizer_denies_does_not_begin() {
    let factory = connection_factory("app.open_db", |_| {
        let connection = memory_connection()?;
        connection
            .authorizer(Some(denying_begins))
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        Ok(ScriptConnection::new(connection))
    });
    let source = r"
        def db = import('app').open_db()
        def called = mutable_cell(false)
        def outcome = db.transaction(fn tx -> called.exchange(true))
        [outcome.ok, outcome.error, called.get(), db.exec('SELECT 1')]
    ";
    assert_eq!(
        run_app(source, vec![("open_db", factory)]).expect("the script runs"),
        frost("[false, 'Function db.transaction failed: not authorized (SQLITE_AUTH)', false, 0]")
    );
}

// --- Arity and types ---

#[test]
fn transaction_checks_its_arity() {
    assert_arity(
        "sqlite.open_memory().transaction",
        "db.transaction",
        1,
        &[0, 2],
    );
    for (argc, args) in [(0, ""), (2, "null, null")] {
        let source = with_db(&format!(
            r"
            def message = mutable_cell()
            db.transaction(fn tx -> message.exchange(try_call(fn -> tx.transaction({args})).error))
            message.get()
            "
        ));
        assert_values(&[(
            &source,
            &string_literal(&format!(
                "Function tx.transaction expects 1 argument, but was called with {argc}"
            )),
        )]);
    }
}

#[test]
fn transaction_requires_a_function() {
    for (argument, kind) in [
        ("null", "Null"),
        ("true", "Bool"),
        ("1", "Int"),
        ("1.5", "Float"),
        ("'text'", "String"),
        ("x'00'", "Bytes"),
        ("[]", "Array"),
        ("{}", "Map"),
    ] {
        assert_raises(&[(
            &with_db(&format!("db.transaction({argument})")),
            &format!("Function db.transaction requires Function as argument 1, got {kind}"),
        )]);
        let source = with_db(&format!(
            r"
            def message = mutable_cell()
            db.transaction(fn tx -> message.exchange(try_call(fn -> tx.transaction({argument})).error))
            message.get()
            "
        ));
        assert_values(&[(
            &source,
            &string_literal(&format!(
                "Function tx.transaction requires Function as argument 1, got {kind}"
            )),
        )]);
    }
}

/// The error `call` raises, made on a `tx` kept from a transaction that has
/// ended: arguments are checked before the transaction is.
fn kept_tx_raises(call: &str) -> Value {
    let source = format!(
        r"
        def app = import('app')
        app.open_db().transaction(fn tx -> app.keep(tx))
        def tx = app.kept()
        try_call(fn -> {call}).error
        "
    );
    with_app(&source, EnumSet::all(), keeper())
}

#[test]
fn a_tx_method_checks_its_arguments_as_the_connections_does() {
    let arity =
        |name: &str, expects: &str, argc: usize| Value::from(arity_error(name, expects, argc));
    for (call, expected) in [
        ("tx.exec()", arity("tx.exec", "between 1 and 2", 0)),
        (
            "tx.exec('SELECT 1', [], [])",
            arity("tx.exec", "between 1 and 2", 3),
        ),
        ("tx.query()", arity("tx.query", "between 1 and 2", 0)),
        (
            "tx.query('SELECT 1', [], [])",
            arity("tx.query", "between 1 and 2", 3),
        ),
        (
            "tx.fold('SELECT 1', null)",
            arity("tx.fold", "between 3 and 4", 2),
        ),
        ("tx.script()", arity("tx.script", "1", 0)),
        ("tx.script('SELECT 1', [])", arity("tx.script", "1", 2)),
        (
            "tx.last_insert_rowid(1)",
            arity("tx.last_insert_rowid", "0", 1),
        ),
    ] {
        assert_eq!(kept_tx_raises(call), expected, "{call}");
    }
    for (argument, kind) in NON_STRINGS {
        for method in ["exec", "query", "script"] {
            assert_eq!(
                kept_tx_raises(&format!("tx.{method}({argument})")),
                Value::from(format!(
                    "Function tx.{method} requires String as argument 1 (sql), got {kind}"
                )),
            );
        }
        assert_eq!(
            kept_tx_raises(&format!("tx.fold({argument}, null, fn acc, row -> acc)")),
            Value::from(format!(
                "Function tx.fold requires String as argument 1 (sql), got {kind}"
            )),
        );
    }
    for (call, expected) in [
        (
            "tx.exec('SELECT 1', 1)",
            "Function tx.exec requires Structured as argument 2 (bindings), got Int",
        ),
        (
            "tx.query('SELECT 1', 'x')",
            "Function tx.query requires Structured as argument 2 (bindings), got String",
        ),
        (
            "tx.fold('SELECT 1', null, 1)",
            "Function tx.fold requires Function as argument 3, got Int",
        ),
        (
            "tx.fold('SELECT 1', [], null, 1)",
            "Function tx.fold requires Function as argument 4, got Int",
        ),
    ] {
        assert_eq!(kept_tx_raises(call), Value::from(expected), "{call}");
    }
}
