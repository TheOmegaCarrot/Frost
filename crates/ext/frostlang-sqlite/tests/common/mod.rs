//! Harness for the extension's tests: a script run with only `ext.sqlite`
//! installed, bound as `sqlite`, or with a host's own component.

use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{FrostError, HostComponent, Importer, ImporterBuilder, RunError, Value, Vm};
use frostlang_sqlite::rusqlite::Connection;
use frostlang_sqlite::sqlite_error;
use tempfile::TempDir;

/// An importer providing only the extension.
pub(crate) fn importer() -> Arc<Importer> {
    ImporterBuilder::new()
        .with_extension(frostlang_sqlite::extension())
        .expect("a lone extension is accepted")
        .build()
}

/// An importer providing only `component`, and not the extension.
pub(crate) fn host_importer(component: HostComponent) -> Arc<Importer> {
    ImporterBuilder::new()
        .with_component(component)
        .expect("a lone component is accepted")
        .build()
}

/// The host component `app`, holding `entries`.
pub(crate) fn app_component(entries: Vec<(&str, Value)>) -> HostComponent {
    entries
        .into_iter()
        .fold(HostComponent::builder("app"), |app, (key, value)| {
            app.value(key, value)
        })
        .build()
}

/// Run `source` with only the host component `app`, holding `entries`.
pub(crate) fn run_app(source: &str, entries: Vec<(&str, Value)>) -> Result<Value, FrostError> {
    run_source(source, host_importer(app_component(entries)))
}

/// The value of `source`, run with `app.open_db` from `factory`, which must
/// run.
pub(crate) fn app_value(source: &str, factory: Value) -> Value {
    run_app(source, vec![("open_db", factory)])
        .unwrap_or_else(|error| panic!("{source:?} should run, but raised: {error}"))
}

/// The error message `source` raises, run with `app.open_db` from `factory`.
pub(crate) fn app_raises(source: &str, factory: Value) -> String {
    match run_app(source, vec![("open_db", factory)]) {
        Ok(value) => panic!("{source:?} should raise, but produced {value:?}"),
        Err(error) => error.message().into_owned(),
    }
}

/// The value of the Frost expression `expression`, which needs no import.
pub(crate) fn frost(expression: &str) -> Value {
    run_app(expression, Vec::new()).expect("the expected value runs")
}

/// A new in-memory connection, failing as the factory `app.open_db`.
pub(crate) fn memory_connection() -> Result<Connection, FrostError> {
    Connection::open_in_memory().map_err(|err| sqlite_error("app.open_db", &err))
}

/// A connection as a pool lends it, which runs a check on the connection as
/// it returns to the pool.
pub(crate) struct Pooled {
    connection: Connection,
    on_release: Box<dyn FnMut(&Connection) + Send>,
}

impl Pooled {
    /// Lends `connection`, running `on_release` on it as it returns.
    pub(crate) fn new(
        connection: Connection,
        on_release: impl FnMut(&Connection) + Send + 'static,
    ) -> Self {
        Self {
            connection,
            on_release: Box::new(on_release),
        }
    }
}

impl Deref for Pooled {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        &self.connection
    }
}

impl DerefMut for Pooled {
    fn deref_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}

impl Drop for Pooled {
    fn drop(&mut self) {
        (self.on_release)(&self.connection);
    }
}

/// Run `source` as a whole script with `importer`.
pub(crate) fn run_source(source: &str, importer: Arc<Importer>) -> Result<Value, FrostError> {
    let closure = compile_program("main.frst", source, CompilerOptions::new())
        .unwrap_or_else(|errors| panic!("the script compiles:\n{}", errors.render()))
        .code
        .into_closure()
        .expect("the script captures nothing");
    Vm::factory()
        .with_importer(importer)
        .build(closure)
        .run()
        .map_err(RunError::into_error)
        .map(|result| result.tail().clone())
}

/// Run `expression` with the extension bound as `sqlite`.
fn run(expression: &str) -> Result<Value, FrostError> {
    let source = format!(
        r"
        def sqlite = import('ext.sqlite')
        {expression}
        "
    );
    run_source(&source, importer())
}

/// `body`, run with `db` bound to a new in-memory connection.
pub(crate) fn with_db(body: &str) -> String {
    format!(
        r"
        def db = sqlite.open_memory()
        {body}
        "
    )
}

/// `body`, run with `db` holding an empty table `t (id INTEGER PRIMARY KEY,
/// name TEXT)`.
pub(crate) fn with_table(body: &str) -> String {
    with_db(&format!(
        r"
        db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)')
        {body}
        "
    ))
}

/// `body`, run with `db` holding the table `t (id INTEGER PRIMARY KEY, name
/// TEXT)` of three rows: `(1, 'a')`, `(2, 'b')`, and `(3, 'c')`.
pub(crate) fn with_rows(body: &str) -> String {
    with_table(&format!(
        r#"
        db.exec("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        {body}
        "#
    ))
}

/// The value of `expression`, which must run.
pub(crate) fn value(expression: &str) -> Value {
    run(expression).unwrap_or_else(|error| panic!("{expression:?} should run, but raised: {error}"))
}

/// The error message `expression` raises.
pub(crate) fn raises(expression: &str) -> String {
    match run(expression) {
        Ok(value) => panic!("{expression:?} should raise, but produced {value:?}"),
        Err(error) => error.message().into_owned(),
    }
}

/// Assert each `expression` runs to the value of the Frost expression
/// `expected`.
pub(crate) fn assert_values(cases: &[(&str, &str)]) {
    for (expression, expected) in cases {
        assert_eq!(
            value(expression),
            value(expected),
            "{expression:?} is {expected}"
        );
    }
}

/// Assert each `expression` raises exactly `message`.
pub(crate) fn assert_raises(cases: &[(&str, &str)]) {
    for (expression, message) in cases {
        assert_eq!(raises(expression), *message, "{expression:?}");
    }
}

/// Assert the Frost expression `callee`, a function named `name`, raises its
/// arity error when called with each count in `counts`. `expects` is how the
/// error states its arity, such as `1`.
pub(crate) fn assert_arity(callee: &str, name: &str, expects: impl Display, counts: &[usize]) {
    for &argc in counts {
        let expression = format!("{callee}({})", vec!["null"; argc].join(", "));
        assert_eq!(
            raises(&expression),
            format!("Function {name} expects {expects} arguments, but was called with {argc}"),
            "{expression:?}"
        );
    }
}

/// A Frost value of every type but String, each with its type's name: the
/// arguments a String parameter rejects.
pub(crate) const NON_STRINGS: [(&str, &str); 8] = [
    ("null", "Null"),
    ("true", "Bool"),
    ("1", "Int"),
    ("1.5", "Float"),
    ("x'00'", "Bytes"),
    ("[]", "Array"),
    ("{}", "Map"),
    ("fn -> 1", "Function"),
];

/// A scratch directory of one test's own, removed when it is dropped.
pub(crate) struct Scratch(TempDir);

impl Scratch {
    /// A fresh, empty directory.
    pub(crate) fn new() -> Self {
        Self(TempDir::new().expect("the scratch directory is created"))
    }

    /// The path of `relative` within the directory.
    pub(crate) fn path(&self, relative: &str) -> PathBuf {
        self.0.path().join(relative)
    }

    /// The path of `relative` within the directory, as text.
    pub(crate) fn text(&self, relative: &str) -> String {
        self.path(relative)
            .into_os_string()
            .into_string()
            .expect("the scratch path is UTF-8")
    }

    /// The path of `relative` within the directory, as a Frost String
    /// literal.
    pub(crate) fn literal(&self, relative: &str) -> String {
        string_literal(&self.text(relative))
    }
}

/// `text` as a Frost String literal.
pub(crate) fn string_literal(text: &str) -> String {
    format!(
        "'{}'",
        text.replace('\\', r"\\")
            .replace('\'', r"\'")
            .replace('\n', r"\n")
    )
}
