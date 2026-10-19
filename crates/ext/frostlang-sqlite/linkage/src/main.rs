//! Reports the SQLite that `frostlang-sqlite` linked, and how the extension
//! behaves on it, one fact per line:
//!
//! - `version <version>`, as a script reads `sqlite.version`;
//! - `option <option>` for each of the library's compile-time options;
//! - `foreign_keys <function> <enforced|ignored>` for each `open` function;
//! - `uri <interpreted|literal>`, how `open` reads a `file:` URI;
//! - `syntax_error <message>`, the error for SQL that does not parse.

use std::path::Path;
use std::{env, fs, process};

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{FrostError, ImporterBuilder, RunError, Value, Vm};
use frostlang_sqlite::rusqlite::Connection;

fn main() {
    match script(r"import('ext.sqlite').version") {
        Ok(Value::String(version)) => println!("version {version}"),
        other => panic!("`sqlite.version` should be a String, but is {other:?}"),
    }
    for option in compile_options() {
        println!("option {option}");
    }

    let scratch = env::temp_dir().join(format!("frostlang-sqlite-linkage-{}", process::id()));
    fs::create_dir_all(&scratch).expect("the scratch directory is made");
    let database = text(&scratch.join("app.db"));
    script(&format!("import('ext.sqlite').open('{database}')")).expect("the database is made");
    for (function, call) in [
        ("open", format!("open('{database}')")),
        ("open_readonly", format!("open_readonly('{database}')")),
        ("open_memory", "open_memory()".to_string()),
    ] {
        let open = format!("import('ext.sqlite').{call}");
        println!("foreign_keys {function} {}", foreign_keys(&open));
    }

    let target = scratch.join("uri.db");
    let opened = script(&format!(
        "import('ext.sqlite').open('file:{}?mode=rwc')",
        text(&target)
    ));
    println!(
        "uri {}",
        if opened.is_ok() && target.is_file() {
            "interpreted"
        } else {
            "literal"
        }
    );

    match script("import('ext.sqlite').open_memory().script('SELEC 1')") {
        Err(error) => println!("syntax_error {}", error.message()),
        Ok(value) => panic!("SQL that does not parse should fail, but gave {value:?}"),
    }

    fs::remove_dir_all(&scratch).expect("the scratch directory is removed");
}

/// Whether the connection `open` makes enforces foreign keys, shown through
/// temporary tables, which even a read-only connection can write.
fn foreign_keys(open: &str) -> &'static str {
    let orphan = script(&format!(
        r"
        def db = {open}
        db.script('CREATE TEMP TABLE parent (id INTEGER PRIMARY KEY); CREATE TEMP TABLE child (parent INTEGER REFERENCES parent (id))')
        db.script('INSERT INTO child VALUES (1)')
        "
    ));
    match orphan {
        Ok(_) => "ignored",
        Err(error)
            if error.message()
                == "Function db.script failed: FOREIGN KEY constraint failed (SQLITE_CONSTRAINT)" =>
        {
            "enforced"
        }
        Err(error) => panic!("{open} should enforce foreign keys or not, but failed: {error}"),
    }
}

/// `path` as text, which the probe's scratch paths always are, and which
/// needs no escaping in a Frost String or a URI.
fn text(path: &Path) -> String {
    let text = path.to_str().expect("the scratch path is UTF-8");
    assert!(
        !text.contains(['\'', '\\', '?', '#', '%']),
        "the scratch path {text:?} needs no escaping"
    );
    text.to_string()
}

/// What the Frost script `source` returns, run with `ext.sqlite`.
fn script(source: &str) -> Result<Value, FrostError> {
    let importer = ImporterBuilder::new()
        .with_extension(frostlang_sqlite::extension())
        .expect("a lone extension is accepted")
        .build();
    let closure = compile_program("probe.frst", source, CompilerOptions::new())
        .unwrap_or_else(|errors| panic!("the probe compiles:\n{}", errors.render()))
        .code
        .into_closure()
        .expect("the probe captures nothing");
    Vm::factory()
        .with_importer(importer)
        .build(closure)
        .run()
        .map(|result| result.tail().clone())
        .map_err(RunError::into_error)
}

/// The linked library's compile-time options, as `PRAGMA compile_options`
/// lists them.
fn compile_options() -> Vec<String> {
    let connection = Connection::open_in_memory().expect("an in-memory database opens");
    let mut statement = connection
        .prepare("PRAGMA compile_options")
        .expect("the pragma prepares");
    statement
        .query_map([], |row| row.get(0))
        .expect("the pragma runs")
        .collect::<Result<_, _>>()
        .expect("each option is text")
}
