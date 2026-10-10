use frostlang::compile::{CompilerOptions, OptimizationOptions, compile_program};
use frostlang::{FrostError, RunError, Value, Vm};

/// The error running `source` raises, which it must.
fn raised(source: &str) -> FrostError {
    let options = CompilerOptions::new().with_optimization(OptimizationOptions::NONE);
    let closure = compile_program("test.frst", source, options)
        .expect("the source compiles")
        .code
        .into_closure()
        .expect("the source captures nothing");
    Vm::factory()
        .build(closure)
        .run()
        .map_err(RunError::into_error)
        .expect_err("the source raises")
}

#[test]
fn display_shows_the_message_alone() {
    // No `Error:` label: that is the presenter's to add.
    let err = FrostError::from_static("division by zero");
    assert_eq!(err.to_string(), "division by zero");
}

#[test]
fn display_of_a_thrown_value_shows_the_value() {
    let err = FrostError::from_value(Value::from(vec![Value::Int(1), Value::Int(2)]));
    assert_eq!(err.to_string(), "[ 1, 2 ]");
}

#[test]
fn with_backtrace_shows_the_error_then_a_line_per_frame() {
    let err = raised(
        r"
        defn inner() -> [error('boom')]
        defn outer() -> [inner()]
        outer()
        ",
    );
    assert!(err.backtrace().len() >= 2, "{:?}", err.backtrace());
    let frames: String = err
        .backtrace()
        .iter()
        .map(|frame| format!("\n  in {frame}"))
        .collect();
    assert_eq!(err.with_backtrace().to_string(), format!("boom{frames}"));
    assert!(
        frames.contains("\n  in inner") && frames.contains("\n  in outer"),
        "{frames}"
    );
}

#[test]
fn with_backtrace_of_an_error_without_frames_is_just_the_error() {
    let err = FrostError::from_static("division by zero");
    assert_eq!(err.with_backtrace().to_string(), "division by zero");
}

#[test]
fn from_str() {
    let err: FrostError = "something went wrong".into();
    assert_eq!(err.message(), "something went wrong");
    assert!(err.backtrace().is_empty());
}

#[test]
fn from_string() {
    let err: FrostError = String::from("bad input").into();
    assert_eq!(err.message(), "bad input");
}
