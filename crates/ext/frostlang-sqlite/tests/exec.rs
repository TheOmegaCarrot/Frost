//! `db.exec`: running one statement, and `db.last_insert_rowid`.

use crate::common::{
    NON_STRINGS, assert_arity, assert_raises, assert_values, with_db, with_rows, with_table,
};

#[test]
fn exec_returns_the_rows_its_statement_changed() {
    let cases = [
        ("db.exec('CREATE TABLE u (x)')", "0"),
        ("db.exec('SELECT 1')", "0"),
        (r#"db.exec("INSERT INTO t VALUES (1, 'a')")"#, "1"),
        (
            r#"db.exec("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")"#,
            "3",
        ),
        (
            r#"
            db.exec("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
            db.exec("UPDATE t SET name = 'z' WHERE id >= 2")
            "#,
            "2",
        ),
        (
            r#"
            db.exec("INSERT INTO t VALUES (1, 'a'), (2, 'b')")
            db.exec('DELETE FROM t')
            "#,
            "2",
        ),
        ("db.exec('UPDATE t SET name = 1 WHERE id = 9999')", "0"),
        ("db.exec('DELETE FROM t')", "0"),
        // Every row it returns, it changed.
        (
            r#"db.exec("INSERT INTO t VALUES (1, 'a'), (2, 'b') RETURNING id")"#,
            "2",
        ),
    ];
    for (body, expected) in cases {
        assert_values(&[(&with_table(body), expected)]);
    }
}

#[test]
fn exec_counts_a_replaced_or_ignored_row_as_sqlite_does() {
    let cases = [
        (
            r#"db.exec("INSERT OR REPLACE INTO t VALUES (1, 'z')")"#,
            "1",
        ),
        (r#"db.exec("REPLACE INTO t VALUES (1, 'z')")"#, "1"),
        (
            r#"db.exec("INSERT INTO t VALUES (1, 'z') ON CONFLICT (id) DO UPDATE SET name = excluded.name")"#,
            "1",
        ),
        (
            r#"db.exec("INSERT INTO t VALUES (1, 'z') ON CONFLICT DO NOTHING")"#,
            "0",
        ),
        (r#"db.exec("INSERT OR IGNORE INTO t VALUES (1, 'z')")"#, "0"),
    ];
    for (body, expected) in cases {
        assert_values(&[(&with_rows(body), expected)]);
    }
}

#[test]
fn exec_counts_no_rows_for_a_statement_that_changes_none_after_one_that_did() {
    // SQLite keeps the count of the last INSERT, UPDATE, or DELETE through any
    // other statement.
    for statement in [
        "CREATE TABLE u (x)",
        "CREATE TABLE u AS SELECT * FROM t",
        "SELECT * FROM t",
        "DROP TABLE t",
    ] {
        let body = format!(
            r#"
            db.exec("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
            db.exec('{statement}')
            "#
        );
        assert_values(&[(&with_table(&body), "0")]);
    }
}

#[test]
fn exec_counts_the_rows_dropping_a_referenced_table_deletes() {
    // Dropping a table that a foreign key references first deletes its rows,
    // which SQLite counts as the statement's.
    let source = with_db(
        r"
        db.exec('CREATE TABLE parent (id INTEGER PRIMARY KEY)')
        db.exec('CREATE TABLE child (parent INTEGER REFERENCES parent (id))')
        db.exec('INSERT INTO parent VALUES (1), (2)')
        db.exec('DROP TABLE parent')
        ",
    );
    assert_values(&[(&source, "2")]);
}

#[test]
fn exec_counts_only_the_rows_its_statement_changes_itself() {
    // As SQLite counts them: not the rows of triggers or foreign key actions.
    assert_values(&[
        (
            &with_table(
                r#"
                db.exec('CREATE TABLE log (x)')
                db.exec('CREATE TRIGGER logged AFTER INSERT ON t BEGIN INSERT INTO log VALUES (new.id); INSERT INTO log VALUES (new.id); END')
                db.exec("INSERT INTO t VALUES (1, 'a'), (2, 'b')")
                "#,
            ),
            "2",
        ),
        (
            &with_db(
                r"
                db.script('CREATE TABLE parent (id INTEGER PRIMARY KEY); CREATE TABLE child (parent INTEGER REFERENCES parent (id) ON DELETE CASCADE)')
                db.script('INSERT INTO parent VALUES (1); INSERT INTO child VALUES (1), (1)')
                db.exec('DELETE FROM parent')
                ",
            ),
            "1",
        ),
    ]);
}

#[test]
fn exec_changes_the_database() {
    let source = with_table(
        r#"
        db.exec("INSERT INTO t VALUES (1, 'a')")
        db.exec('INSERT INTO t VALUES (?, ?)', [2, 'b'])
        db.query('SELECT * FROM t ORDER BY id')
        "#,
    );
    assert_values(&[(&source, "[{id: 1, name: 'a'}, {id: 2, name: 'b'}]")]);
}

#[test]
fn exec_runs_a_statement_returning_rows_in_full() {
    // Only the second row overflows, so stepping to the first alone would
    // succeed.
    assert_raises(&[(
        &with_db(
            "db.exec('SELECT CASE WHEN v = 2 THEN abs(-9223372036854775807 - 1) ELSE 1 END FROM (SELECT 1 AS v UNION ALL SELECT 2)')",
        ),
        "Function db.exec failed: integer overflow (SQLITE_ERROR)",
    )]);
}

#[test]
fn exec_reports_sqlite_errors() {
    assert_raises(&[
        (
            &with_db("db.exec('NOT VALID SQL')"),
            r#"Function db.exec failed: near "NOT": syntax error (SQLITE_ERROR)"#,
        ),
        (
            &with_db("db.exec('INSERT INTO missing VALUES (1)')"),
            "Function db.exec failed: no such table: missing (SQLITE_ERROR)",
        ),
        (
            &with_table(
                r#"
                db.exec("INSERT INTO t VALUES (1, 'a')")
                db.exec("INSERT INTO t VALUES (1, 'b')")
                "#,
            ),
            "Function db.exec failed: UNIQUE constraint failed: t.id (SQLITE_CONSTRAINT)",
        ),
    ]);
}

#[test]
fn a_connection_works_on_after_exec_fails() {
    let source = with_table(
        r#"
        try_call(db.exec, ['NOT VALID SQL'])
        try_call(db.exec, ['INSERT INTO t VALUES (?, ?)', [1]])
        try_call(db.exec, ['SELECT 1; SELECT 2'])
        db.exec("INSERT INTO t VALUES (1, 'a')")
        "#,
    );
    assert_values(&[(&source, "1")]);
}

#[test]
fn exec_runs_exactly_one_statement() {
    // Comments and semicolons may surround the statement.
    let source = with_table(
        r#"
        [
            db.exec("INSERT INTO t VALUES (1, 'a');"),
            db.exec("INSERT INTO t VALUES (2, 'b') ;  ; "),
            db.exec("INSERT INTO t VALUES (3, 'c'); -- the end"),
            db.exec("/* the start */ INSERT INTO t VALUES (4, 'd') /* the end */"),
            db.exec("; -- the start\n INSERT INTO t VALUES (5, 'e')"),
        ]
        "#,
    );
    assert_values(&[(&source, "[1, 1, 1, 1, 1]")]);

    let none = "Function db.exec requires SQL of exactly one statement, but it has none";
    let more = "Function db.exec requires SQL of exactly one statement, but it has more than one";
    assert_raises(&[
        (&with_db("db.exec('')"), none),
        (&with_db("db.exec('  ;  ; ')"), none),
        (&with_db("db.exec('-- nothing but a comment')"), none),
        (&with_db("db.exec('SELECT 1; SELECT 2')"), more),
        (&with_db("db.exec('SELECT 1; SELECT 2;')"), more),
        (
            &with_db(r"db.exec('SELECT 1; -- a comment\n SELECT 2')"),
            more,
        ),
        // A statement after the first is one, though it would not prepare:
        // on its own, or before the first has run.
        (&with_db("db.exec('SELECT 1; garbage')"), more),
        (&with_db("db.exec('SELECT 1; SELECT * FROM missing')"), more),
        (
            &with_db("db.exec('CREATE TABLE v (x); INSERT INTO v VALUES (1)')"),
            more,
        ),
    ]);
}

#[test]
fn exec_runs_nothing_from_sql_of_more_than_one_statement() {
    let source = with_table(
        r#"
        try_call(db.exec, ["INSERT INTO t VALUES (1, 'a'); INSERT INTO t VALUES (2, 'b')"])
        db.query('SELECT * FROM t')
        "#,
    );
    assert_values(&[(&source, "[]")]);
}

#[test]
fn exec_rejects_sql_containing_nul() {
    assert_raises(&[(
        &with_db(r"db.exec('SELECT 1;\0SELECT 2')"),
        "Function db.exec requires SQL without NUL characters",
    )]);
}

#[test]
fn exec_checks_its_arity() {
    assert_arity(
        "sqlite.open_memory().exec",
        "db.exec",
        "between 1 and 2",
        &[0, 3],
    );
}

#[test]
fn exec_checks_its_argument_types() {
    for (argument, kind) in NON_STRINGS {
        assert_raises(&[(
            &with_db(&format!("db.exec({argument})")),
            &format!("Function db.exec requires String as argument 1 (sql), got {kind}"),
        )]);
    }
}

#[test]
fn last_insert_rowid_is_the_rowid_last_inserted() {
    let source = with_table(
        r#"
        def before = db.last_insert_rowid()
        db.exec("INSERT INTO t (name) VALUES ('a')")
        def first = db.last_insert_rowid()
        db.exec("INSERT INTO t (name) VALUES ('b')")
        def second = db.last_insert_rowid()
        db.exec("INSERT INTO t VALUES (100, 'c')")
        def explicit = db.last_insert_rowid()
        # Neither a failed or ignored insert nor any other statement changes
        # it.
        try_call(db.exec, ["INSERT INTO t VALUES (1, 'duplicate')"])
        db.exec("INSERT OR IGNORE INTO t VALUES (1, 'duplicate')")
        db.exec("UPDATE t SET name = 'z'")
        db.exec('CREATE TABLE u (x)')
        [before, first, second, explicit, db.last_insert_rowid()]
        "#,
    );
    assert_values(&[(&source, "[0, 1, 2, 100, 100]")]);
}

#[test]
fn last_insert_rowid_counts_inserts_by_any_method() {
    let source = with_table(
        r#"
        db.script("INSERT INTO t VALUES (5, 'a')")
        def by_script = db.last_insert_rowid()
        db.query("INSERT INTO t VALUES (6, 'b') RETURNING id")
        def by_query = db.last_insert_rowid()
        db.reduce("INSERT INTO t VALUES (7, 'c') RETURNING id", null, fn acc, row -> acc)
        def by_reduce = db.last_insert_rowid()
        [by_script, by_query, by_reduce]
        "#,
    );
    assert_values(&[(&source, "[5, 6, 7]")]);
}

#[test]
fn last_insert_rowid_is_each_connections_own() {
    let source = r"
        def first = sqlite.open_memory()
        def second = sqlite.open_memory()
        first.exec('CREATE TABLE t (x)')
        first.exec('INSERT INTO t VALUES (1)')
        [first.last_insert_rowid(), second.last_insert_rowid()]
    ";
    assert_values(&[(source, "[1, 0]")]);
}

#[test]
fn last_insert_rowid_checks_its_arity() {
    assert_arity(
        "sqlite.open_memory().last_insert_rowid",
        "db.last_insert_rowid",
        0,
        &[1, 2],
    );
}
