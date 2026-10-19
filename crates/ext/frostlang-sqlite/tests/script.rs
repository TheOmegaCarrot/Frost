//! `db.script`: running statements without bindings.

use crate::common::{NON_STRINGS, assert_arity, assert_raises, assert_values};

/// `body`, run with `db` bound to a new in-memory connection.
fn with_db(body: &str) -> String {
    format!(
        r"
        def db = sqlite.open_memory()
        {body}
        "
    )
}

#[test]
fn script_returns_the_rows_its_statements_changed() {
    let cases = [
        // Statements that change no rows.
        ("db.script('CREATE TABLE t (x); CREATE TABLE u (x)')", "0"),
        ("db.script('SELECT 1')", "0"),
        ("db.script('')", "0"),
        ("db.script('  ;  ; ')", "0"),
        ("db.script('-- nothing but a comment')", "0"),
        // Statements that do.
        (
            "db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2); INSERT INTO t VALUES (3)')",
            "3",
        ),
        (
            "db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')",
            "1",
        ),
        (
            "db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1) ;  ;  ')",
            "1",
        ),
        (
            "db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1), (2), (3); UPDATE t SET x = 0 WHERE x > 1')",
            "5",
        ),
        (
            "db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1), (2); DELETE FROM t')",
            "4",
        ),
        (
            "db.script('CREATE TABLE t (x); UPDATE t SET x = 1 WHERE x = 9999')",
            "0",
        ),
        // Comments around and between statements.
        (
            "db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1); -- the end')",
            "1",
        ),
        (
            "db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1) /* the end */')",
            "1",
        ),
        (
            "db.script('CREATE TABLE t (x); /* between */ INSERT INTO t VALUES (1)')",
            "1",
        ),
        // A statement returning rows, every one of which it changed.
        (
            "db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1), (2), (3) RETURNING x')",
            "3",
        ),
    ];
    for (expression, expected) in cases {
        assert_values(&[(&with_db(expression), expected)]);
    }
}

#[test]
fn script_counts_only_its_own_changes() {
    assert_values(&[(
        &with_db(
            r"
            db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1), (2)')
            db.script('INSERT INTO t VALUES (3)')
            ",
        ),
        "1",
    )]);
}

#[test]
fn script_counts_the_rows_triggers_and_foreign_key_actions_change() {
    assert_values(&[
        (
            &with_db(
                r"
                db.script('CREATE TABLE t (x); CREATE TABLE log (x); CREATE TRIGGER logged AFTER INSERT ON t BEGIN INSERT INTO log VALUES (new.x); END')
                db.script('INSERT INTO t VALUES (1), (2)')
                ",
            ),
            "4",
        ),
        (
            &with_db(
                r"
                db.script('CREATE TABLE parent (id INTEGER PRIMARY KEY); CREATE TABLE child (parent INTEGER REFERENCES parent (id) ON DELETE CASCADE)')
                db.script('INSERT INTO parent VALUES (1); INSERT INTO child VALUES (1), (1)')
                db.script('DELETE FROM parent')
                ",
            ),
            "3",
        ),
    ]);
}

#[test]
fn script_runs_a_statement_returning_rows_in_full() {
    // Only the second row overflows, so stepping to the first alone would
    // succeed.
    assert_raises(&[(
        &with_db(
            "db.script('SELECT CASE WHEN v = 2 THEN abs(-9223372036854775807 - 1) ELSE 1 END FROM (SELECT 1 AS v UNION ALL SELECT 2)')",
        ),
        "Function db.script failed: integer overflow (SQLITE_ERROR)",
    )]);
}

#[test]
fn script_reports_sqlite_errors() {
    assert_raises(&[
        (
            &with_db("db.script('NOT VALID SQL')"),
            // As SQLite words it, without the SQL repeated.
            r#"Function db.script failed: near "NOT": syntax error (SQLITE_ERROR)"#,
        ),
        (
            &with_db("db.script('INSERT INTO missing VALUES (1)')"),
            "Function db.script failed: no such table: missing (SQLITE_ERROR)",
        ),
        (
            &with_db(
                "db.script('CREATE TABLE t (id INTEGER PRIMARY KEY); INSERT INTO t VALUES (1); INSERT INTO t VALUES (1)')",
            ),
            "Function db.script failed: UNIQUE constraint failed: t.id (SQLITE_CONSTRAINT)",
        ),
    ]);
}

#[test]
fn script_leaves_the_statements_before_a_failure_committed() {
    let source = with_db(
        r"
        db.script('CREATE TABLE t (id INTEGER PRIMARY KEY)')
        try_call(db.script, ['INSERT INTO t VALUES (1); INSERT INTO t VALUES (2); INSERT INTO t VALUES (1); INSERT INTO t VALUES (3)'])
        # The two rows before the failure, and none after it.
        db.script('DELETE FROM t')
        ",
    );
    assert_values(&[(&source, "2")]);
}

#[test]
fn a_connection_works_on_after_a_failure() {
    let source = with_db(
        r"
        try_call(db.script, ['NOT VALID SQL'])
        try_call(db.script, ['INSERT INTO missing VALUES (1)'])
        db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')
        ",
    );
    assert_values(&[(&source, "1")]);
}

#[test]
fn script_rejects_sql_containing_nul() {
    // SQLite would take the NUL as the end of the SQL, and silently skip the
    // statement after it.
    let source = with_db(
        r"
        db.script('CREATE TABLE t (x)')
        db.script('INSERT INTO t VALUES (1);\0INSERT INTO t VALUES (2)')
        ",
    );
    assert_raises(&[(
        &source,
        "Function db.script requires SQL without NUL characters",
    )]);
    // Nothing ran.
    let source = with_db(
        r"
        db.script('CREATE TABLE t (id INTEGER PRIMARY KEY)')
        try_call(db.script, ['INSERT INTO t VALUES (1);\0'])
        db.script('INSERT INTO t VALUES (1)')
        ",
    );
    assert_values(&[(&source, "1")]);
}

#[test]
fn script_checks_its_arity() {
    assert_arity("sqlite.open_memory().script", "db.script", 1, &[0, 2]);
}

#[test]
fn script_checks_its_argument_type() {
    for (argument, kind) in NON_STRINGS {
        assert_raises(&[(
            &with_db(&format!("db.script({argument})")),
            &format!("Function db.script requires String as argument 1 (sql), got {kind}"),
        )]);
    }
}
