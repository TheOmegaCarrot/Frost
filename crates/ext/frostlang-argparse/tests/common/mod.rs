//! Harness for the extension's tests: a script run with only `ext.argparse`
//! installed, bound as `argparse`.

use std::sync::Arc;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{FrostError, Importer, ImporterBuilder, MapKey, RunError, Value, Vm};

/// An importer providing only the extension.
fn importer() -> Arc<Importer> {
    ImporterBuilder::new()
        .with_extension(frostlang_argparse::extension())
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

/// Run `expression` with the extension bound as `argparse`.
pub(crate) fn run(expression: &str) -> Result<Value, FrostError> {
    let source = format!(
        r"
        def argparse = import('ext.argparse')
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

/// The program's arguments as a Frost Array, after a program name `prog`.
fn command_line(arguments: &[&str]) -> String {
    let quoted: Vec<String> = ["prog"]
        .iter()
        .chain(arguments)
        .map(|argument| Value::from(*argument).to_debug_string())
        .collect();
    format!("[{}]", quoted.join(", "))
}

/// What parsing `arguments`, after a program name, as the Frost Map `spec`
/// gives: the parse's outcome Map.
pub(crate) fn parse(spec: &str, arguments: &[&str]) -> Value {
    value(&format!(
        "argparse.parse({}, {spec})",
        command_line(arguments)
    ))
}

/// The one key of `outcome`, an outcome Map, and its value.
fn outcome(outcome: Value) -> (String, Value) {
    let Value::Map(map) = &outcome else {
        panic!("an outcome is a Map, but got {outcome:?}");
    };
    let mut entries = map.iter();
    match (entries.next(), entries.next()) {
        (Some((MapKey::String(key), value)), None) => (key.as_str().to_string(), value.clone()),
        _ => panic!("an outcome has exactly one String key, but got {outcome:?}"),
    }
}

/// The outcome of parsing `arguments` as `spec`, which must be the one keyed
/// `expected`.
fn parse_to(expected: &str, spec: &str, arguments: &[&str]) -> Value {
    let (key, value) = outcome(parse(spec, arguments));
    assert_eq!(
        key, expected,
        "{arguments:?} parsed as {spec} should give `{expected}`, but gave `{key}`: {value:?}"
    );
    value
}

/// The `value` Map parsing `arguments` as `spec` gives; the parse must succeed.
pub(crate) fn parsed(spec: &str, arguments: &[&str]) -> Value {
    parse_to("value", spec, arguments)
}

/// The text of the outcome keyed `key`, `help`, `version`, or `error`, that
/// parsing `arguments` as `spec` must give.
pub(crate) fn text(key: &str, spec: &str, arguments: &[&str]) -> String {
    match parse_to(key, spec, arguments) {
        Value::String(text) => text.as_str().to_string(),
        other => panic!("`{key}` holds a String, but got {other:?}"),
    }
}

/// The usage error parsing `arguments` as `spec` must give.
pub(crate) fn usage_error(spec: &str, arguments: &[&str]) -> String {
    text("error", spec, arguments)
}

/// Assert that parsing each command line as `spec` gives the `value` Map of
/// the Frost expression paired with it.
pub(crate) fn assert_parses(spec: &str, cases: &[(&[&str], &str)]) {
    for (arguments, expected) in cases {
        assert_eq!(
            parsed(spec, arguments),
            value(expected),
            "{arguments:?} parsed as {spec} should be {expected}"
        );
    }
}

/// Assert that parsing each command line as `spec` is a usage error whose text
/// holds what is paired with it: the argument at fault, as clap writes it.
pub(crate) fn assert_usage_errors(spec: &str, cases: &[(&[&str], &str)]) {
    for (arguments, names) in cases {
        let error = usage_error(spec, arguments);
        assert!(
            error.contains(names),
            "{arguments:?} parsed as {spec} should be an error naming {names:?}, but said:\n{error}"
        );
    }
}

/// Assert that each spec raises the error that `parse` raises for an invalid
/// spec, with the problem paired with it.
pub(crate) fn assert_invalid_specs(cases: &[(&str, &str)]) {
    for (spec, problem) in cases {
        assert_eq!(
            raises(&format!("argparse.parse(['prog'], {spec})")),
            format!(
                "Function ext.argparse.parse requires argument 2 (spec) to be valid: {problem}"
            ),
            "{spec}"
        );
    }
}
