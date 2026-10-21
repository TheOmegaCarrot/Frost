//! Transaction statements, such as `BEGIN`, which only the transaction methods
//! run: SQL holding one is an error in every other method.

use crate::common::{assert_raises, assert_values, string_literal, with_db, with_table};

/// What a transaction statement's error says to do instead, from `db`.
const DB_HINT: &str = "; use db.transaction";

/// What a transaction statement's error says to do instead, from a `tx`.
const TX_HINT: &str =
    "; tx.transaction makes savepoints, and a transaction ends as its function returns";

/// Transaction statements, in the forms SQLite reads.
const STATEMENTS: &[&str] = &[
    "BEGIN",
    "begin",
    "BEGIN IMMEDIATE TRANSACTION",
    "COMMIT",
    "END",
    "END TRANSACTION",
    "ROLLBACK",
    "ROLLBACK TO s",
    "ROLLBACK TRANSACTION TO SAVEPOINT s",
    "SAVEPOINT s",
    "RELEASE s",
    "RELEASE SAVEPOINT s",
    "BEGIN--x",
    "END;;",
    // After what SQLite allows before a statement.
    "  ;; /* first */ -- then\n BeGiN",
    "/**/BEGIN",
    "\tBEGIN",
    "\rCOMMIT",
    "\x0cEND",
    // SQLite takes a vertical tab as whitespace after other whitespace.
    " \x0bBEGIN",
    "-- c\n\x0bCOMMIT",
];

#[test]
fn transaction_statements_are_errors_through_the_connection() {
    for statement in STATEMENTS {
        let sql = string_literal(statement);
        for (call, name) in [
            (format!("db.exec({sql})"), "db.exec"),
            (format!("db.query({sql})"), "db.query"),
            (
                format!("db.fold({sql}, null, fn acc, row -> acc)"),
                "db.fold",
            ),
            (format!("db.script({sql})"), "db.script"),
        ] {
            assert_raises(&[(
                &with_db(&call),
                &format!("Function {name} cannot run a transaction statement{DB_HINT}"),
            )]);
        }
    }
}

#[test]
fn transaction_statements_are_errors_through_a_tx() {
    for statement in STATEMENTS {
        let sql = string_literal(statement);
        for (call, name) in [
            (format!("tx.exec({sql})"), "tx.exec"),
            (format!("tx.query({sql})"), "tx.query"),
            (
                format!("tx.fold({sql}, null, fn acc, row -> acc)"),
                "tx.fold",
            ),
            (format!("tx.script({sql})"), "tx.script"),
            (format!("tx.script('SELECT 1; ' + {sql})"), "tx.script"),
        ] {
            let source = with_db(&format!(
                r"
                def outcome = db.transaction(fn tx -> {call})
                [outcome.ok, outcome.error]
                "
            ));
            assert_values(&[(
                &source,
                &format!(
                    "[false, {}]",
                    string_literal(&format!(
                        "Function {name} cannot run a transaction statement{TX_HINT}"
                    ))
                ),
            )]);
        }
    }
}

#[test]
fn script_checks_each_statement() {
    let source = with_table(
        r"
        def attempt = try_call(db.script, ['INSERT INTO t (name) VALUES (1); COMMIT; INSERT INTO t (name) VALUES (2)'])
        [attempt.error, db.query('SELECT name FROM t')]
        ",
    );
    // The statements before it ran, each committing on its own.
    assert_values(&[(
        &source,
        &format!(
            "[{}, [{{name: '1'}}]]",
            string_literal(&format!(
                "Function db.script cannot run a transaction statement{DB_HINT}"
            ))
        ),
    )]);
    for sql in [
        "SELECT 1; BEGIN",
        "SELECT 1;\n-- the end\nEND",
        "SELECT 1; /* and */ ROLLBACK",
    ] {
        assert_raises(&[(
            &with_db(&format!("db.script({})", string_literal(sql))),
            &format!("Function db.script cannot run a transaction statement{DB_HINT}"),
        )]);
    }
}

#[test]
fn statements_merely_naming_a_transaction_word_run() {
    assert_values(&[
        (&with_db("db.query('SELECT 1 AS begin')"), "[{begin: 1}]"),
        (&with_db("db.exec('CREATE TABLE commits (x)')"), "0"),
        (&with_db(r#"db.exec('CREATE TABLE "end" (x)')"#), "0"),
        // A trigger's body is no transaction.
        (
            &with_db(
                "db.script('CREATE TABLE t (x); CREATE TRIGGER tr AFTER INSERT ON t BEGIN SELECT 1; END; INSERT INTO t VALUES (1)')",
            ),
            "1",
        ),
        // Explaining a statement runs none.
        (&with_db("db.query('EXPLAIN BEGIN')[0].opcode"), "'Init'"),
        (
            &with_db("db.query('EXPLAIN QUERY PLAN SELECT 1') @ len()"),
            "1",
        ),
        (
            &with_db("db.query('WITH x AS (SELECT 1 AS a) SELECT * FROM x')"),
            "[{a: 1}]",
        ),
    ]);
}

#[test]
fn what_is_not_a_transaction_statement_fails_as_sqlite_fails_it() {
    let failure = |message: &str| format!("Function db.exec failed: {message} (SQLITE_ERROR)");
    assert_raises(&[
        // A word beginning with a keyword is another word.
        (
            &with_db("db.exec('BEGINNING')"),
            &failure(r#"near "BEGINNING": syntax error"#),
        ),
        // A quoted keyword is a name.
        (
            &with_db(r#"db.exec('"BEGIN"')"#),
            &failure(r#"near ""BEGIN"": syntax error"#),
        ),
        (
            &with_db("db.exec('[BEGIN]')"),
            &failure(r#"near "[BEGIN]": syntax error"#),
        ),
        (
            &with_db("db.exec('`BEGIN`')"),
            &failure(r#"near "`BEGIN`": syntax error"#),
        ),
        // Before any other whitespace, a vertical tab is no whitespace.
        (
            &with_db(&format!("db.exec({})", string_literal("\x0bSELECT 1"))),
            &failure("unrecognized token: \"\x0b\""),
        ),
    ]);
    let none = "Function db.exec requires SQL of exactly one statement, but it has none";
    assert_raises(&[
        (&with_db("db.exec('/* unclosed BEGIN')"), none),
        (&with_db("db.exec('-- BEGIN')"), none),
    ]);
}

#[test]
fn a_vertical_tab_first_is_taken_for_a_transaction_statement() {
    // SQLite would fail it anyway, unable to read the tab.
    assert_raises(&[(
        &with_db(&format!("db.exec({})", string_literal("\x0bBEGIN"))),
        &format!("Function db.exec cannot run a transaction statement{DB_HINT}"),
    )]);
}
