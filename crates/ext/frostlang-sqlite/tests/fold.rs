//! `db.fold`: folding the rows of one statement as they are read, with a
//! callback that may use the connection meanwhile.

use std::sync::{Arc, Mutex};

use frostlang::{Value, frost_closure};
use frostlang_sqlite::rusqlite::functions::FunctionFlags;
use frostlang_sqlite::{ScriptConnection, connection_factory, sqlite_error};

use crate::common::{
    NON_STRINGS, assert_arity, assert_raises, assert_values, frost, memory_connection, run_app,
    with_db, with_rows,
};

#[test]
fn fold_folds_the_rows_in_order() {
    assert_values(&[
        (
            &with_rows("db.fold('SELECT id FROM t', 0, fn total, row -> total + row.id)"),
            "6",
        ),
        (
            &with_rows(
                "db.fold('SELECT * FROM t ORDER BY id DESC', [], fn names, row -> names + [row.name])",
            ),
            "['c', 'b', 'a']",
        ),
        // The accumulator comes first, then the row.
        (
            &with_rows(
                "db.fold('SELECT id FROM t ORDER BY id', 'start', fn acc, row -> [acc, row])",
            ),
            "[[['start', {id: 1}], {id: 2}], {id: 3}]",
        ),
        // With bindings.
        (
            &with_rows(
                "db.fold('SELECT id FROM t WHERE id > ?', [1], 0, fn total, row -> total + row.id)",
            ),
            "5",
        ),
        (
            &with_rows(
                "db.fold('SELECT id FROM t WHERE id < :below', {below: 3}, 0, fn total, row -> total + row.id)",
            ),
            "3",
        ),
    ]);
}

#[test]
fn fold_returns_its_initial_value_for_no_rows() {
    assert_values(&[
        (
            &with_rows(
                "db.fold('SELECT * FROM t WHERE id > 9', 'initial', fn acc, row -> assert(false))",
            ),
            "'initial'",
        ),
        (
            &with_db("db.fold('CREATE TABLE u (x)', null, fn acc, row -> assert(false))"),
            "null",
        ),
        // The statement still runs.
        (
            &with_db(
                r"
                db.fold('CREATE TABLE u (x)', null, fn acc, row -> acc)
                db.query('SELECT name FROM sqlite_schema')
                ",
            ),
            "[{name: 'u'}]",
        ),
    ]);
}

#[test]
fn fold_takes_any_initial_value() {
    for initial in [
        "null", "true", "1", "1.5", "'text'", "x'00'", "[1]", "{a: 1}",
    ] {
        assert_values(&[(
            &with_rows(&format!(
                "db.fold('SELECT id FROM t WHERE id = 1', {initial}, fn acc, row -> [acc])"
            )),
            &format!("[{initial}]"),
        )]);
    }
    // A Function, the very one given.
    let source = with_rows(
        r"
        def initial = fn -> 1
        db.fold('SELECT id FROM t', initial, fn acc, row -> acc) == initial
        ",
    );
    assert_values(&[(&source, "true")]);
}

#[test]
fn three_arguments_are_sql_initial_and_function() {
    // An Array in the middle is the initial value, not bindings.
    assert_values(&[(
        &with_rows("db.fold('SELECT id FROM t WHERE id = 1', [1], fn acc, row -> acc + [row.id])"),
        "[1, 1]",
    )]);
    assert_raises(&[(
        &with_db("db.fold('SELECT ?', [1], fn acc, row -> acc)"),
        "Function db.fold requires 1 binding, but none were given",
    )]);
}

/// The Frost function `mark(array)`, which returns `array` with one more
/// element: whether the array was held by nothing else, so that it could be
/// updated in place.
fn mark() -> Value {
    frost_closure!("mark", |array: Value| match array {
        Value::Array(array) => {
            let (mut elements, unshared) = match array.try_into_vec() {
                Ok(elements) => (elements, true),
                Err(shared) => (shared.into_vec(), false),
            };
            elements.push(Value::Bool(unshared));
            Value::from(elements)
        }
        other => panic!("`mark` takes an Array, not {other:?}"),
    })
}

/// The Frost function `fresh()`, which returns a new Array, `['start']`, that
/// nothing else holds. A script's own literal may be a constant the program
/// holds too.
fn fresh() -> Value {
    frost_closure!("fresh", || Value::from(vec![Value::from("start")]))
}

/// The value of `source`, run with `app.open_db`, a factory of in-memory
/// connections, and `app.mark` and `app.fresh`, from [`mark`] and [`fresh`].
fn with_mark(source: &str) -> Value {
    let factory = connection_factory("app.open_db", |_| {
        Ok(ScriptConnection::from(memory_connection()?))
    });
    run_app(
        source,
        vec![("open_db", factory), ("mark", mark()), ("fresh", fresh())],
    )
    .unwrap_or_else(|error| panic!("{source:?} should run, but raised: {error}"))
}

#[test]
fn fold_hands_on_the_accumulator_so_it_can_be_updated_in_place() {
    let source = r"
        def app = import('app')
        def db = app.open_db()
        db.fold('SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3', app.fresh(), fn acc, row -> app.mark(acc))
    ";
    assert_eq!(
        with_mark(source),
        frost("['start', true, true, true]"),
        "every row's call got the only reference"
    );

    // The probe tells: a script still holding the initial value shares it.
    let source = r"
        def app = import('app')
        def db = app.open_db()
        def initial = app.fresh()
        [db.fold('SELECT 1 UNION ALL SELECT 2', initial, fn acc, row -> app.mark(acc)), initial]
    ";
    assert_eq!(
        with_mark(source),
        frost("[['start', false, true], ['start']]"),
        "the first call shared the initial value; the next got its own"
    );
}

#[test]
fn fold_reads_each_row_only_as_its_callback_takes_it() {
    // SQLite calls the host's `tick()` as it reads each row, and the script
    // calls `app.note()` from its callback: each logs to `events`.
    let events = Arc::new(Mutex::new(Vec::new()));
    let reads = Arc::clone(&events);
    let factory = connection_factory("app.open_db", move |_| {
        let connection = memory_connection()?;
        let reads = Arc::clone(&reads);
        connection
            .create_scalar_function("tick", 0, FunctionFlags::SQLITE_UTF8, move |_| {
                reads.lock().expect("no test panicked").push("read");
                Ok(1)
            })
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        Ok(ScriptConnection::new(connection))
    });
    let calls = Arc::clone(&events);
    let note = frost_closure!("note", move || {
        calls.lock().expect("no test panicked").push("call");
    });
    let source = r"
        def app = import('app')
        def db = app.open_db()
        db.fold('SELECT tick() FROM (SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3)', null, fn acc, row -> app.note())
    ";
    run_app(source, vec![("open_db", factory), ("note", note)])
        .unwrap_or_else(|error| panic!("the fold should run, but raised: {error}"));
    assert_eq!(
        *events.lock().expect("no test panicked"),
        ["read", "call", "read", "call", "read", "call"],
        "each row read just before its call"
    );
}

#[test]
fn a_callback_may_use_the_connection() {
    assert_values(&[
        // Querying.
        (
            &with_rows(
                r"
                db.fold('SELECT id FROM t ORDER BY id', [], fn counts, row -> {
                    def [count] = db.query('SELECT count(*) AS c FROM t WHERE id >= ?', [row.id])
                    counts + [count.c]
                })
                ",
            ),
            "[3, 2, 1]",
        ),
        // Changing another table.
        (
            &with_rows(
                r"
                db.exec('CREATE TABLE log (id)')
                db.fold('SELECT id FROM t', null, fn acc, row -> {
                    db.exec('INSERT INTO log VALUES (?)', [row.id * 10])
                })
                db.query('SELECT id FROM log ORDER BY id')
                ",
            ),
            "[{id: 10}, {id: 20}, {id: 30}]",
        ),
        // Folding again, on the same table.
        (
            &with_rows(
                r"
                db.fold('SELECT id FROM t WHERE id < 3 ORDER BY id', [], fn pairs, outer -> {
                    db.fold('SELECT id FROM t WHERE id > ? ORDER BY id', [outer.id], pairs, fn pairs, inner -> {
                        pairs + [[outer.id, inner.id]]
                    })
                })
                ",
            ),
            "[[1, 2], [1, 3], [2, 3]]",
        ),
        // Running SQL of several statements.
        (
            &with_rows(
                r"
                db.fold('SELECT id FROM t', 0, fn total, row -> {
                    total + db.script('CREATE TABLE IF NOT EXISTS u (x); INSERT INTO u VALUES (1)')
                })
                ",
            ),
            "3",
        ),
        // Reading the latest rowid.
        (
            &with_rows(
                r"
                db.exec('CREATE TABLE u (id INTEGER PRIMARY KEY)')
                db.fold('SELECT id FROM t ORDER BY id', [], fn ids, row -> {
                    db.exec('INSERT INTO u VALUES (?)', [row.id * 10])
                    ids + [db.last_insert_rowid()]
                })
                ",
            ),
            "[10, 20, 30]",
        ),
        // Counting its own changes, apart from the statement being folded.
        (
            &with_rows(
                r"
                db.fold('UPDATE t SET name = upper(name) RETURNING id', [], fn counts, row -> {
                    counts + [db.exec('SELECT 1')]
                })
                ",
            ),
            "[0, 0, 0]",
        ),
    ]);
}

#[test]
fn a_callback_cannot_drop_the_table_being_read() {
    assert_raises(&[(
        &with_rows("db.fold('SELECT id FROM t', null, fn acc, row -> db.exec('DROP TABLE t'))"),
        "Function db.exec failed: database table is locked (SQLITE_LOCKED)",
    )]);
}

#[test]
fn a_callbacks_error_is_the_calls_error_unchanged() {
    assert_raises(&[
        (
            &with_rows(
                "db.fold('SELECT id FROM t', null, fn acc, row -> assert(false, 'refused'))",
            ),
            "Failed assertion: refused",
        ),
        // A failing call the callback makes keeps its own wording.
        (
            &with_rows(
                "db.fold('SELECT id FROM t', null, fn acc, row -> db.exec('NOT VALID SQL'))",
            ),
            r#"Function db.exec failed: near "NOT": syntax error (SQLITE_ERROR)"#,
        ),
        // A callback taking the wrong number of arguments.
        (
            &with_rows("db.fold('SELECT id FROM t', null, fn row -> row)"),
            "Function <lambda> expects 1 arguments, but was called with 2",
        ),
    ]);
    // A thrown value arrives as that value.
    assert_values(&[(
        &with_rows(
            "try_call(db.fold, ['SELECT id FROM t', null, fn acc, row -> assert(false, {reason: 'refused', id: row.id})]).error",
        ),
        "{reason: 'refused', id: 1}",
    )]);
}

#[test]
fn a_callbacks_error_ends_the_statement() {
    let source = with_rows(
        r"
        db.exec('CREATE TABLE log (id)')
        try_call(db.fold, ['SELECT id FROM t ORDER BY id', null, fn acc, row -> {
            db.exec('INSERT INTO log VALUES (?)', [row.id])
            if row.id == 2: assert(false)
        }])
        # A live statement would keep its table from being dropped.
        db.exec('DROP TABLE t')
        db.query('SELECT id FROM log ORDER BY id')
        ",
    );
    assert_values(&[(&source, "[{id: 1}, {id: 2}]")]);
}

#[test]
fn a_failing_fold_leaves_the_changes_its_statement_made() {
    // SQLite makes every change as the statement begins, before any row
    // reaches the callback, and the statement commits on its own.
    let source = with_rows(
        r"
        def deleted = try_call(db.fold, ['DELETE FROM t RETURNING id', null, fn acc, row -> assert(false)])
        [deleted.ok, db.query('SELECT count(*) AS c FROM t')]
        ",
    );
    assert_values(&[(&source, "[false, [{c: 0}]]")]);
}

#[test]
fn a_callback_cannot_close_the_connection() {
    assert_raises(&[
        (
            &with_rows("db.fold('SELECT id FROM t', null, fn acc, row -> db.close())"),
            "Function db.close cannot close the connection while it is in use",
        ),
        // Nor from a fold within a fold.
        (
            &with_rows(
                r"
                db.fold('SELECT id FROM t', null, fn acc, row -> {
                    db.fold('SELECT 1', null, fn inner, inner_row -> db.close())
                })
                ",
            ),
            "Function db.close cannot close the connection while it is in use",
        ),
    ]);
    // Caught, the failure leaves the fold and the connection running.
    let source = with_rows(
        r"
        def closes = db.fold('SELECT id FROM t', [], fn acc, row -> acc + [try_call(db.close).ok])
        [closes, db.query('SELECT count(*) AS c FROM t')]
        ",
    );
    assert_values(&[(&source, "[[false, false, false], [{c: 3}]]")]);
}

#[test]
fn a_fold_once_ended_leaves_the_connection_free_to_close() {
    assert_values(&[
        (
            &with_rows(
                r"
                db.fold('SELECT id FROM t', null, fn acc, row -> acc)
                db.close()
                ",
            ),
            "null",
        ),
        (
            &with_rows(
                r"
                try_call(db.fold, ['SELECT id FROM t', null, fn acc, row -> assert(false)])
                db.close()
                ",
            ),
            "null",
        ),
    ]);
}

#[test]
fn fold_reports_sqlite_errors() {
    assert_raises(&[
        (
            &with_db("db.fold('NOT VALID SQL', null, fn acc, row -> acc)"),
            r#"Function db.fold failed: near "NOT": syntax error (SQLITE_ERROR)"#,
        ),
        (
            &with_db(
                "db.fold('SELECT CASE WHEN v = 2 THEN abs(-9223372036854775807 - 1) ELSE 1 END AS x FROM (SELECT 1 AS v UNION ALL SELECT 2)', 0, fn acc, row -> acc + 1)",
            ),
            "Function db.fold failed: integer overflow (SQLITE_ERROR)",
        ),
    ]);
}

#[test]
fn fold_runs_exactly_one_statement() {
    assert_raises(&[
        (
            &with_db("db.fold('', null, fn acc, row -> acc)"),
            "Function db.fold requires SQL of exactly one statement, but it has none",
        ),
        (
            &with_db("db.fold('SELECT 1; SELECT 2', null, fn acc, row -> acc)"),
            "Function db.fold requires SQL of exactly one statement, but it has more than one",
        ),
        (
            &with_db("db.fold('SELECT 1; garbage', null, fn acc, row -> acc)"),
            "Function db.fold requires SQL of exactly one statement, but it has more than one",
        ),
        (
            &with_db(r"db.fold('SELECT 1;\0', null, fn acc, row -> acc)"),
            "Function db.fold requires SQL without NUL characters",
        ),
    ]);
}

#[test]
fn fold_streams_many_rows() {
    let source = with_db(
        r"
        db.fold('WITH RECURSIVE n (i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 10000) SELECT i FROM n', 0, fn total, row -> total + row.i)
        ",
    );
    assert_values(&[(&source, "50005000")]);
}

#[test]
fn fold_checks_its_arity() {
    assert_arity(
        "sqlite.open_memory().fold",
        "db.fold",
        "between 3 and 4",
        &[0, 1, 2, 5],
    );
}

#[test]
fn fold_checks_its_argument_types() {
    for (argument, kind) in NON_STRINGS {
        assert_raises(&[
            (
                &with_db(&format!("db.fold({argument}, null, fn acc, row -> acc)")),
                &format!("Function db.fold requires String as argument 1 (sql), got {kind}"),
            ),
            (
                &with_db(&format!(
                    "db.fold({argument}, [], null, fn acc, row -> acc)"
                )),
                &format!("Function db.fold requires String as argument 1 (sql), got {kind}"),
            ),
        ]);
    }
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
        assert_raises(&[
            (
                &with_db(&format!("db.fold('SELECT 1', null, {argument})")),
                &format!("Function db.fold requires Function as argument 3, got {kind}"),
            ),
            (
                &with_db(&format!("db.fold('SELECT 1', [], null, {argument})")),
                &format!("Function db.fold requires Function as argument 4, got {kind}"),
            ),
        ]);
    }
}
