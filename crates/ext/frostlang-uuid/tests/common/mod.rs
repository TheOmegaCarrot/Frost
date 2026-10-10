//! Harness for the extension's tests: a script run with only `ext.uuid`
//! installed, bound as `uuid`.

use std::fmt::Display;
use std::sync::Arc;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{FrostError, Importer, ImporterBuilder, RunError, Value, Vm};

/// An importer providing only the extension.
fn importer() -> Arc<Importer> {
    ImporterBuilder::new()
        .with_extension(frostlang_uuid::extension())
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

/// Run `expression` with the extension bound as `uuid`.
fn run(expression: &str) -> Result<Value, FrostError> {
    let source = format!(
        r"
        def uuid = import('ext.uuid')
        {expression}
        "
    );
    run_source(&source, importer())
}

/// The value of `expression`, which must run.
pub(crate) fn value(expression: &str) -> Value {
    run(expression).unwrap_or_else(|error| panic!("{expression:?} should run, but raised: {error}"))
}

/// The String that `expression` runs to.
pub(crate) fn string(expression: &str) -> String {
    match value(expression) {
        Value::String(text) => text.as_str().to_string(),
        other => panic!("{expression:?} should be a String, but is {other:?}"),
    }
}

/// The Int that `expression` runs to.
pub(crate) fn int(expression: &str) -> i64 {
    value(expression)
        .as_int()
        .unwrap_or_else(|| panic!("{expression:?} should be an Int"))
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

/// Assert each `expression` runs to `true`.
pub(crate) fn assert_true(expressions: &[&str]) {
    for expression in expressions {
        assert_eq!(value(expression), Value::Bool(true), "{expression:?} holds");
    }
}

/// Assert each `expression` runs to Null.
pub(crate) fn assert_nulls(expressions: &[&str]) {
    for expression in expressions {
        assert_eq!(value(expression), Value::Null, "{expression:?} is Null");
    }
}

/// Assert each `expression` raises exactly `message`.
pub(crate) fn assert_raises(cases: &[(&str, &str)]) {
    for (expression, message) in cases {
        assert_eq!(raises(expression), *message, "{expression:?}");
    }
}

/// Assert `function`, a path within the extension such as `ulid.new`, raises
/// its arity error when called with each count in `counts`. `expects` is how
/// the error states its arity, such as `1`.
pub(crate) fn assert_arity(function: &str, expects: impl Display, counts: &[usize]) {
    for &argc in counts {
        let expression = format!("uuid.{function}({})", vec!["null"; argc].join(", "));
        assert_eq!(
            raises(&expression),
            format!(
                "Function uuid.{function} expects {expects} arguments, but was called with {argc}"
            ),
            "{expression:?}"
        );
    }
}

/// Milliseconds since the Unix epoch, now.
pub(crate) fn now_millis() -> u64 {
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is past the epoch");
    u64::try_from(since_epoch.as_millis()).expect("the time fits 64 bits")
}
