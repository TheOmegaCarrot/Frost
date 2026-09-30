//! A `Repl` through its public API: each input runs against the bindings of
//! those before it, and a failed input changes nothing.

use std::collections::VecDeque;
use std::io;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use frost_compile::OptimizationOptions;
use frost_repl::{Repl, ReplError, ReplInput};
use frost_runtime::{Extension, ImporterBuilder, Value, VmRuntimeConfiguration};

/// Evaluate each of `inputs` in turn on a fresh REPL, returning the last
/// value. Every input before the last must succeed.
fn evaluate_all(inputs: &[&str]) -> Result<Value, ReplError> {
    let mut repl = Repl::new();
    let (last, earlier) = inputs.split_last().expect("at least one input");
    for input in earlier {
        repl.evaluate(input)
            .unwrap_or_else(|error| panic!("{input:?} failed: {error:?}"));
    }
    repl.evaluate(last)
}

/// [`evaluate_all`], whose last input must succeed too.
fn value_of(inputs: &[&str]) -> Value {
    evaluate_all(inputs).unwrap_or_else(|error| panic!("{inputs:?} failed: {error:?}"))
}

/// The value of `source` run as a single input.
fn value(source: &str) -> Value {
    value_of(&[source])
}

/// The runtime error message of `inputs`' last input, which must raise one.
fn raises(inputs: &[&str]) -> String {
    match evaluate_all(inputs) {
        Err(ReplError::Run(error)) => error.message().into_owned(),
        other => panic!("{inputs:?} should raise, but gave {other:?}"),
    }
}

/// The rendered diagnostics of `inputs`' last input, which must not compile.
fn diagnostics(inputs: &[&str]) -> String {
    match evaluate_all(inputs) {
        Err(ReplError::Compile(errors)) => errors.render_plain(),
        other => panic!("{inputs:?} should not compile, but gave {other:?}"),
    }
}

// --- Values ---

#[test]
fn an_input_evaluates_to_its_last_expression() {
    assert_eq!(value("1 + 2"), Value::Int(3));
    assert_eq!(value("def x = 1; x + 1"), Value::Int(2));
    assert_eq!(value(r#""text""#), Value::from("text"));
}

#[test]
fn an_input_without_a_final_expression_evaluates_to_null() {
    for source in ["def x = 1", "", "  ", "# only a comment"] {
        assert_eq!(value(source), Value::Null, "{source:?}");
    }
}

// --- Bindings carry over ---

#[test]
fn a_binding_is_seen_by_every_later_input() {
    assert_eq!(
        value_of(&["def x = 20", "def y = 1", "x + y"]),
        Value::Int(21)
    );
    assert_eq!(value_of(&["def x = 20", "1", "2", "x"]), Value::Int(20));
}

#[test]
fn a_later_input_may_bind_a_name_again() {
    assert_eq!(value_of(&["def x = 1", "def x = 2", "x"]), Value::Int(2));
    // The new binding may be built from the old one.
    assert_eq!(
        value_of(&["def x = 1", "def x = x + 1", "def x = x * 10", "x"]),
        Value::Int(20)
    );
    // Of any type.
    assert_eq!(
        value_of(&["def x = 1", r#"def x = "one""#, "x"]),
        Value::from("one")
    );
}

#[test]
fn a_name_bound_twice_in_one_input_is_still_an_error() {
    let rendered = diagnostics(&["def x = 1; def x = 2"]);
    assert!(rendered.contains("`x` is already bound"), "{rendered}");
}

#[test]
fn a_function_carries_over_and_may_recurse() {
    let inputs = [
        "defn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)",
        "defn twice(f, x) -> f(f(x))",
        "twice(fact, 3)",
    ];
    assert_eq!(value_of(&inputs), Value::Int(720));
}

#[test]
fn a_function_keeps_the_values_it_was_defined_with() {
    // Rebinding `x` binds a new `x`; the function still holds the old one.
    let inputs = ["def x = 1", "defn get() -> x", "def x = 2", "[get(), x]"];
    assert_eq!(value_of(&inputs), Value::array([1, 2]));
}

#[test]
fn every_top_level_binding_is_kept_not_just_exported_ones() {
    let inputs = [
        "export def a = 1",
        "def [b, c] = [2, 3]",
        "def {d} = {d: 4}",
        "[a, b, c, d]",
    ];
    assert_eq!(value_of(&inputs), Value::array([1, 2, 3, 4]));
}

#[test]
fn a_binding_nested_in_a_block_is_not_kept() {
    let rendered = diagnostics(&["def y = do { def z = 1; z }", "z"]);
    assert!(rendered.contains("`z` is not defined"), "{rendered}");
}

#[test]
fn a_binding_shadows_a_global() {
    assert_eq!(value_of(&["def id = 5", "id"]), Value::Int(5));
}

#[test]
fn bindings_lists_every_name_bound_so_far() {
    let mut repl = Repl::new();
    repl.evaluate("def a = 1; def b = 2").unwrap();
    repl.evaluate("def a = 3").unwrap();
    let bindings: Vec<(&str, &Value)> = repl.bindings().collect();
    assert_eq!(bindings, [("a", &Value::Int(3)), ("b", &Value::Int(2))]);
}

// --- A failed input changes nothing ---

#[test]
fn an_input_that_does_not_compile_binds_nothing() {
    let mut repl = Repl::new();
    repl.evaluate("def x = 1").unwrap();
    assert!(matches!(
        repl.evaluate("def x = 2; def y = nope"),
        Err(ReplError::Compile(_))
    ));
    assert_eq!(repl.evaluate("x").unwrap(), Value::Int(1));
    assert!(matches!(repl.evaluate("y"), Err(ReplError::Compile(_))));
}

#[test]
fn an_input_that_raises_binds_nothing_even_what_it_bound_before_raising() {
    let mut repl = Repl::new();
    repl.evaluate("def x = 1").unwrap();
    assert!(matches!(
        repl.evaluate("def x = 2; def y = 3; 1 / 0"),
        Err(ReplError::Run(_))
    ));
    assert_eq!(repl.evaluate("x").unwrap(), Value::Int(1));
    assert!(matches!(repl.evaluate("y"), Err(ReplError::Compile(_))));
}

#[test]
fn inputs_run_normally_after_a_failure() {
    let mut repl = Repl::new();
    for failing in ["1 / 0", "nope", "error('boom')", "def"] {
        assert!(repl.evaluate(failing).is_err(), "{failing:?}");
        assert_eq!(
            repl.evaluate("def a = [1]; a + [2]").unwrap(),
            Value::array([1, 2])
        );
    }
}

#[test]
fn a_runtime_error_reports_its_message() {
    assert!(raises(&["1 / 0"]).contains("Division by zero"));
    assert_eq!(raises(&["def msg = 'boom'", "error(msg)"]), "boom");
}

// --- Configuration ---

#[test]
fn every_input_runs_under_the_configuration() {
    // Each input gets the whole budget: fuel does not carry over between inputs.
    let config = VmRuntimeConfiguration {
        fuel: NonZeroUsize::new(20),
        ..Default::default()
    };
    let mut repl = Repl::new().with_configuration(config);
    repl.evaluate("defn count(n) -> if n == 0: 0 else: count(n - 1)")
        .unwrap();
    for _ in 0..3 {
        assert_eq!(repl.evaluate("count(15)").unwrap(), Value::Int(0));
    }
    let Err(ReplError::Run(error)) = repl.evaluate("count(50)") else {
        panic!("the budget should run out");
    };
    assert!(error.message().contains("fuel"), "{}", error.message());
}

#[test]
fn print_goes_to_the_configured_sink() {
    let printed = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let printed = Arc::clone(&printed);
        move |text: &str| printed.lock().unwrap().push(text.to_string())
    };
    let config = VmRuntimeConfiguration {
        print_sink: Arc::new(sink),
        ..Default::default()
    };
    let mut repl = Repl::new().with_configuration(config);
    repl.evaluate("def x = 1").unwrap();
    repl.evaluate("print(x); print('two')").unwrap();
    assert_eq!(*printed.lock().unwrap(), ["1", "two"]);
}

#[test]
fn inputs_import_from_the_importer() {
    let importer = ImporterBuilder::new()
        .with_extension(Extension::new("answer", Value::Int(42)).unwrap())
        .unwrap()
        .build();
    let mut repl = Repl::new().with_importer(importer);
    repl.evaluate("def answer = import('ext.answer')").unwrap();
    assert_eq!(repl.evaluate("answer").unwrap(), Value::Int(42));
    // The default REPL can import nothing.
    assert!(matches!(
        Repl::new().evaluate("import('ext.answer')"),
        Err(ReplError::Run(_))
    ));
}

#[test]
fn inputs_compile_with_the_optimizations_chosen() {
    // Folded, this makes no calls at runtime; unfolded, it needs eleven.
    let foldable = "(fn f(n) -> if n == 0: 0 else: f(n - 1))(10)";
    let config = || VmRuntimeConfiguration {
        fuel: NonZeroUsize::new(5),
        ..Default::default()
    };
    let mut folding = Repl::new().with_configuration(config());
    assert!(folding.evaluate(foldable).is_ok());
    let mut plain = Repl::new()
        .with_configuration(config())
        .with_optimization(OptimizationOptions::NONE);
    assert!(matches!(plain.evaluate(foldable), Err(ReplError::Run(_))));
}

// --- A whole session ---

/// Input that hands over prepared segments, then ends.
struct Segments(VecDeque<String>);

impl Segments {
    fn of(segments: &[&str]) -> Self {
        Self(segments.iter().map(ToString::to_string).collect())
    }
}

impl ReplInput for Segments {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        Ok(self.0.pop_front())
    }
}

/// What a session over `segments` wrote: its output and its errors.
fn session(segments: &[&str]) -> (String, String) {
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    Repl::new()
        .run(&mut Segments::of(segments), &mut output, &mut errors)
        .unwrap();
    (
        String::from_utf8(output).unwrap(),
        String::from_utf8(errors).unwrap(),
    )
}

#[test]
fn a_session_pretty_prints_each_value() {
    let (output, errors) = session(&["1 + 2", "true", "[1, 'two', {k: null}]"]);
    let expected = r#"3
true
[
    1,
    "two",
    {
        k: null
    }
]
"#;
    assert_eq!(output, expected);
    assert_eq!(errors, "");
}

#[test]
fn a_session_writes_nothing_for_null() {
    // A `def`, a Null, and a call returning Null all write nothing.
    let (output, errors) = session(&["def x = 1", "null", "print(x)", "x"]);
    assert_eq!(output, "1\n");
    assert_eq!(errors, "");
}

#[test]
fn a_session_reports_failures_and_carries_on() {
    let (output, errors) = session(&["def x = 1", "nope", "1 / 0", "x + 1"]);
    assert_eq!(output, "2\n");
    assert!(errors.contains("`nope` is not defined"), "{errors}");
    assert!(errors.contains("Division by zero"), "{errors}");
}

#[test]
fn a_session_ends_when_input_does() {
    let (output, errors) = session(&[]);
    assert_eq!((output.as_str(), errors.as_str()), ("", ""));
}

#[test]
fn a_session_stops_on_a_failure_to_read_input() {
    struct Broken;
    impl ReplInput for Broken {
        fn read_segment(&mut self) -> io::Result<Option<String>> {
            Err(io::Error::other("input is gone"))
        }
    }
    let result = Repl::new().run(&mut Broken, &mut Vec::new(), &mut Vec::new());
    assert_eq!(result.unwrap_err().to_string(), "input is gone");
}
