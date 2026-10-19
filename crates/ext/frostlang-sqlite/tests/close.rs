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
    assert_raises(&[
        (
            &format!("{closed}\ndb.script('SELECT 1')"),
            "Function db.script requires an open connection, but it is closed",
        ),
        (
            &format!("{closed}\ndb.close()"),
            "Function db.close requires an open connection, but it is closed",
        ),
    ]);
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
