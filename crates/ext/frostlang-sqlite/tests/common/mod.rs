//! Harness for the extension's tests: a script run with only `ext.sqlite`
//! installed, bound as `sqlite`.

use std::sync::Arc;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{FrostError, Importer, ImporterBuilder, RunError, Value, Vm};

/// An importer providing only the extension.
fn importer() -> Arc<Importer> {
    ImporterBuilder::new()
        .with_extension(frostlang_sqlite::extension())
        .expect("a lone extension is accepted")
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
