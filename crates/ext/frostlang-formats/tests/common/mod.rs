//! Harness for the extension's tests: a script run with the extension
//! installed with every format, each format bound by its module name, as
//! `json`. `std.math` is bound as `math`, for its special floats, and a host
//! component as `test`, whose `widget` is an Opaque no format knows.

use std::sync::Arc;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{
    FrostError, FrostOpaque, HostComponent, Importer, ImporterBuilder, RunError, Stdlib, Value, Vm,
    stdlib,
};
use frostlang_formats::Formats;

/// An Opaque of the host's own, which no format has a form for.
#[derive(Debug)]
struct Widget;

impl FrostOpaque for Widget {
    fn type_name() -> &'static str {
        "Widget"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// An importer providing the extension with `formats`, `std.math`, and the
/// `test` component.
pub(crate) fn importer(formats: Formats) -> Arc<Importer> {
    let stdlib = Stdlib::new()
        .with_module(stdlib::math())
        .expect("a lone module is accepted");
    let test = HostComponent::builder("test")
        .value("widget", Value::opaque(Widget))
        .build();
    ImporterBuilder::new()
        .with_stdlib(stdlib)
        .with_extension(frostlang_formats::extension(formats))
        .expect("a lone extension is accepted")
        .with_component(test)
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

/// Run `expression` with every format bound.
fn run(expression: &str) -> Result<Value, FrostError> {
    let source = format!(
        r"
        def json = import('ext.formats.json')
        def math = import('std.math')
        def test = import('test')
        {expression}
        "
    );
    run_source(&source, importer(Formats::all()))
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

/// Assert each `expression` runs to the String `expected`.
pub(crate) fn assert_strings(cases: &[(&str, &str)]) {
    for (expression, expected) in cases {
        assert_eq!(string(expression), *expected, "{expression:?}");
    }
}

/// Assert each `expression` runs to `true`.
pub(crate) fn assert_true(expressions: &[&str]) {
    for expression in expressions {
        assert_eq!(value(expression), Value::Bool(true), "{expression:?} holds");
    }
}

/// Assert each `expression` raises exactly `message`.
pub(crate) fn assert_raises(cases: &[(&str, &str)]) {
    for (expression, message) in cases {
        assert_eq!(raises(expression), *message, "{expression:?}");
    }
}

/// Assert `function`, a path within the extension such as `json.encode`,
/// raises its arity error when called with each count in `counts`. `expects`
/// is how the error states its arity, such as `1` or `between 1 and 2`.
pub(crate) fn assert_arity(function: &str, expects: &str, counts: &[usize]) {
    let noun = if expects == "1" {
        "argument"
    } else {
        "arguments"
    };
    for &argc in counts {
        let expression = format!("{function}({})", vec!["null"; argc].join(", "));
        assert_eq!(
            raises(&expression),
            format!(
                "Function ext.formats.{function} expects {expects} {noun}, \
                 but was called with {argc}"
            ),
            "{expression:?}"
        );
    }
}
