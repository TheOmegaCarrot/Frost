//! `db.close`, and using a closed connection.

use crate::common::{assert_arity, assert_raises, assert_values};

#[test]
fn close_returns_null() {
    assert_values(&[("sqlite.open_memory().close()", "null")]);
}

#[test]
fn every_call_on_a_closed_connection_is_an_error() {
    let closed = r"
        def db = sqlite.open_memory()
        db.close()
    ";
    for (call, name) in [
        ("db.script('SELECT 1')", "db.script"),
        ("db.exec('SELECT 1')", "db.exec"),
        ("db.query('SELECT 1')", "db.query"),
        ("db.fold('SELECT 1', null, fn acc, row -> acc)", "db.fold"),
        ("db.last_insert_rowid()", "db.last_insert_rowid"),
        ("db.close()", "db.close"),
    ] {
        assert_raises(&[(
            &format!("{closed}\n{call}"),
            &format!("Function {name} requires an open connection, but it is closed"),
        )]);
    }
}

#[test]
fn a_closed_connection_still_checks_its_arguments_first() {
    let closed = r"
        def db = sqlite.open_memory()
        db.close()
    ";
    assert_raises(&[
        (
            &format!("{closed}\ndb.script(1)"),
            "Function db.script requires String as argument 1 (sql), got Int",
        ),
        (
            &format!("{closed}\ndb.script('\\0')"),
            "Function db.script requires SQL without NUL characters",
        ),
        (
            &format!("{closed}\ndb.exec('\\0')"),
            "Function db.exec requires SQL without NUL characters",
        ),
        (
            &format!("{closed}\ndb.query('\\0')"),
            "Function db.query requires SQL without NUL characters",
        ),
        (
            &format!("{closed}\ndb.fold('\\0', null, fn acc, row -> acc)"),
            "Function db.fold requires SQL without NUL characters",
        ),
        (
            &format!("{closed}\ndb.query(1)"),
            "Function db.query requires String as argument 1 (sql), got Int",
        ),
        (
            &format!("{closed}\ndb.last_insert_rowid(1)"),
            "Function db.last_insert_rowid expects 0 arguments, but was called with 1",
        ),
        (
            &format!("{closed}\ndb.exec('SELECT 1', 1)"),
            "Function db.exec requires Structured as argument 2 (bindings), got Int",
        ),
        (
            &format!("{closed}\ndb.fold('SELECT 1', null, 1)"),
            "Function db.fold requires Function as argument 3, got Int",
        ),
    ]);
}

#[test]
fn closing_one_connection_leaves_others_open() {
    let source = r"
        def first = sqlite.open_memory()
        def second = sqlite.open_memory()
        first.close()
        second.script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')
    ";
    assert_values(&[(source, "1")]);
}

#[test]
fn close_checks_its_arity() {
    assert_arity("sqlite.open_memory().close", "db.close", 0, &[1, 2]);
}
