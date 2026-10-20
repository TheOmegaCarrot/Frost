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
fn a_filter_sees_no_call_that_fails_first() {
    let seen = Seen::default();
    let source = r"
        def db = import('app').open_db()
        [
            try_call(db.script, [1]).ok,
            try_call(db.script, ['SELECT 1;\0']).ok,
            db.close(),
            try_call(db.script, ['SELECT 1']).ok,
        ]
    ";
    assert_eq!(
        app_value(source, filtered(&seen, |_| Ok(()))),
        frost("[false, false, null, false]")
    );
    assert!(
        seen_sql(&seen).is_empty(),
        "the wrong type, a NUL, and a closed connection each fail before the filter"
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
