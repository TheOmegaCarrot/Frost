//! Reports the SQLite that `frostlang-sqlite` linked, one fact per line:
//! `version <version>`, as a script reads `sqlite.version`, then
//! `option <option>` for each of the library's compile-time options.

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{ImporterBuilder, Value, Vm};
use frostlang_sqlite::rusqlite::Connection;

fn main() {
    println!("version {}", script_version());
    for option in compile_options() {
        println!("option {option}");
    }
}

/// `sqlite.version`, read by a Frost script.
fn script_version() -> String {
    let importer = ImporterBuilder::new()
        .with_extension(frostlang_sqlite::extension())
        .expect("a lone extension is accepted")
        .build();
    let closure = compile_program(
        "probe.frst",
        "import('ext.sqlite').version",
        CompilerOptions::new(),
    )
    .unwrap_or_else(|errors| panic!("the probe compiles:\n{}", errors.render()))
    .code
    .into_closure()
    .expect("the probe captures nothing");
    let result = Vm::factory()
        .with_importer(importer)
        .build(closure)
        .run()
        .unwrap_or_else(|error| panic!("the probe runs: {}", error.into_error()));
    match result.tail() {
        Value::String(version) => version.as_str().to_string(),
        other => panic!("`sqlite.version` should be a String, but is {other:?}"),
    }
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
