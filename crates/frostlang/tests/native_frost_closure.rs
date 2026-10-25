//! `frost_closure!`: Frost functions made from Rust closures.
//!
//! The first section checks generated functions against equivalents built by
//! hand in `native_args`, so that the macro is shown to generate the same
//! functions. The rest covers what only this form decides: captures, the
//! body's control flow, runtime names, generic factories, and parameters
//! written without a type.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use frostlang::native::{
    FromArg, FrostArg, Function, IntoNativeResult, Nullable, Optional, Rest, Structured,
};
use frostlang::{FrostError, FrostType, Value, frost_closure};

use crate::native_args;
use crate::script::Script;

/// Asserts that a run of each of `sources`, with `name` bound to the
/// generated function, ends exactly as with the hand-built one.
fn assert_alike(name: &str, generated: &Value, hand_built: &Value, sources: &[&str]) {
    for source in sources {
        let outcome = |function: &Value| {
            Script::new(source)
                .capture(name, function.clone())
                .outcome()
        };
        assert_eq!(
            outcome(generated),
            outcome(hand_built),
            "{source:?} ends alike with either function"
        );
    }
}

fn run(function: Value, source: &str) -> Value {
    Script::new(source).capture("f", function).run()
}

fn raises(function: Value, source: &str) -> String {
    Script::new(source).capture("f", function).raises()
}

// ============================================================
// Alike to hand-built functions
// ============================================================

#[test]
fn required_and_optional_nullable_parameters_are_alike() {
    let pad = frost_closure!(
        "pad",
        |text: String, width: u16, fill: Optional<Nullable<String>>| {
            let fill = match fill {
                Optional(None) => " ".to_string(),
                Optional(Some(Nullable(None))) => return text,
                Optional(Some(Nullable(Some(fill)))) => fill,
            };
            let missing = usize::from(width).saturating_sub(text.chars().count());
            format!("{}{text}", fill.repeat(missing))
        }
    );
    assert_alike(
        "pad",
        &pad,
        &native_args::pad(),
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

#[test]
fn an_interior_optional_and_the_context_are_alike() {
    let each =
        frost_closure!("each", |#[frost(ctx)] ctx,
                                sql: String,
                                bindings: Optional<Structured>,
                                callback: Function| {
            let count = match bindings.0 {
                None => Value::Null,
                Some(Structured::Array(array)) => Value::Int(array.len() as i64),
                Some(Structured::Map(map)) => Value::Int(map.len() as i64),
            };
            ctx.invoke(&callback, [Value::from(sql), count])
        });
    assert_alike(
        "each",
        &each,
        &native_args::each(),
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

#[test]
fn a_rest_parameter_is_alike() {
    let sum = frost_closure!("sum", |ns: Rest<i64>| ns.0.iter().sum::<i64>());
    assert_alike(
        "sum",
        &sum,
        &native_args::sum(),
        &["sum()", "sum(5)", "sum(1, 2, 3)", "sum(1, 2, 'x')"],
    );
}

// ============================================================
// Captures and control flow
// ============================================================

#[test]
fn the_closure_captures_by_move() {
    let greeting = String::from("hello");
    let greet = frost_closure!("greet", move |name: String| format!("{greeting}, {name}"));
    assert_eq!(run(greet, "f('Ada')"), Value::from("hello, Ada"));
}

#[test]
fn shared_state_is_captured_as_any_closure_captures_it() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = {
        let calls = Arc::clone(&calls);
        frost_closure!("counted", move || calls.fetch_add(1, Ordering::Relaxed)
            as i64)
    };
    // Only one optimization permutation, so that the count is exact.
    let optimization = crate::script::optimization_permutations()
        .next()
        .expect("there is always one permutation");
    let script = Script::new("[f(), f(), f()]").capture("f", counted);
    assert_eq!(script.run_under(optimization), Value::array([0, 1, 2]));
    assert_eq!(calls.load(Ordering::Relaxed), 3);
}

#[test]
fn the_closure_may_capture_names_the_generated_code_uses() {
    // Each of these names a local of the generated code too; the closure sees
    // only its own.
    let (name, params, function, ctx, args, result) = (1, 2, 3, 4, 5, 6);
    let sum = frost_closure!("sum", move || name
        + params
        + function
        + ctx
        + args
        + result);
    assert_eq!(run(sum, "f()"), Value::Int(21));
}

#[test]
fn return_and_question_mark_return_from_the_closure() {
    let checked = frost_closure!("checked", |n: i64| -> Result<i64, FrostError> {
        if n < 0 {
            return Err(FrostError::from_static("negative"));
        }
        let halved = u8::try_from(n).map_err(|_| FrostError::from_static("too big"))?;
        Ok(i64::from(halved) / 2)
    });
    assert_eq!(run(checked.clone(), "f(9)"), Value::Int(4));
    assert_eq!(raises(checked.clone(), "f(-1)"), "negative");
    assert_eq!(raises(checked, "f(300)"), "too big");
}

// ============================================================
// Names
// ============================================================

/// A function of no arguments returning `value`, named `name` at runtime.
fn constant(name: &'static str, value: i64) -> Value {
    frost_closure!(name, move || value)
}

#[test]
fn the_name_is_given_at_runtime() {
    assert_eq!(
        raises(constant("one", 1), "f(0)"),
        "Function one expects 0 arguments, but was called with 1"
    );
    assert_eq!(
        raises(constant("two", 2), "f(0)"),
        "Function two expects 0 arguments, but was called with 1"
    );
}

#[test]
fn the_name_is_evaluated_once_before_the_closure() {
    let evaluations = AtomicUsize::new(0);
    #[expect(clippy::useless_vec, reason = "not `Copy`, so the closure moves it")]
    let names = vec!["first", "second"];
    let function = frost_closure!(
        {
            evaluations.fetch_add(1, Ordering::Relaxed);
            names[0]
        },
        // Moves `names`, which the name has already used.
        move || names.len() as i64,
    );
    assert_eq!(evaluations.load(Ordering::Relaxed), 1);
    assert_eq!(
        raises(function.clone(), "f(0)"),
        "Function first expects 0 arguments, but was called with 1"
    );
    assert_eq!(run(function, "[f(), f()]"), Value::array([2, 2]));
    assert_eq!(evaluations.load(Ordering::Relaxed), 1, "not per call");
}

#[test]
fn the_context_knows_the_name() {
    let whoami = frost_closure!("whoami", |#[frost(ctx)] ctx| ctx.name().to_string());
    assert_eq!(run(whoami, "f()"), Value::from("whoami"));
}

#[test]
fn parameters_are_named_without_a_raw_prefix_and_never_shadow_generated_locals() {
    let difference = frost_closure!("difference", |args: i64, r#type: i64| args - r#type);
    assert_eq!(run(difference.clone(), "f(5, 3)"), Value::Int(2));
    assert_eq!(
        raises(difference, "f(5, 'x')"),
        "Function difference requires Int as argument 2 (type), got String"
    );
}

// ============================================================
// Parameters
// ============================================================

#[test]
fn a_parameter_without_a_type_accepts_any_value() {
    let describe = frost_closure!("describe", |value| value.type_name());
    assert_eq!(run(describe.clone(), "f(1)"), Value::from("Int"));
    assert_eq!(run(describe.clone(), "f([])"), Value::from("Array"));
    assert_eq!(run(describe.clone(), "f(null)"), Value::from("Null"));
    assert_eq!(
        raises(describe, "f()"),
        "Function describe expects 1 argument, but was called with 0"
    );
}

#[test]
fn a_parameter_without_a_type_takes_a_types_constraint() {
    let describe = frost_closure!("describe", |#[frost(types = Numeric)] n| n.type_name());
    assert_eq!(run(describe.clone(), "f(2)"), Value::from("Int"));
    assert_eq!(run(describe.clone(), "f(2.5)"), Value::from("Float"));
    assert_eq!(
        raises(describe, "f('2')"),
        "Function describe requires Numeric as argument 1 (n), got String"
    );
}

#[test]
fn typed_and_untyped_parameters_mix() {
    let tag = frost_closure!("tag", |label: String, value| Value::array([
        label.into(),
        value
    ]));
    assert_eq!(
        run(tag.clone(), "f('x', 1)"),
        Value::array([Value::from("x"), Value::Int(1)])
    );
    assert_eq!(
        raises(tag, "f(1, 1)"),
        "Function tag requires String as argument 1 (label), got Int"
    );
}

#[test]
fn the_context_may_be_typed_or_not() {
    let typed = frost_closure!(
        "typed",
        |#[frost(ctx)] ctx: &mut frostlang::NativeCtx<'_>, function: Function| {
            ctx.invoke(&function, [])
        }
    );
    let untyped = frost_closure!("untyped", |#[frost(ctx)] ctx, function: Function| {
        ctx.invoke(&function, [])
    });
    assert_eq!(run(typed, "f(fn -> 7)"), Value::Int(7));
    assert_eq!(run(untyped, "f(fn -> 7)"), Value::Int(7));
}

#[test]
fn a_mutable_parameter_works() {
    let appended = frost_closure!("appended", |mut items: Vec<i64>, item: i64| {
        items.push(item);
        items
    });
    assert_eq!(run(appended, "f([1, 2], 3)"), Value::array([1, 2, 3]));
}

#[test]
fn an_unnamed_parameter_is_reported_by_position_alone() {
    // Unnamed, a parameter's pattern need not give a name.
    let sum_after_first = frost_closure!("sum_after_first", |#[frost(unnamed)] _: i64,
                                                             #[frost(unnamed)] Rest(ns): Rest<
        i64,
    >| ns.iter().sum::<i64>());
    assert_eq!(run(sum_after_first, "f(100, 1, 2)"), Value::Int(3));
    let repeat = frost_closure!("repeat", |#[frost(unnamed)] text: String, times: usize| {
        text.repeat(times)
    });
    assert_eq!(
        raises(repeat.clone(), "f(1, 2)"),
        "Function repeat requires String as argument 1, got Int"
    );
    assert_eq!(
        raises(repeat, "f('a', 'b')"),
        "Function repeat requires Int as argument 2 (times), got String"
    );
}

/// The first of `items`, or Null when there are none, for any element type.
fn first<T>(name: &'static str) -> Value
where
    T: FromArg + IntoNativeResult + 'static,
{
    frost_closure!(name, |items: Vec<T>| Nullable(items.into_iter().next()))
}

#[test]
fn a_generic_factory_makes_a_function_per_type() {
    assert_eq!(run(first::<i64>("first_int"), "f([4, 5])"), Value::Int(4));
    assert_eq!(run(first::<String>("first_string"), "f([])"), Value::Null);
    assert_eq!(
        raises(first::<String>("first_string"), "f([1])"),
        "Function first_string requires String as element 0 of argument 1 (items), got Int"
    );
    assert_eq!(
        <Vec<String> as FrostArg>::PARAM.types(),
        FrostType::ARRAY,
        "the generic parameter's spec is its type's"
    );
}

#[test]
fn a_trailing_comma_is_accepted() {
    let one = frost_closure!("one", || 1,);
    assert_eq!(run(one, "f()"), Value::Int(1));
}

// ============================================================
// Results
// ============================================================

#[test]
fn the_result_converts_through_into_native_result() {
    let unit = frost_closure!("unit", || {});
    let maybe = frost_closure!("maybe", |some: bool| Nullable(some.then_some(1)));
    let ratio = frost_closure!("ratio", |a: f64, b: f64| a / b);
    assert_eq!(run(unit, "f()"), Value::Null, "nothing is Null");
    assert_eq!(run(maybe.clone(), "f(true)"), Value::Int(1));
    assert_eq!(run(maybe, "f(false)"), Value::Null);
    assert_eq!(
        raises(ratio, "f(1, 0)"),
        f64::INFINITY
            .into_result("ratio")
            .expect_err("a non-finite f64 is an error")
            .message(),
        "the conversion's error, naming the function"
    );
}
