//! Harness for the extension's tests: a script run with only `ext.sqlite`
//! installed, bound as `sqlite`, or with a host's own component.

use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{FrostError, HostComponent, Importer, ImporterBuilder, RunError, Value, Vm};
use tempfile::TempDir;

/// An importer providing only the extension.
fn importer() -> Arc<Importer> {
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
    format!("'{}'", text.replace('\\', r"\\").replace('\'', r"\'"))
}
