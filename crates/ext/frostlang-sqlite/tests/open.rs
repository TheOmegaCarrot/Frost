//! `ext.sqlite`'s own connection factory: `open`, `open_readonly`, and
//! `open_memory`.

use std::fs;

use crate::common::{
    NON_STRINGS, Scratch, assert_arity, assert_raises, assert_values, raises, string_literal, value,
};

/// The error for `function` failing to open the file at `path`.
fn cannot_open(function: &str, path: &str) -> String {
    format!(
        "Function ext.sqlite.{function} failed: unable to open database file: {path} \
         (SQLITE_CANTOPEN)"
    )
}

#[test]
fn open_creates_a_database_that_persists() {
    let scratch = Scratch::new();
    let path = scratch.literal("app.db");
    assert!(!scratch.path("app.db").exists(), "the file starts absent");
    assert_values(&[
        (
            &format!("sqlite.open({path}).script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')"),
            "1",
        ),
        // A second connection finds the table the first made.
        (
            &format!("sqlite.open({path}).script('INSERT INTO t VALUES (2)')"),
            "1",
        ),
    ]);
    assert!(scratch.path("app.db").is_file(), "`open` created the file");
}

#[test]
fn open_fails_where_no_database_can_be_made() {
    let scratch = Scratch::new();
    fs::create_dir(scratch.path("directory")).expect("the directory is made");
    assert_raises(&[
        (
            &format!("sqlite.open({})", scratch.literal("missing/app.db")),
            &cannot_open("open", &scratch.text("missing/app.db")),
        ),
        (
            &format!("sqlite.open({})", scratch.literal("directory")),
            &cannot_open("open", &scratch.text("directory")),
        ),
    ]);
}

#[test]
fn opening_a_file_that_is_not_a_database_fails() {
    let scratch = Scratch::new();
    fs::write(
        scratch.path("notes.txt"),
        "These are not the tables you seek.",
    )
    .expect("the file is written");
    for function in ["open", "open_readonly"] {
        assert_raises(&[(
            &format!("sqlite.{function}({})", scratch.literal("notes.txt")),
            &format!(
                "Function ext.sqlite.{function} failed: file is not a database (SQLITE_NOTADB)"
            ),
        )]);
    }
}

#[test]
fn an_empty_file_opens_as_an_empty_database() {
    let scratch = Scratch::new();
    fs::write(scratch.path("empty.db"), "").expect("the file is written");
    let path = scratch.literal("empty.db");
    assert_values(&[
        (
            &format!("sqlite.open_readonly({path}).script('SELECT 1')"),
            "0",
        ),
        (
            &format!("sqlite.open({path}).script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')"),
            "1",
        ),
    ]);
}

#[test]
fn open_takes_sqlites_special_names() {
    assert_values(&[
        // An in-memory database, as from `open_memory`.
        (
            "sqlite.open(':memory:').script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')",
            "1",
        ),
        // A private, temporary database on disk.
        (
            "sqlite.open('').script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')",
            "1",
        ),
    ]);
    assert_raises(&[(
        "sqlite.open_readonly(':memory:').script('CREATE TABLE t (x)')",
        "Function db.script failed: attempt to write a readonly database (SQLITE_READONLY)",
    )]);
}

/// Unix only: a `file:` URI of a Windows path takes another form.
#[cfg(unix)]
#[test]
fn open_takes_a_uri_too() {
    let scratch = Scratch::new();
    let target = scratch.text("app.db");
    assert!(
        !target.contains(['?', '#', '%']),
        "the scratch path {target:?} needs no escaping in a URI"
    );
    let uri = |query: &str| string_literal(&format!("file:{target}?{query}"));
    assert_values(&[(
        &format!(
            "sqlite.open({}).script('CREATE TABLE t (x)')",
            uri("cache=private")
        ),
        "0",
    )]);
    assert!(scratch.path("app.db").is_file(), "the URI named `app.db`");
    // A URI's `mode` cannot loosen read-only access.
    assert_raises(&[(
        &format!("sqlite.open_readonly({})", uri("mode=rw")),
        "Function ext.sqlite.open_readonly failed: access mode not allowed: rw (SQLITE_PERM)",
    )]);
}

#[test]
fn open_readonly_reads_but_never_writes() {
    let scratch = Scratch::new();
    let path = scratch.literal("app.db");
    value(&format!(
        "sqlite.open({path}).script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')"
    ));
    // `script` runs a SELECT, though it discards its rows.
    assert_values(&[(
        &format!("sqlite.open_readonly({path}).script('SELECT * FROM t')"),
        "0",
    )]);
    for write in [
        "INSERT INTO t VALUES (2)",
        "UPDATE t SET x = 2",
        "DELETE FROM t",
        "CREATE TABLE u (x)",
    ] {
        assert_raises(&[(
            &format!("sqlite.open_readonly({path}).script('{write}')"),
            "Function db.script failed: attempt to write a readonly database (SQLITE_READONLY)",
        )]);
    }
}

#[test]
fn open_readonly_creates_nothing() {
    let scratch = Scratch::new();
    assert_raises(&[(
        &format!("sqlite.open_readonly({})", scratch.literal("missing.db")),
        &cannot_open("open_readonly", &scratch.text("missing.db")),
    )]);
    assert!(
        !scratch.path("missing.db").exists(),
        "`open_readonly` left no file behind"
    );
}

#[test]
fn open_memory_gives_each_connection_its_own_database() {
    assert_values(&[(
        "sqlite.open_memory().script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')",
        "1",
    )]);
    let source = r"
        def first = sqlite.open_memory()
        first.script('CREATE TABLE t (x)')
        sqlite.open_memory().script('INSERT INTO t VALUES (1)')
    ";
    assert_eq!(
        raises(source),
        "Function db.script failed: no such table: t (SQLITE_ERROR)",
        "a second in-memory connection does not see the first's table"
    );
}

/// The bundled SQLite enforces foreign keys by default, so this pins only the
/// behaviour; the linkage probe checks it against libraries that do not.
#[test]
fn every_open_function_enforces_foreign_keys() {
    let scratch = Scratch::new();
    let path = scratch.literal("app.db");
    value(&format!("sqlite.open({path})"));
    // Temporary tables, which even a read-only connection can write.
    let schema = "CREATE TEMP TABLE parent (id INTEGER PRIMARY KEY); CREATE TEMP TABLE child (parent INTEGER REFERENCES parent (id))";
    for open in [
        format!("sqlite.open({path})"),
        format!("sqlite.open_readonly({path})"),
        "sqlite.open_memory()".to_string(),
    ] {
        let source = format!(
            r"
            def db = {open}
            db.script('{schema}')
            db.script('INSERT INTO child VALUES (1)')
            "
        );
        assert_eq!(
            raises(&source),
            "Function db.script failed: FOREIGN KEY constraint failed (SQLITE_CONSTRAINT)",
            "{open} enforces foreign keys"
        );
    }
}

#[test]
fn the_open_functions_reject_a_path_containing_nul() {
    for function in ["open", "open_readonly"] {
        assert_raises(&[(
            &format!(r"sqlite.{function}('app\0.db')"),
            &format!("Function ext.sqlite.{function} requires a path without NUL characters"),
        )]);
    }
}

#[test]
fn the_open_functions_check_their_arity() {
    assert_arity("sqlite.open", "ext.sqlite.open", 1, &[0, 2]);
    assert_arity(
        "sqlite.open_readonly",
        "ext.sqlite.open_readonly",
        1,
        &[0, 2],
    );
    assert_arity("sqlite.open_memory", "ext.sqlite.open_memory", 0, &[1, 2]);
}

#[test]
fn the_open_functions_check_their_argument_types() {
    for function in ["open", "open_readonly"] {
        for (argument, kind) in NON_STRINGS {
            assert_raises(&[(
                &format!("sqlite.{function}({argument})"),
                &format!(
                    "Function ext.sqlite.{function} requires String as argument 1 (path), \
                     got {kind}"
                ),
            )]);
        }
    }
}
