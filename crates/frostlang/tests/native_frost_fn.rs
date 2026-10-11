//! `#[frost_fn]`: Frost functions made from Rust functions.
//!
//! The first section checks generated functions against equivalents built by
//! hand in `native_args`, so that the macro is shown to generate the same
//! functions. The rest covers what only the macro decides: names, the
//! context, return conversion, and how parameter types are named.

use frostlang::native::{ArgSite, FromArg, IntoNativeResult, Nullable, Optional, Rest};
use frostlang::{FrostError, FrostResult, FrostType, NativeCtx, TypeSet, Value, frost_fn};

use crate::native_args;
use crate::script::Script;

/// Asserts that a run of each of `sources`, with `name` bound to the
/// generated function, ends exactly as with the hand-built one.
fn assert_alike(name: &str, generated: fn() -> Value, hand_built: fn() -> Value, sources: &[&str]) {
    for source in sources {
        let outcome =
            |function: fn() -> Value| Script::new(source).capture(name, function()).outcome();
        assert_eq!(
            outcome(generated),
            outcome(hand_built),
            "{source:?} ends alike with either function"
        );
    }
}

fn run(name: &str, function: Value, source: &str) -> Value {
    Script::new(source).capture(name, function).run()
}

fn raises(name: &str, function: Value, source: &str) -> String {
    Script::new(source).capture(name, function).raises()
}

// ============================================================
// Alike to hand-built functions
// ============================================================

#[frost_fn(name = "pad")]
fn pad(text: String, width: u16, fill: Optional<Nullable<String>>) -> String {
    let fill = match fill {
        Optional(None) => " ".to_string(),
        Optional(Some(Nullable(None))) => return text,
        Optional(Some(Nullable(Some(fill)))) => fill,
    };
    let missing = usize::from(width).saturating_sub(text.chars().count());
    format!("{}{text}", fill.repeat(missing))
}

#[test]
fn required_and_optional_nullable_parameters_are_alike() {
    assert_alike(
        "pad",
        pad_frost,
        native_args::pad,
        &[
            "pad('ab', 4)",
            "pad('ab', 4, '0')",
            "pad('ab', 4, null)",
            "pad('abcdef', 4)",
            "pad('ab', '4')",
            "pad('ab', 4, 0)",
            "pad('ab', 70000)",
            "pad('ab')",
            "pad('ab', 4, '0', 'extra')",
        ],
    );
}

/// Any Array or Map, as `native_args::each` declares its bindings.
struct Bindings(Value);

impl FromArg for Bindings {
    const TYPES: TypeSet = FrostType::STRUCTURED;

    fn from_arg(value: Value, _: &ArgSite<'_>) -> Result<Self, FrostError> {
        Ok(Self(value))
    }
}

/// Any Function, as `native_args::each` declares its callback.
struct Callback(Value);

impl FromArg for Callback {
    const TYPES: TypeSet = FrostType::FUNCTION;

    fn from_arg(value: Value, _: &ArgSite<'_>) -> Result<Self, FrostError> {
        Ok(Self(value))
    }
}

#[frost_fn(name = "each")]
fn each(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    sql: String,
    bindings: Optional<Bindings>,
    callback: Callback,
) -> FrostResult {
    let count = match bindings.0.map(|Bindings(value)| value) {
        None => Value::Null,
        Some(Value::Array(array)) => Value::Int(array.len() as i64),
        Some(Value::Map(map)) => Value::Int(map.len() as i64),
        Some(other) => unreachable!("type-checked, got {}", other.type_name()),
    };
    ctx.invoke(&callback.0, [Value::from(sql), count])
}

#[test]
fn an_interior_optional_and_the_context_are_alike() {
    assert_alike(
        "each",
        each_frost,
        native_args::each,
        &[
            "each('q', fn sql, n -> [sql, n])",
            "each('q', [1, 2], fn sql, n -> [sql, n])",
            "each('q', {a: 1}, fn sql, n -> [sql, n])",
            "each('q', [1])",
            "each('q', fn x -> x, fn x -> x)",
            "each('q', [1], [2])",
            "each('q')",
            "each('q', [1], fn x, y -> x + y)",
        ],
    );
}

#[frost_fn(name = "sum")]
fn sum(ns: Rest<i64>) -> i64 {
    ns.0.iter().sum()
}

#[test]
fn a_rest_parameter_is_alike() {
    assert_alike(
        "sum",
        sum_frost,
        native_args::sum,
        &["sum()", "sum(5)", "sum(1, 2, 3)", "sum(1, 2, 'x')"],
    );
}

// ============================================================
// Names
// ============================================================

#[frost_fn(name = concat!("text", ".", "shout"))]
fn shout(text: String) -> String {
    text.to_uppercase()
}

const SHOUT_NAME: &str = "loud";

#[frost_fn(name = SHOUT_NAME)]
fn loud(text: String) -> String {
    text.to_uppercase()
}

#[test]
fn the_name_is_any_constant_str_expression() {
    assert_eq!(
        raises("f", shout_frost(), "f(1)"),
        "Function text.shout requires String as argument 1 (text), got Int"
    );
    assert_eq!(
        raises("f", loud_frost(), "f(1)"),
        "Function loud requires String as argument 1 (text), got Int"
    );
}

#[frost_fn(name = "whoami")]
fn whoami(#[frost(ctx)] ctx: &mut NativeCtx<'_>) -> &'static str {
    ctx.name()
}

#[test]
fn the_context_knows_the_name() {
    assert_eq!(run("f", whoami_frost(), "f()"), Value::from("whoami"));
}

#[frost_fn(name = "raw")]
fn raw(r#type: String) -> String {
    r#type
}

#[test]
fn a_raw_parameter_name_is_named_without_its_prefix() {
    assert_eq!(
        raises("f", raw_frost(), "f(1)"),
        "Function raw requires String as argument 1 (type), got Int"
    );
}

#[frost_fn(name = "difference")]
fn difference(args: i64, ctx: i64) -> i64 {
    args - ctx
}

#[test]
fn parameters_named_like_the_generated_code_s_locals_are_their_own() {
    assert_eq!(run("f", difference_frost(), "f(5, 3)"), Value::Int(2));
    assert_eq!(
        raises("f", difference_frost(), "f(5, 'x')"),
        "Function difference requires Int as argument 2 (ctx), got String"
    );
}

// ============================================================
// Parameters
// ============================================================

#[frost_fn(name = "repeat_text")]
fn repeat_text(#[frost(unnamed)] text: String, times: usize) -> String {
    text.repeat(times)
}

/// Unnamed, a parameter's pattern need not give a name.
#[frost_fn(name = "sum_after_first")]
fn sum_after_first(#[frost(unnamed)] _: i64, #[frost(unnamed)] Rest(ns): Rest<i64>) -> i64 {
    ns.iter().sum()
}

#[test]
fn an_unnamed_parameter_is_reported_by_position_alone() {
    assert_eq!(
        raises("f", repeat_text_frost(), "f(1, 2)"),
        "Function repeat_text requires String as argument 1, got Int"
    );
    assert_eq!(
        raises("f", repeat_text_frost(), "f('a', 'b')"),
        "Function repeat_text requires Int as argument 2 (times), got String"
    );
    assert_eq!(
        run("f", sum_after_first_frost(), "f(100, 1, 2)"),
        Value::Int(3)
    );
}

#[frost_fn(name = "appended")]
fn appended(mut items: Vec<i64>, item: i64) -> Vec<i64> {
    items.push(item);
    items
}

#[test]
fn a_mutable_parameter_works() {
    assert_eq!(
        run("f", appended_frost(), "f([1, 2], 3)"),
        Value::array([1, 2, 3])
    );
}

#[frost_fn(name = "nothing")]
fn nothing() -> i64 {
    0
}

#[test]
fn a_function_without_parameters_takes_no_arguments() {
    assert_eq!(run("f", nothing_frost(), "f()"), Value::Int(0));
    assert_eq!(
        raises("f", nothing_frost(), "f(1)"),
        "Function nothing expects 0 arguments, but was called with 1"
    );
}

type Text = String;

mod reexports {
    pub(crate) use frostlang::native::Rest as Many;
}

use frostlang::native::Optional as Maybe;

/// Parameter types named through an alias and a renamed import.
#[frost_fn(name = "aliased")]
fn aliased(text: Text, times: Maybe<u8>) -> Value {
    Value::array([
        Value::from(text),
        times
            .0
            .map_or(Value::Null, |times| Value::Int(times.into())),
    ])
}

/// A parameter type named through a renamed re-export.
#[frost_fn(name = "reexported")]
fn reexported(rest: reexports::Many<i64>) -> i64 {
    rest.0.len() as i64
}

#[test]
fn parameter_types_are_known_by_their_traits_however_they_are_named() {
    assert_eq!(
        run("f", aliased_frost(), "f('a')"),
        Value::array([Value::from("a"), Value::Null])
    );
    assert_eq!(
        run("f", aliased_frost(), "f('a', 2)"),
        Value::array([Value::from("a"), Value::Int(2)])
    );
    assert_eq!(
        raises("f", aliased_frost(), "f(1)"),
        "Function aliased requires String as argument 1 (text), got Int"
    );
    assert_eq!(run("f", reexported_frost(), "f(7, 8)"), Value::Int(2));
    assert_eq!(
        raises("f", reexported_frost(), "f(1, 'x')"),
        "Function reexported requires Int as argument 2 (rest), got String"
    );
}

// ============================================================
// Results
// ============================================================

#[frost_fn(name = "unit")]
fn unit() {}

#[frost_fn(name = "maybe")]
fn maybe(some: bool) -> Option<i64> {
    some.then_some(1)
}

#[frost_fn(name = "fallible")]
fn fallible(fail: bool) -> Result<i64, FrostError> {
    if fail {
        Err(FrostError::from_static("it failed"))
    } else {
        Ok(1)
    }
}

#[frost_fn(name = "ratio")]
fn ratio(a: f64, b: f64) -> f64 {
    a / b
}

#[test]
fn the_result_converts_through_into_native_result() {
    assert_eq!(
        run("f", unit_frost(), "f()"),
        Value::Null,
        "nothing is Null"
    );
    assert_eq!(run("f", maybe_frost(), "f(true)"), Value::Int(1));
    assert_eq!(run("f", maybe_frost(), "f(false)"), Value::Null);
    assert_eq!(run("f", fallible_frost(), "f(false)"), Value::Int(1));
    assert_eq!(raises("f", fallible_frost(), "f(true)"), "it failed");
    assert_eq!(
        raises("f", ratio_frost(), "f(1, 0)"),
        f64::INFINITY
            .into_result("ratio")
            .expect_err("a non-finite f64 is an error")
            .message(),
        "the conversion's error, naming the function"
    );
}

// ============================================================
// The function as written
// ============================================================

#[test]
fn the_function_stays_callable_from_rust() {
    assert_eq!(pad("ab".to_string(), 4, Optional(None)), "  ab");
    assert_eq!(sum(Rest(vec![1, 2])), 3);
}

mod inner {
    use frostlang::frost_fn;

    #[frost_fn(name = "inner")]
    pub(crate) fn visible() -> i64 {
        1
    }
}

#[test]
fn the_maker_has_the_function_s_visibility() {
    assert_eq!(run("f", inner::visible_frost(), "f()"), Value::Int(1));
}

// Were the maker not configured out with the function, it would call a
// function that does not exist, and this file would not compile.
#[frost_fn(name = "absent")]
#[cfg(any())]
fn absent() {}

// And before the attribute, which rustc applies first, removing the attribute
// too.
#[cfg(any())]
#[frost_fn(name = "also_absent")]
fn also_absent() {}

// Always configured in, as a test.
#[frost_fn(name = "present")]
#[cfg(test)]
fn present() -> i64 {
    1
}

#[test]
fn the_maker_is_configured_with_the_function() {
    assert_eq!(run("f", present_frost(), "f()"), Value::Int(1));
}
