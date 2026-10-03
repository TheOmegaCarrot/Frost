//! A `Repl` through its public API: each input runs against the bindings of
//! those before it, and a failed input changes nothing.

use std::error::Error;
use std::io;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use frost_compile::{CompilerErrors, OptimizationOptions};
use frost_repl::{Frontend, InvalidName, Repl, ReplError, ScriptedFrontend, check_name};
use frost_runtime::{Extension, FrostError, ImporterBuilder, Value, VmRuntimeConfiguration};

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

// --- Seeded bindings ---

#[test]
fn a_seeded_binding_is_bound_as_if_an_input_had_defined_it() -> Result<(), InvalidName> {
    let mut repl = Repl::new()
        .with_binding("x", Value::Int(20))?
        .with_binding("id", Value::from("seeded"))?;
    // Seen by inputs, shadowing the global `id`, and listed.
    assert_eq!(
        repl.evaluate("[x + 1, id]").unwrap(),
        Value::array([Value::Int(21), Value::from("seeded")])
    );
    let bound: Vec<&str> = repl.bindings().map(|(name, _)| name).collect();
    assert_eq!(bound, ["id", "x"]);
    // Rebound like any binding.
    repl.evaluate("def x = x * 2").unwrap();
    assert_eq!(repl.evaluate("x").unwrap(), Value::Int(40));
    Ok(())
}

#[test]
fn a_seeded_binding_survives_a_failed_input() -> Result<(), InvalidName> {
    let mut repl = Repl::new().with_binding("x", Value::Int(1))?;
    assert!(repl.evaluate("def x = 2; 1 / 0").is_err());
    assert_eq!(repl.evaluate("x").unwrap(), Value::Int(1));
    Ok(())
}

#[test]
fn seeding_several_bindings_keeps_a_repeated_names_last_value() -> Result<(), InvalidName> {
    let mut repl = Repl::new().with_bindings([
        ("a", Value::Int(1)),
        ("b", Value::Int(2)),
        ("a", Value::Int(3)),
    ])?;
    assert_eq!(repl.evaluate("[a, b]").unwrap(), Value::array([3, 2]));
    Ok(())
}

#[test]
fn a_name_frost_source_cannot_refer_to_cannot_be_seeded() {
    for name in [
        "",
        "if",
        "def",
        "and",
        "init",
        "$",
        "$1",
        "1x",
        "my-name",
        "two words",
        " x",
        "x ",
        "x.y",
        "'x'",
    ] {
        assert_eq!(
            check_name(name).map_err(|invalid| invalid.name().to_string()),
            Err(name.to_string()),
            "{name:?}"
        );
        let refused = Repl::new().with_binding(name, Value::Null).err();
        assert_eq!(
            refused.as_ref().map(InvalidName::name),
            Some(name),
            "{name:?}"
        );
        let refused = Repl::new()
            .with_bindings([("fine", Value::Null), (name, Value::Null)])
            .err();
        assert_eq!(
            refused.as_ref().map(InvalidName::name),
            Some(name),
            "{name:?}"
        );
    }
}

#[test]
fn any_identifier_can_be_seeded() {
    for name in [
        "x",
        "_",
        "_private",
        "snake_case_2",
        "CamelCase",
        "iffy",
        "define",
    ] {
        assert_eq!(check_name(name), Ok(()), "{name:?}");
    }
}

#[test]
fn a_refused_name_explains_itself() {
    let refused = check_name("if").unwrap_err();
    assert_eq!(
        refused.to_string(),
        "`if` is not a name Frost source can refer to"
    );
}

// --- Recent results ---

/// Evaluate each of `inputs` in turn on `repl`, returning the last value.
/// Every input must succeed.
fn value_on(mut repl: Repl, inputs: &[&str]) -> Value {
    let mut last = Value::Null;
    for input in inputs {
        last = repl
            .evaluate(input)
            .unwrap_or_else(|error| panic!("{input:?} failed: {error:?}"));
    }
    last
}

#[test]
fn results_holds_recent_values_oldest_first() {
    assert_eq!(value_of(&["1", "'two'", "results"]), value("[1, 'two']"));
    assert_eq!(value_of(&["1", "2", "results[-1]"]), Value::Int(2));
}

#[test]
fn results_starts_empty() {
    assert_eq!(value("results"), value("[]"));
}

#[test]
fn results_keeps_the_last_five_by_default() {
    let inputs = ["1", "2", "3", "4", "5", "6", "7", "results"];
    assert_eq!(value_of(&inputs), value("[3, 4, 5, 6, 7]"));
}

#[test]
fn results_keeps_neither_null_nor_failures() {
    let mut repl = Repl::new();
    for input in ["1", "def x = 2", "null", "1 / 0", "nope"] {
        let _ = repl.evaluate(input);
    }
    assert_eq!(repl.evaluate("results").unwrap(), value("[1]"));
}

#[test]
fn a_value_of_results_is_itself_kept() {
    assert_eq!(value_of(&["1", "results", "results"]), value("[1, [1]]"));
}

#[test]
fn how_many_results_are_kept_can_be_set() {
    let inputs = ["1", "2", "3", "results"];
    for (count, expected) in [(1, "[3]"), (2, "[2, 3]"), (10, "[1, 2, 3]")] {
        assert_eq!(
            value_on(Repl::new().with_results_kept(count), &inputs),
            value(expected),
            "keeping {count}"
        );
    }
}

#[test]
fn keeping_no_results_binds_no_results() {
    let mut repl = Repl::new().with_results_kept(0);
    repl.evaluate("1").unwrap();
    let Err(ReplError::Compile(errors)) = repl.evaluate("results") else {
        panic!("`results` should not be bound");
    };
    let rendered = errors.render_plain();
    assert!(rendered.contains("`results` is not defined"), "{rendered}");
}

#[test]
fn binding_results_takes_the_name_for_good() {
    let inputs = ["1", "def results = 'mine'", "2", "3", "results"];
    assert_eq!(value_of(&inputs), Value::from("mine"));
    // The new binding may be made from the old; either way, no more are kept.
    let inputs = ["1", "def results = results[-1]", "5", "results"];
    assert_eq!(value_of(&inputs), Value::Int(1));
    // However it is bound.
    let inputs = ["1", "def [results] = [9]", "2", "results"];
    assert_eq!(value_of(&inputs), Value::Int(9));
}

#[test]
fn a_failed_input_binding_results_leaves_it_kept() {
    let mut repl = Repl::new();
    for input in ["1", "def results = 2; 1 / 0", "def results = nope"] {
        let _ = repl.evaluate(input);
    }
    assert_eq!(repl.evaluate("results").unwrap(), value("[1]"));
}

#[test]
fn seeding_results_takes_the_name_from_the_start() -> Result<(), InvalidName> {
    let repl = Repl::new().with_binding("results", Value::from("seeded"))?;
    assert_eq!(value_on(repl, &["1", "results"]), Value::from("seeded"));
    Ok(())
}

#[test]
fn bindings_lists_results_only_once_an_input_binds_it() {
    let mut repl = Repl::new();
    repl.evaluate("1").unwrap();
    assert_eq!(repl.bindings().count(), 0);
    repl.evaluate("def results = 2").unwrap();
    let bindings: Vec<(&str, &Value)> = repl.bindings().collect();
    assert_eq!(bindings, [("results", &Value::Int(2))]);
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

/// The outcomes a session over `segments` rendered, in order.
fn session(segments: &[&str]) -> Vec<Result<Value, ReplError>> {
    let mut frontend = ScriptedFrontend::new(segments.iter().copied());
    Repl::new()
        .run(&mut frontend)
        .expect("a scripted frontend never fails");
    frontend.transcript().outcomes()
}

#[test]
fn a_session_renders_every_value_in_order_null_included() {
    let values: Vec<Value> = session(&["1 + 2", "def x = 1", "null", "[x]"])
        .into_iter()
        .map(|outcome| outcome.expect("every input succeeds"))
        .collect();
    assert_eq!(
        values,
        [Value::Int(3), Value::Null, Value::Null, Value::array([1])]
    );
}

#[test]
fn a_session_renders_failures_and_carries_on() {
    let outcomes = session(&["def x = 1", "nope", "1 / 0", "x + 1"]);
    assert!(
        matches!(
            outcomes.as_slice(),
            [
                Ok(Value::Null),
                Err(ReplError::Compile(_)),
                Err(ReplError::Run(_)),
                Ok(Value::Int(2)),
            ]
        ),
        "{outcomes:?}"
    );
}

#[test]
fn a_session_ends_when_input_does() {
    assert!(session(&[]).is_empty());
}

#[test]
fn a_sessions_prints_go_to_the_sink_not_the_frontend() {
    let printed = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let printed = Arc::clone(&printed);
        move |text: &str| printed.lock().unwrap().push(text.to_string())
    };
    let config = VmRuntimeConfiguration {
        print_sink: Arc::new(sink),
        ..Default::default()
    };
    let mut frontend = ScriptedFrontend::new(["print('a'); 1"]);
    Repl::new()
        .with_configuration(config)
        .run(&mut frontend)
        .unwrap();
    assert_eq!(*printed.lock().unwrap(), ["a"]);
    let outcomes = frontend.transcript().outcomes();
    assert!(
        matches!(outcomes.as_slice(), [Ok(Value::Int(1))]),
        "{outcomes:?}"
    );
}

#[test]
fn a_session_stops_on_a_failure_to_read_input() {
    struct Broken;
    impl Frontend for Broken {
        fn read_segment(&mut self) -> io::Result<Option<String>> {
            Err(io::Error::other("input is gone"))
        }
        fn render(&mut self, _: Result<&Value, &ReplError>) -> io::Result<()> {
            panic!("nothing was read, so nothing should be rendered");
        }
    }
    let result = Repl::new().run(&mut Broken);
    assert_eq!(result.unwrap_err().to_string(), "input is gone");
}

#[test]
fn a_session_stops_on_a_failure_to_render() {
    /// Reads `1` for as long as it is asked, but cannot render.
    struct Mute {
        reads: usize,
    }
    impl Frontend for Mute {
        fn read_segment(&mut self) -> io::Result<Option<String>> {
            self.reads += 1;
            Ok(Some("1".to_string()))
        }
        fn render(&mut self, _: Result<&Value, &ReplError>) -> io::Result<()> {
            Err(io::Error::other("output is gone"))
        }
    }
    let mut frontend = Mute { reads: 0 };
    let result = Repl::new().run(&mut frontend);
    assert_eq!(result.unwrap_err().to_string(), "output is gone");
    assert_eq!(frontend.reads, 1, "the session should stop at the failure");
}

// --- How a failure shows ---

#[test]
fn a_compile_failure_displays_its_diagnostics_as_plain_text() {
    let error = evaluate_all(&["nope"]).unwrap_err();
    let ReplError::Compile(diagnostics) = &error else {
        panic!("should not compile, but gave {error:?}");
    };
    let shown = error.to_string();
    assert_eq!(shown, diagnostics.render_plain().trim_end());
    assert!(shown.contains("`nope` is not defined"), "{shown}");
    assert!(!shown.contains('\x1b'), "no color: {shown:?}");
}

#[test]
fn a_runtime_failure_displays_its_error_then_each_frame_of_its_backtrace() {
    let error = evaluate_all(&[
        "defn inner() -> [error('boom')]",
        "defn outer() -> [inner()]",
        "outer()",
    ])
    .unwrap_err();
    let ReplError::Run(raised) = &error else {
        panic!("should raise, but gave {error:?}");
    };
    // Which frames a backtrace holds is the VM's business; how they show is ours.
    let frames = raised.backtrace();
    assert!(frames.len() >= 2, "{frames:?}");
    let expected: String = frames
        .iter()
        .map(|frame| format!("\n  in {frame}"))
        .collect();
    assert_eq!(error.to_string(), format!("Error: boom{expected}"));
}

#[test]
fn a_failures_source_is_the_error_it_holds() {
    let compile = evaluate_all(&["nope"]).unwrap_err();
    assert!(
        compile.source().unwrap().is::<CompilerErrors>(),
        "{compile:?}"
    );
    let run = evaluate_all(&["1 / 0"]).unwrap_err();
    assert!(run.source().unwrap().is::<FrostError>(), "{run:?}");
}
