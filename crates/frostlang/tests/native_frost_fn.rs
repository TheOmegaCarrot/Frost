//! `#[frost_fn]`: Frost functions made from Rust functions.
//!
//! The first section checks generated functions against equivalents built by
//! hand in `native_args`, so that the macro is shown to generate the same
//! functions. The rest covers what only the macro decides: names, the
//! context, return conversion, and how parameter types are named.

use frostlang::native::{
    Function, IntoNativeResult, NativeDef, Nullable, Numeric, Optional, Rest, Structured,
};
use frostlang::{FrostError, FrostResult, NativeCtx, Value, frost_fn};

use crate::native_args;
use crate::script::Script;

/// Asserts that a run of each of `sources`, with `name` bound to the
/// generated function, ends exactly as with the hand-built one.
fn assert_alike(
    name: &str,
    generated: fn() -> NativeDef,
    hand_built: fn() -> Value,
    sources: &[&str],
) {
    for source in sources {
        let outcome = |function: Value| Script::new(source).capture(name, function).outcome();
        assert_eq!(
            outcome(generated().build()),
            outcome(hand_built()),
            "{source:?} ends alike with either function"
        );
    }
}

/// Runs `source` with `name` bound to the function `definition` builds under
/// its base name.
fn run(name: &str, definition: NativeDef, source: &str) -> Value {
    Script::new(source).capture(name, definition.build()).run()
}

/// The message `source` raises with `name` bound to the function `definition`
/// builds under its base name.
fn raises(name: &str, definition: NativeDef, source: &str) -> String {
    Script::new(source)
        .capture(name, definition.build())
        .raises()
}

// ============================================================
// Alike to hand-built functions
// ============================================================

#[frost_fn]
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

#[frost_fn]
fn each(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    sql: String,
    bindings: Optional<Structured>,
    callback: Function,
) -> FrostResult {
    let count = match bindings.0 {
        None => Value::Null,
        Some(Structured::Array(array)) => Value::Int(array.len() as i64),
        Some(Structured::Map(map)) => Value::Int(map.len() as i64),
    };
    ctx.invoke(&callback, [Value::from(sql), count])
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

#[frost_fn]
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

#[frost_fn]
fn shout(text: String) -> String {
    text.to_uppercase()
}

#[frost_fn(name = "loud")]
fn yell(text: String) -> String {
    text.to_uppercase()
}

/// A Rust keyword, but not a Frost one.
#[frost_fn]
fn r#struct() -> i64 {
    1
}

#[test]
fn the_base_name_is_the_function_s_own_unless_given() {
    assert_eq!(shout_frost().base_name(), "shout");
    assert_eq!(yell_frost().base_name(), "loud");
    assert_eq!(struct_frost().base_name(), "struct", "less its raw prefix");
}

#[test]
fn a_function_built_alone_goes_by_its_base_name() {
    assert_eq!(
        raises("f", shout_frost(), "f(1)"),
        "Function shout requires String as argument 1 (text), got Int"
    );
    assert_eq!(
        raises("f", yell_frost(), "f(1)"),
        "Function loud requires String as argument 1 (text), got Int"
    );
    assert_eq!(
        raises("f", struct_frost(), "f(1)"),
        "Function struct expects 0 arguments, but was called with 1"
    );
}

#[test]
fn a_function_built_with_a_name_goes_by_it() {
    let given = shout_frost().build_named("text.shout");
    assert_eq!(
        Script::new("f(1)").capture("f", given).raises(),
        "Function text.shout requires String as argument 1 (text), got Int"
    );
    let owned = yell_frost().build_named(format!("text.{}", "yell"));
    assert_eq!(
        Script::new("f(1)").capture("f", owned).raises(),
        "Function text.yell requires String as argument 1 (text), got Int",
        "the name may be built at runtime"
    );
}

#[frost_fn]
fn whoami(#[frost(ctx)] ctx: &mut NativeCtx<'_>) -> String {
    ctx.name().to_string()
}

#[test]
fn the_context_knows_the_name() {
    assert_eq!(run("f", whoami_frost(), "f()"), Value::from("whoami"));
    let named = whoami_frost().build_named("who.am_i");
    assert_eq!(
        Script::new("f()").capture("f", named).run(),
        Value::from("who.am_i")
    );
}

#[frost_fn]
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

#[frost_fn]
fn difference(args: i64, ctx: i64, name: i64) -> i64 {
    args - ctx - name
}

#[test]
fn parameters_named_like_the_generated_code_s_locals_are_their_own() {
    assert_eq!(run("f", difference_frost(), "f(10, 3, 2)"), Value::Int(5));
    assert_eq!(
        raises("f", difference_frost(), "f(5, 'x', 1)"),
        "Function difference requires Int as argument 2 (ctx), got String"
    );
}

// ============================================================
// Parameters
// ============================================================

#[frost_fn]
fn repeat_text(#[frost(unnamed)] text: String, times: usize) -> String {
    text.repeat(times)
}

/// Unnamed, a parameter's pattern need not give a name.
#[frost_fn]
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

#[frost_fn]
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

#[frost_fn]
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
#[frost_fn]
fn aliased(text: Text, times: Maybe<u8>) -> Value {
    Value::array([
        Value::from(text),
        times
            .0
            .map_or(Value::Null, |times| Value::Int(times.into())),
    ])
}

/// A parameter type named through a renamed re-export.
#[frost_fn]
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

/// A `Value` constrained to the types its body handles.
#[frost_fn]
fn kind(#[frost(types = Flat | Array)] sequence: Value) -> &'static str {
    match sequence {
        Value::String(_) => "text",
        Value::Bytes(_) => "binary",
        Value::Array(_) => "array",
        other => unreachable!("type-checked, got {}", other.type_name()),
    }
}

/// Constraints narrowing a Rust type, on an optional parameter, and on an
/// unnamed rest parameter.
#[frost_fn]
fn narrowed(
    #[frost(types = Int)] n: Numeric,
    #[frost(types = Null | String)] label: Optional<Value>,
) -> Value {
    Value::array([Value::from(n), label.0.unwrap_or(Value::from("none"))])
}

#[frost_fn]
fn count_flags(#[frost(unnamed, types = Bool)] flags: Rest<Value>) -> i64 {
    flags.0.len() as i64
}

#[test]
fn a_types_constraint_admits_its_types() {
    assert_eq!(run("f", kind_frost(), "f('a')"), Value::from("text"));
    assert_eq!(run("f", kind_frost(), "f(x'61')"), Value::from("binary"));
    assert_eq!(run("f", kind_frost(), "f([])"), Value::from("array"));
    assert_eq!(
        run("f", narrowed_frost(), "f(1)"),
        Value::array([Value::Int(1), Value::from("none")])
    );
    assert_eq!(
        run("f", narrowed_frost(), "f(1, null)"),
        Value::array([Value::Int(1), Value::Null])
    );
    assert_eq!(
        run("f", narrowed_frost(), "f(1, 'a')"),
        Value::array([Value::Int(1), Value::from("a")])
    );
    assert_eq!(
        run("f", count_flags_frost(), "f(true, false)"),
        Value::Int(2)
    );
}

#[test]
fn a_types_constraint_rejects_other_types_as_its_parameter_s_types() {
    let cases = [
        (
            kind_frost(),
            "f(1)",
            "Function kind requires String or Bytes or Array as argument 1 (sequence), got Int",
        ),
        (
            kind_frost(),
            "f({})",
            "Function kind requires String or Bytes or Array as argument 1 (sequence), got Map",
        ),
        (
            narrowed_frost(),
            "f(1.5)",
            "Function narrowed requires Int as argument 1 (n), got Float",
        ),
        (
            narrowed_frost(),
            "f(1, 2)",
            "Function narrowed requires Null or String as argument 2 (label), got Int",
        ),
        (
            count_flags_frost(),
            "f(true, 1)",
            "Function count_flags requires Bool as argument 2, got Int",
        ),
    ];
    for (definition, source, message) in cases {
        assert_eq!(raises("f", definition, source), message, "{source:?}");
    }
}

// ============================================================
// Results
// ============================================================

#[frost_fn]
fn unit() {}

#[frost_fn]
fn maybe(some: bool) -> Nullable<i64> {
    Nullable(some.then_some(1))
}

#[frost_fn]
fn fallible(fail: bool) -> Result<i64, FrostError> {
    if fail {
        Err(FrostError::from_static("it failed"))
    } else {
        Ok(1)
    }
}

#[frost_fn]
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
    let conversion_error = |name: &str| {
        f64::INFINITY
            .into_result(name)
            .expect_err("a non-finite f64 is an error")
            .message()
            .to_string()
    };
    assert_eq!(
        raises("f", ratio_frost(), "f(1, 0)"),
        conversion_error("ratio"),
        "the conversion's error, naming the function"
    );
    let named = ratio_frost().build_named("calc.ratio");
    assert_eq!(
        Script::new("f(1, 0)").capture("f", named).raises(),
        conversion_error("calc.ratio"),
        "the conversion's error, naming the function by the name it was built with"
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

/// The maker is a `const fn`.
const PRESENT: NativeDef = present_frost();

#[test]
fn the_maker_runs_at_compile_time() {
    assert_eq!(PRESENT.base_name(), "present");
    assert_eq!(run("f", PRESENT, "f()"), Value::Int(1));
}

// Were the maker not configured out with the function, it would call a
// function that does not exist, and this file would not compile.
#[frost_fn]
#[cfg(any())]
fn absent() {}

// And before the attribute, which rustc applies first, removing the attribute
// too.
#[cfg(any())]
#[frost_fn]
fn also_absent() {}

// Always configured in, as a test.
#[frost_fn]
#[cfg(test)]
fn present() -> i64 {
    1
}

#[test]
fn the_maker_is_configured_with_the_function() {
    assert_eq!(run("f", present_frost(), "f()"), Value::Int(1));
}
