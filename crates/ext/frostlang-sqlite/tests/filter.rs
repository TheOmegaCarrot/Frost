//! SQL filters: a host's check of the SQL a script passes, before anything
//! else is done with it.

use std::sync::{Arc, Mutex};

use frostlang::{FrostError, MapKey, Value};
use frostlang_sqlite::{Authorizer, ScriptConnection, connection_factory};

use crate::common::{app_raises, app_value, frost, memory_connection};

/// The SQL a filter has seen, in order.
type Seen = Arc<Mutex<Vec<String>>>;

/// A factory of in-memory connections whose filter records the SQL it sees
/// in `seen`, then judges it with `judge`.
fn filtered(
    seen: &Seen,
    judge: impl Fn(&str) -> Result<(), FrostError> + Clone + Send + Sync + 'static,
) -> Value {
    let seen = Arc::clone(seen);
    connection_factory("app.open_db", move |_| {
        let seen = Arc::clone(&seen);
        let judge = judge.clone();
        Ok(
            ScriptConnection::from(memory_connection()?).with_sql_filter(move |sql| {
                seen.lock().expect("no test panicked").push(sql.to_string());
                judge(sql)
            }),
        )
    })
}

/// What `seen` holds.
fn seen_sql(seen: &Seen) -> Vec<String> {
    seen.lock().expect("no test panicked").clone()
}

#[test]
fn a_filter_sees_each_calls_sql_exactly_as_given() {
    let seen = Seen::default();
    let source = r"
        def db = import('app').open_db()
        db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')
        db.script('  insert into T values (2) -- spaced, cased, commented  ')
        db.script('')
    ";
    assert_eq!(
        app_value(source, filtered(&seen, |_| Ok(()))),
        Value::Int(0)
    );
    assert_eq!(
        seen_sql(&seen),
        [
            "CREATE TABLE t (x); INSERT INTO t VALUES (1)",
            "  insert into T values (2) -- spaced, cased, commented  ",
            "",
        ],
        "one call each, with the SQL untouched"
    );
}

#[test]
fn a_filter_sees_sql_before_sqlite_does() {
    let seen = Seen::default();
    assert_eq!(
        app_raises(
            "import('app').open_db().script('NOT VALID SQL')",
            filtered(&seen, |_| Ok(()))
        ),
        r#"Function db.script failed: near "NOT": syntax error (SQLITE_ERROR)"#
    );
    assert_eq!(seen_sql(&seen), ["NOT VALID SQL"]);
}

#[test]
fn a_filters_error_is_the_calls_error() {
    let seen = Seen::default();
    let refuse_drops = |sql: &str| {
        if sql.to_ascii_lowercase().contains("drop") {
            Err(FrostError::from_static("no dropping"))
        } else {
            Ok(())
        }
    };
    let source = r"
        def db = import('app').open_db()
        [
            db.script('CREATE TABLE t (x)'),
            try_call(db.script, ['INSERT INTO t VALUES (1); DROP TABLE t']).error,
            db.script('INSERT INTO t VALUES (2)'),
            db.script('DELETE FROM t'),
        ]
    ";
    assert_eq!(
        app_value(source, filtered(&seen, refuse_drops)),
        frost("[0, 'no dropping', 1, 1]"),
        "the refused call inserted no row, and the connection works on"
    );

    // A thrown value arrives as that value.
    let thrown = |_: &str| {
        Err(FrostError::from_value(Value::from_iter([(
            MapKey::from("reason"),
            Value::from("refused"),
        )])))
    };
    assert_eq!(
        app_value(
            "try_call(import('app').open_db().script, ['SELECT 1']).error",
            filtered(&Seen::default(), thrown)
        ),
        frost("{reason: 'refused'}")
    );
}

#[test]
fn every_method_running_sql_passes_it_through_the_filter() {
    let seen = Seen::default();
    let source = r"
        def db = import('app').open_db()
        db.script('CREATE TABLE t (x)')
        db.exec('INSERT INTO t VALUES (?)', [1])
        db.query('SELECT x FROM t WHERE x = :x', {x: 1})
        db.fold('SELECT x FROM t', null, fn acc, row -> db.query('SELECT 2'))
        db.last_insert_rowid()
    ";
    assert_eq!(
        app_value(source, filtered(&seen, |_| Ok(()))),
        Value::Int(1)
    );
    assert_eq!(
        seen_sql(&seen),
        [
            "CREATE TABLE t (x)",
            "INSERT INTO t VALUES (?)",
            "SELECT x FROM t WHERE x = :x",
            "SELECT x FROM t",
            // The callback's own call.
            "SELECT 2",
        ],
        "each call's SQL once, placeholders and all"
    );
}

#[test]
fn a_filter_sees_a_transactions_sql_but_not_its_own_statements() {
    let seen = Seen::default();
    let source = r"
        def db = import('app').open_db()
        db.transaction(fn tx -> {
            tx.exec('CREATE TABLE t (x)')
            tx.transaction(fn inner -> inner.query('SELECT x FROM t'))
        })
    ";
    assert_eq!(
        app_value(source, filtered(&seen, |_| Ok(()))),
        frost("{ok: true, value: 0}")
    );
    assert_eq!(
        seen_sql(&seen),
        ["CREATE TABLE t (x)", "SELECT x FROM t"],
        "the script's SQL, and none of the BEGIN, SAVEPOINT, RELEASE, or COMMIT the transaction ran"
    );
}

#[test]
fn a_filter_refuses_sql_before_it_is_bound_or_run() {
    let seen = Seen::default();
    let refuse_all = |_: &str| Err(FrostError::from_static("refused"));
    let source = r"
        def db = import('app').open_db()
        [
            try_call(db.exec, ['INSERT INTO missing VALUES (?)', [[1]]]).error,
            try_call(db.query, ['SELECT :x', {y: 1}]).error,
            try_call(db.fold, ['NOT VALID SQL', null, fn acc, row -> acc]).error,
        ]
    ";
    assert_eq!(
        app_value(source, filtered(&seen, refuse_all)),
        frost("['refused', 'refused', 'refused']"),
        "the filter's error, rather than the binding's or SQLite's"
    );
}

#[test]
fn a_filter_sees_no_call_that_fails_first() {
    let seen = Seen::default();
    let source = r"
        def db = import('app').open_db()
        def fails = fn function, args -> try_call(function, args).ok
        [
            fails(db.script, [1]),
            fails(db.script, ['SELECT 1;\0']),
            fails(db.exec, [1]),
            fails(db.exec, ['SELECT 1', 1]),
            fails(db.exec, ['SELECT 1;\0']),
            fails(db.query, [1]),
            fails(db.query, ['SELECT 1', 1]),
            fails(db.query, ['SELECT 1;\0']),
            fails(db.fold, [1, null, fn acc, row -> acc]),
            fails(db.fold, ['SELECT 1', 1, null, fn acc, row -> acc]),
            fails(db.fold, ['SELECT 1', null, 1]),
            fails(db.fold, ['SELECT 1;\0', null, fn acc, row -> acc]),
            db.close(),
            fails(db.script, ['SELECT 1']),
            fails(db.exec, ['SELECT 1']),
            fails(db.query, ['SELECT 1']),
            fails(db.fold, ['SELECT 1', null, fn acc, row -> acc]),
        ]
    ";
    assert_eq!(
        app_value(source, filtered(&seen, |_| Ok(()))),
        frost(
            "[false, false, false, false, false, false, false, false, false, false, false, false, null, false, false, false, false]"
        )
    );
    assert!(
        seen_sql(&seen).is_empty(),
        "a wrong type, a NUL, and a closed connection each fail before the filter, in every method"
    );
}

#[test]
fn a_filter_and_an_authorizer_both_judge() {
    let factory = connection_factory("app.open_db", |_| {
        Ok(ScriptConnection::from(memory_connection()?)
            .with_authorizer(Authorizer::new().deny_functions(["upper"]))
            .with_sql_filter(|sql| {
                if sql.contains("lower") {
                    Err(FrostError::from_static("no lower"))
                } else {
                    Ok(())
                }
            }))
    });
    let source = r"
        def db = import('app').open_db()
        [
            try_call(db.script, ['SELECT upper(1)']).error,
            try_call(db.script, ['SELECT lower(1)']).error,
            db.script('SELECT abs(1)'),
        ]
    ";
    assert_eq!(
        app_value(source, factory),
        frost("['Function db.script failed: not authorized (SQLITE_AUTH)', 'no lower', 0]")
    );
}
