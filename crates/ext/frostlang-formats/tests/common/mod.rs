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
use frostlang_formats::uint64::Uint64;

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
        def cbor = import('ext.formats.cbor')
        def json = import('ext.formats.json')
        def msgpack = import('ext.formats.msgpack')
        def toml = import('ext.formats.toml')
        def u64 = import('ext.formats.u64')
        def yaml = import('ext.formats.yaml')
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

/// A format's module of the extension, as tests of its errors name it.
pub(crate) struct FormatModule {
    /// The module's name, as in `ext.formats.json`.
    pub(crate) name: &'static str,
    /// The format's name, as errors give it, such as `JSON`.
    pub(crate) format: &'static str,
}

impl FormatModule {
    /// The error `encode` raises for `what`, which the format has no form
    /// for, as the whole value encoded.
    pub(crate) fn cannot_encode(&self, what: &str) -> String {
        format!(
            "Function ext.formats.{}.encode cannot encode {what} in {}",
            self.name, self.format
        )
    }

    /// The error `encode` raises for `what`, which the format has no form
    /// for, found at `path` within the value encoded.
    pub(crate) fn cannot_encode_at(&self, what: &str, path: &str) -> String {
        format!("{} (at `{path}`)", self.cannot_encode(what))
    }

    /// The error `encode` raises for `what`, which the format has a form for,
    /// but not as a whole document.
    pub(crate) fn cannot_encode_as_document(&self, what: &str) -> String {
        format!(
            "Function ext.formats.{}.encode cannot encode {what} as a {} document",
            self.name, self.format
        )
    }

    /// The error `encode` raises for Arrays and Maps nested deeper than
    /// `limit`.
    pub(crate) fn too_deep(&self, limit: usize) -> String {
        format!(
            "Function ext.formats.{}.encode cannot encode Arrays and Maps nested more than \
             {limit} deep in {}",
            self.name, self.format
        )
    }

    /// The start of every error `decode` raises. The rest is the parser's
    /// account of the problem, which the tests trust to be reasonable rather
    /// than pin, except where the extension gives the reason itself.
    fn cannot_decode(&self) -> String {
        format!(
            "Function ext.formats.{}.decode cannot decode {}: ",
            self.name, self.format
        )
    }

    /// The reason given by the decode error `expression` raises, which must
    /// be one: the error with its prefix taken off.
    pub(crate) fn decode_error_reason(&self, expression: &str) -> String {
        let prefix = self.cannot_decode();
        let message = raises(expression);
        message
            .strip_prefix(&prefix)
            .unwrap_or_else(|| panic!("{expression:?} raises {message:?}, not {prefix:?}..."))
            .to_string()
    }

    /// Assert each expression raises a decode error.
    pub(crate) fn assert_cannot_decode(&self, expressions: &[&str]) {
        for expression in expressions {
            self.assert_cannot_decode_because(&[(expression, "")]);
        }
    }

    /// Assert each expression raises a decode error whose reason begins with
    /// the given text.
    pub(crate) fn assert_cannot_decode_because(&self, cases: &[(&str, &str)]) {
        let prefix = self.cannot_decode();
        for (expression, reason) in cases {
            let message = raises(expression);
            assert!(
                message.starts_with(&format!("{prefix}{reason}")),
                "{expression:?} raises {message:?}, which should begin {prefix:?}{reason:?}"
            );
        }
    }
}

/// A Frost expression for `depth` Arrays, each the only element of the one
/// enclosing it.
pub(crate) fn nested_arrays(depth: usize) -> String {
    format!("reduce(range({}), [], fn acc, i -> [acc])", depth - 1)
}

/// A Frost expression for `depth` Maps, each the only entry of the one
/// enclosing it, under `a`.
pub(crate) fn nested_maps(depth: usize) -> String {
    format!(
        "reduce(range({}), {{}}, fn acc, i -> {{a: acc}})",
        depth - 1
    )
}

/// The [`Uint64`] of `n`, which must be beyond the Int range.
pub(crate) fn uint64(n: u64) -> Uint64 {
    Uint64::new(n).unwrap_or_else(|| panic!("{n} should be beyond the Int range"))
}

/// The cases of `cases`, borrowed as [`assert_raises`] takes them.
pub(crate) fn borrowed(cases: &[(String, String)]) -> Vec<(&str, &str)> {
    cases
        .iter()
        .map(|(expression, message)| (expression.as_str(), message.as_str()))
        .collect()
}

/// The type errors `function`, a path within the extension such as
/// `toml.decode`, raises for each argument in `arguments` as its only
/// argument: one of the type paired with it, where the parameter `parameter`
/// requires `expected`.
pub(crate) fn type_errors(
    function: &str,
    expected: &str,
    parameter: &str,
    arguments: &[(&str, &str)],
) -> Vec<(String, String)> {
    arguments
        .iter()
        .map(|(argument, type_name)| {
            (
                format!("{function}({argument})"),
                format!(
                    "Function ext.formats.{function} requires {expected} as argument 1 \
                     ({parameter}), got {type_name}"
                ),
            )
        })
        .collect()
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
