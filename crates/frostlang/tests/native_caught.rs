//! `native::caught`: a call's outcome as the value `try_call` reports.

use crate::script::Script;

use frostlang::native::{Function, caught};
use frostlang::{FrostError, NativeCtx, Value, frost_closure};

#[test]
fn a_success_is_ok_with_its_value() {
    assert_eq!(
        caught(Ok(Value::Int(3))),
        Value::map([("ok", Value::Bool(true)), ("value", Value::Int(3))])
    );
    assert_eq!(
        caught(Ok(Value::Null)),
        Value::map([("ok", Value::Bool(true)), ("value", Value::Null)])
    );
}

#[test]
fn a_failure_is_not_ok_with_its_error_and_no_trace_of_its_own() {
    assert_eq!(
        caught(Err(FrostError::from_static("refused"))),
        Value::map([
            ("ok", Value::Bool(false)),
            ("error", Value::from("refused")),
            ("trace", Value::from(Vec::<Value>::new())),
        ])
    );
    // An error raised as a value is that value.
    let thrown = Value::map([("reason", Value::from("refused"))]);
    assert_eq!(
        caught(Err(FrostError::from_value(thrown.clone()))),
        Value::map([
            ("ok", Value::Bool(false)),
            ("error", thrown),
            ("trace", Value::from(Vec::<Value>::new())),
        ])
    );
}

#[test]
fn a_failure_raised_as_null_is_an_error_of_null() {
    assert_eq!(
        caught(Err(FrostError::from_value(Value::Null))),
        Value::map([
            ("ok", Value::Bool(false)),
            ("error", Value::Null),
            ("trace", Value::from(Vec::<Value>::new())),
        ])
    );
}

#[test]
fn a_trace_names_the_frames_an_error_passed_through_outermost_last() {
    let source = r"
        defn inner() -> assert(false, 'deep')
        # Not a tail call, which would leave no frame of its own.
        defn outer() -> [inner()]
        catching(outer).trace
    ";
    let trace = Script::new(source).capture("catching", catching()).run();
    assert_eq!(
        trace,
        Value::array([
            Value::from("assert"),
            Value::from("inner"),
            Value::from("outer"),
        ])
    );
}

/// The Frost function `catching(function)`, which calls `function` and
/// returns its outcome through [`caught`].
fn catching() -> Value {
    frost_closure!(
        "catching",
        |#[frost(ctx)] ctx: &mut NativeCtx<'_>, function: Function| {
            caught(ctx.invoke(&function, []))
        }
    )
}

#[test]
fn a_native_reports_an_outcome_exactly_as_try_call_does() {
    for callee in [
        "fn -> 42",
        "fn -> assert(false, 'refused')",
        "fn -> assert(false, {reason: 'refused'})",
        "fn inner() -> (fn outer() -> error('deep'))()",
        "fn -> [1][5]",
    ] {
        let source = format!(
            r"
            def callee = {callee}
            [catching(callee), try_call(callee)]
            "
        );
        let both = Script::new(&source).capture("catching", catching()).run();
        let Value::Array(both) = both else {
            panic!("the script returns an Array, not {both:?}");
        };
        assert_eq!(both.as_slice()[0], both.as_slice()[1], "{callee}");
    }
}
