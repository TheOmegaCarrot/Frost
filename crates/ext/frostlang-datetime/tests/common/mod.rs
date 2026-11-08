//! Harness for the extension's tests: a script run with only `ext.datetime`
//! installed, bound as `dt`.

use std::fmt::Display;
use std::sync::Arc;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{FrostError, Importer, ImporterBuilder, RunError, Value, Vm};

/// An importer providing only the extension.
fn importer() -> Arc<Importer> {
    ImporterBuilder::new()
        .with_extension(frostlang_datetime::extension())
        .expect("a lone extension is accepted")
        .build()
}

/// Run `expression` with the extension bound as `dt`.
fn run(expression: &str) -> Result<Value, FrostError> {
    let source = format!(
        r"
        def dt = import('ext.datetime')
        {expression}
        "
    );
    let closure = compile_program("main.frst", &source, CompilerOptions::new())
        .unwrap_or_else(|errors| panic!("the script compiles:\n{}", errors.render()))
        .code
        .into_closure()
        .expect("the script captures nothing");
    Vm::factory()
        .with_importer(importer())
        .build(closure)
        .run()
        .map_err(RunError::into_error)
        .map(|result| result.tail().clone())
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

/// Assert `function`, a function of the extension such as `components`, raises
/// its arity error when called with each count in `counts`. `expects` is how
/// the error states its arity, such as `1` or `between 1 and 2`.
pub(crate) fn assert_arity(function: &str, expects: impl Display, counts: &[usize]) {
    let expects = expects.to_string();
    let noun = if expects == "1" {
        "argument"
    } else {
        "arguments"
    };
    for &argc in counts {
        let expression = format!("dt.{function}({})", vec!["null"; argc].join(", "));
        assert_eq!(
            raises(&expression),
            format!(
                "Function ext.datetime.{function} expects {expects} {noun}, but was called with {argc}"
            ),
            "{expression:?}"
        );
    }
}
