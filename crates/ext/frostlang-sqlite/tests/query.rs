//! `db.query`: reading every row of one statement.

use crate::common::{
    NON_STRINGS, assert_arity, assert_raises, assert_values, value, with_db, with_rows,
};

#[test]
fn query_returns_every_row_in_order() {
    assert_values(&[
        (
            &with_rows("db.query('SELECT * FROM t ORDER BY id')"),
            "[{id: 1, name: 'a'}, {id: 2, name: 'b'}, {id: 3, name: 'c'}]",
        ),
        (
            &with_rows("db.query('SELECT name FROM t ORDER BY id DESC')"),
            "[{name: 'c'}, {name: 'b'}, {name: 'a'}]",
        ),
        (&with_rows("db.query('SELECT * FROM t WHERE id > 9')"), "[]"),
        (&with_db("db.query('SELECT 1 AS one')"), "[{one: 1}]"),
    ]);
}

#[test]
fn a_row_is_keyed_by_its_column_names() {
    assert_values(&[
        // A column takes its name from `AS`, else from SQLite.
        (
            &with_rows("db.query('SELECT id AS key, upper(name), count(*) FROM t WHERE id = 1')"),
            r#"[{key: 1, ["upper(name)"]: 'A', ["count(*)"]: 1}]"#,
        ),
        (
            &with_db(r#"db.query('SELECT 1 AS "two words", 2 AS "", 3 AS "x\'y"')"#),
            r#"[{["two words"]: 1, [""]: 2, ["x'y"]: 3}]"#,
        ),
        // Names differing only in case are different names.
        (
            &with_db("db.query('SELECT 1 AS x, 2 AS X')"),
            "[{x: 1, X: 2}]",
        ),
        (
            &with_db(r#"db.query('SELECT 1 AS "\u{2744}"')"#),
            r#"[{["\u{2744}"]: 1}]"#,
        ),
        // A statement with no columns yields no rows to key.
        (&with_db("db.query('CREATE TABLE u (x)')"), "[]"),
    ]);
}

#[test]
fn of_columns_sharing_a_name_the_last_stands() {
    assert_values(&[
        (
            &with_db("db.query('SELECT 1 AS x, 2 AS y, 3 AS x')"),
            "[{x: 3, y: 2}]",
        ),
        (
            &with_rows(
                r"
                db.exec('CREATE TABLE u (id INTEGER PRIMARY KEY, name TEXT)')
                db.exec('INSERT INTO u VALUES (1, NULL)')
                db.query('SELECT * FROM t JOIN u USING (id)')
                ",
            ),
            // `u.name` is the last `name`, though NULL.
            "[{id: 1, name: null}]",
        ),
    ]);
}

#[test]
fn query_runs_a_statement_that_changes_rows() {
    assert_values(&[(
        &with_rows(
            r"
            def returned = db.query('UPDATE t SET name = upper(name) WHERE id < 3 RETURNING id')
            [returned, db.query('SELECT name FROM t ORDER BY id')]
            ",
        ),
        "[[{id: 1}, {id: 2}], [{name: 'A'}, {name: 'B'}, {name: 'c'}]]",
    )]);
}

#[test]
fn query_reports_sqlite_errors() {
    assert_raises(&[
        (
            &with_db("db.query('NOT VALID SQL')"),
            r#"Function db.query failed: near "NOT": syntax error (SQLITE_ERROR)"#,
        ),
        (
            &with_db("db.query('SELECT * FROM missing')"),
            "Function db.query failed: no such table: missing (SQLITE_ERROR)",
        ),
        // A failure on a later row fails the call, returning no rows.
        (
            &with_db(
                "db.query('SELECT CASE WHEN v = 2 THEN abs(-9223372036854775807 - 1) ELSE 1 END AS x FROM (SELECT 1 AS v UNION ALL SELECT 2)')",
            ),
            "Function db.query failed: integer overflow (SQLITE_ERROR)",
        ),
    ]);
}

#[test]
fn a_failing_query_leaves_the_changes_its_statement_made() {
    // SQLite makes every change as the statement begins, before any row is
    // read, and the statement commits on its own.
    let source = with_rows(
        r#"
        db.exec("UPDATE t SET name = CAST(x'ff' AS TEXT) WHERE id = 2")
        def deleted = try_call(db.query, ['DELETE FROM t RETURNING id, name'])
        [deleted.ok, db.query('SELECT count(*) AS c FROM t')]
        "#,
    );
    assert_values(&[(&source, "[false, [{c: 0}]]")]);
}

#[test]
fn query_runs_exactly_one_statement() {
    assert_values(&[(
        &with_db("db.query('SELECT 1 AS x; -- the end')"),
        "[{x: 1}]",
    )]);
    assert_raises(&[
        (
            &with_db("db.query('')"),
            "Function db.query requires SQL of exactly one statement, but it has none",
        ),
        (
            &with_db("db.query('SELECT 1; SELECT 2')"),
            "Function db.query requires SQL of exactly one statement, but it has more than one",
        ),
        (
            &with_db("db.query('SELECT 1; garbage')"),
            "Function db.query requires SQL of exactly one statement, but it has more than one",
        ),
        (
            &with_db(r"db.query('SELECT 1;\0')"),
            "Function db.query requires SQL without NUL characters",
        ),
    ]);
}

#[test]
fn query_holds_many_rows() {
    let rows = value(&with_db(
        r"
        db.query('WITH RECURSIVE n (i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 10000) SELECT i FROM n')
        ",
    ));
    let frostlang::Value::Array(rows) = rows else {
        panic!("query returns an Array, not {rows:?}");
    };
    assert_eq!(rows.len(), 10_000);
    assert_eq!(
        rows.as_slice().last(),
        Some(&value("{i: 10000}")),
        "the last row is the last read"
    );
}

#[test]
fn query_checks_its_arity() {
    assert_arity(
        "sqlite.open_memory().query",
        "db.query",
        "between 1 and 2",
        &[0, 3],
    );
}

#[test]
fn query_checks_its_argument_types() {
    for (argument, kind) in NON_STRINGS {
        assert_raises(&[(
            &with_db(&format!("db.query({argument})")),
            &format!("Function db.query requires String as argument 1 (sql), got {kind}"),
        )]);
    }
}
