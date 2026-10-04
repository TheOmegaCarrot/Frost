//! The REPL's own metacommands, run in sessions on a `ScriptedFrontend`.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use frost_compile::{Optimization, OptimizationOptions};
use frost_repl::{Frontend, Repl, ReplError, ScriptedFrontend, Transcript};
use frost_runtime::{Value, VmRuntimeConfiguration};

/// The transcript of a session over `segments` on `repl`, with text unstyled.
fn session_on(mut repl: Repl, segments: &[&str]) -> Transcript {
    let mut frontend = ScriptedFrontend::new(segments.iter().copied());
    repl.run(&mut frontend)
        .expect("a scripted frontend never fails");
    frontend.transcript()
}

/// The transcript of a session over `segments` on a fresh REPL.
fn session(segments: &[&str]) -> Transcript {
    session_on(Repl::new(), segments)
}

/// The one text `transcript` holds.
fn only_text(transcript: &Transcript) -> String {
    match transcript.texts().as_slice() {
        [text] => text.clone(),
        texts => panic!("expected one text, got {texts:?}"),
    }
}

/// Assert that `segments`' last segment fails with a message containing
/// `expected`, and that the session carries on to evaluate `1` after it.
fn assert_fails(segments: &[&str], expected: &str) {
    let mut with_next = segments.to_vec();
    with_next.push("1");
    let transcript = session(&with_next);
    let outcomes = transcript.outcomes();
    let [.., Err(ReplError::Run(error)), Ok(Value::Int(1))] = outcomes.as_slice() else {
        panic!("{segments:?} should fail, then the session carry on, but gave {outcomes:?}");
    };
    assert!(
        error.message().contains(expected),
        "{segments:?}: {}",
        error.message()
    );
}

// --- Any metacommand ---

#[test]
fn an_unknown_metacommand_fails_and_the_session_carries_on() {
    assert_fails(
        &[":nope"],
        "there is no metacommand `:nope`; `:help` lists them",
    );
}

#[test]
fn a_colon_not_right_before_a_name_is_frost_source() {
    for segment in [": help", " :help", ":"] {
        let outcomes = session(&[segment]).outcomes();
        assert!(
            matches!(outcomes.as_slice(), [Err(ReplError::Compile(_))]),
            "{segment:?} should be compiled, and fail, but gave {outcomes:?}"
        );
    }
}

#[test]
fn a_metacommand_keeps_no_result() {
    let transcript = session(&["1", ":bindings", ":help", "results"]);
    let outcomes = transcript.outcomes();
    assert!(
        matches!(outcomes.as_slice(), [Ok(Value::Int(1)), Ok(results)] if *results == Value::array([1])),
        "{outcomes:?}"
    );
}

#[test]
fn a_metacommand_that_takes_no_argument_refuses_one() {
    for name in ["help", "quit", "bindings"] {
        assert_fails(
            &[&format!(":{name} please")],
            &format!("`:{name}` takes no argument"),
        );
    }
}

// --- :help ---

#[test]
fn help_lists_each_built_in_with_its_arguments_and_summary() {
    let help = only_text(&session(&[":help"]));
    let expected = r":help                       List the metacommands
:quit                       End the session
:bindings                   List the names bound, but not globals
:undef <name>...            Remove bindings
:disassemble <source>       Show the bytecode source compiles to, without running it
:ast <source>               Show the syntax tree source parses to
:optimize [<setting>, ...]  Show the optimizations, or set them: `<name>=true|false`, `all`, or `none`";
    assert_eq!(help, expected);
}

// --- :quit ---

#[test]
fn quit_ends_the_session_reading_nothing_more() {
    let mut frontend = ScriptedFrontend::new(["1", ":quit", "2"]);
    Repl::new().run(&mut frontend).unwrap();
    let outcomes = frontend.transcript().outcomes();
    assert!(
        matches!(outcomes.as_slice(), [Ok(Value::Int(1))]),
        "{outcomes:?}"
    );
    assert_eq!(
        frontend.read_segment().unwrap().as_deref(),
        Some("2"),
        "what follows `:quit` is left unread"
    );
}

// --- :bindings ---

#[test]
fn bindings_lists_each_name_bound_with_its_type_in_name_order() {
    let transcript = session(&[
        "def zed = 'z'",
        "defn inc(x) -> x + 1",
        "def [a, b] = [1, 2.5]",
        ":bindings",
    ]);
    let expected = r"a        Int
b        Float
inc      Function
results  Array
zed      String";
    assert_eq!(only_text(&transcript), expected);
}

#[test]
fn bindings_shows_nothing_when_nothing_is_bound() {
    let transcript = session_on(Repl::new().with_results_kept(0), &[":bindings"]);
    assert!(transcript.texts().is_empty(), "{:?}", transcript.texts());
    assert!(
        transcript.outcomes().is_empty(),
        "{:?}",
        transcript.outcomes()
    );
}

#[test]
fn bindings_lists_results_once_an_input_has_taken_it() {
    let transcript = session(&["def results = 'mine'", ":bindings"]);
    assert_eq!(only_text(&transcript), "results  String");
}

#[test]
fn bindings_leaves_out_globals() {
    let transcript = session_on(
        Repl::new().with_results_kept(0),
        &["def x = len([1])", ":bindings"],
    );
    assert_eq!(only_text(&transcript), "x  Int");
}

// --- :undef ---

#[test]
fn undef_unbinds_each_name_given() {
    let transcript = session(&[
        "def a = 1",
        "def b = 2",
        "def c = 3",
        ":undef a  c",
        "b",
        "a",
        "c",
    ]);
    assert!(transcript.texts().is_empty(), "`:undef` shows nothing");
    let outcomes = transcript.outcomes();
    assert!(
        matches!(
            outcomes.as_slice(),
            [
                Ok(Value::Null),
                Ok(Value::Null),
                Ok(Value::Null),
                Ok(Value::Int(2)),
                Err(ReplError::Compile(_)),
                Err(ReplError::Compile(_)),
            ]
        ),
        "{outcomes:?}"
    );
}

#[test]
fn undef_may_take_names_across_lines() {
    let transcript = session(&["def a = 1", "def b = 2", ":undef a\nb", ":bindings"]);
    assert_eq!(only_text(&transcript), "results  Array");
}

#[test]
fn undef_of_a_name_not_bound_unbinds_nothing() {
    assert_fails(&["def a = 1", ":undef a nope"], "`nope` is not bound");
    let transcript = session(&["def a = 1", ":undef a nope", "a"]);
    let outcomes = transcript.outcomes();
    assert!(
        matches!(outcomes.last(), Some(Ok(Value::Int(1)))),
        "`a` should still be bound: {outcomes:?}"
    );
}

#[test]
fn undef_of_a_global_is_refused() {
    assert_fails(&[":undef len"], "`len` is not bound");
}

#[test]
fn undef_needs_a_name() {
    assert_fails(&[":undef"], "`:undef` needs the names to remove");
}

#[test]
fn undef_may_not_remove_the_repls_results() {
    assert_fails(
        &["1", ":undef results"],
        "`results` holds recent results, and cannot be removed",
    );
}

#[test]
fn undef_of_an_inputs_results_gives_the_name_back_to_the_repl() {
    let transcript = session(&[
        "1",
        "def results = 'mine'",
        ":undef results",
        "2",
        "results",
    ]);
    let outcomes = transcript.outcomes();
    assert!(
        matches!(outcomes.last(), Some(Ok(results)) if *results == Value::array([1, 2])),
        "{outcomes:?}"
    );
}

// --- :disassemble ---

#[test]
fn disassemble_shows_the_bytecode_without_styling_by_default() {
    let listing = only_text(&session(&[":disassemble 1 + 2"]));
    assert!(!listing.is_empty());
    assert!(!listing.contains('\x1b'), "{listing:?}");
    assert!(!listing.ends_with('\n'), "{listing:?}");
}

#[test]
fn disassemble_styles_the_bytecode_for_a_frontend_that_takes_ansi() {
    let mut frontend = ScriptedFrontend::new([":disassemble 1 + 2"]).with_ansi_styling(true);
    Repl::new().run(&mut frontend).unwrap();
    let listing = only_text(&frontend.transcript());
    assert!(listing.contains('\x1b'), "{listing:?}");
}

#[test]
fn disassemble_does_not_run_the_source() {
    let printed = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let printed = Arc::clone(&printed);
        move |text: &str| printed.lock().unwrap().push(text.to_string())
    };
    let configuration = VmRuntimeConfiguration {
        print_sink: Arc::new(sink),
        ..Default::default()
    };
    let repl = Repl::new().with_configuration(configuration);
    let transcript = session_on(repl, &[":disassemble def x = 1; print(x)", "x"]);
    assert!(
        printed.lock().unwrap().is_empty(),
        "nothing should be printed"
    );
    let outcomes = transcript.outcomes();
    assert!(
        matches!(outcomes.as_slice(), [Err(ReplError::Compile(_))]),
        "`x` should not be bound: {outcomes:?}"
    );
}

#[test]
fn disassemble_compiles_in_the_scope_of_the_bindings() {
    // Each binding the source refers to is a capture, as when it is evaluated.
    let transcript = session(&["def foo = 42", "1", ":disassemble [foo + 10, results]"]);
    assert!(
        matches!(
            transcript.outcomes().as_slice(),
            [Ok(Value::Null), Ok(Value::Int(1))]
        ),
        "the source should compile: {:?}",
        transcript.outcomes()
    );
    let listing = only_text(&transcript);
    assert!(listing.contains("2 captures"), "{listing}");
    for name in ["foo", "results"] {
        let slot = listing
            .lines()
            .find(|line| line.split_whitespace().nth(1) == Some(name))
            .unwrap_or_else(|| panic!("`{name}` should have a slot:\n{listing}"));
        assert!(
            slot.ends_with("capture"),
            "`{name}` should be captured:\n{listing}"
        );
    }
}

#[test]
fn disassemble_of_source_that_does_not_compile_shows_its_diagnostics() {
    let outcomes = session(&[":disassemble nope + 1"]).outcomes();
    let [Err(ReplError::Compile(errors))] = outcomes.as_slice() else {
        panic!("should not compile, but gave {outcomes:?}");
    };
    let rendered = errors.render_plain();
    assert!(rendered.contains("`nope` is not defined"), "{rendered}");
}

#[test]
fn disassemble_needs_source() {
    assert_fails(
        &[":disassemble"],
        "`:disassemble` needs source to disassemble",
    );
}

// --- :ast ---

#[test]
fn ast_shows_the_syntax_tree() {
    let tree = only_text(&session(&[":ast 1 + 2"]));
    assert!(tree.contains("BinOp"), "{tree}");
    assert!(tree.contains("Literal"), "{tree}");
}

#[test]
fn ast_does_not_compile_or_run_the_source() {
    // An unbound name parses, though it would not compile.
    let transcript = session(&[":ast def x = nope", "x"]);
    assert_eq!(transcript.texts().len(), 1, "{:?}", transcript.outcomes());
    let outcomes = transcript.outcomes();
    assert!(
        matches!(outcomes.as_slice(), [Err(ReplError::Compile(_))]),
        "`x` should not be bound: {outcomes:?}"
    );
}

#[test]
fn ast_of_source_that_does_not_parse_shows_its_diagnostic() {
    let outcomes = session(&[":ast def"]).outcomes();
    let [Err(ReplError::Compile(errors))] = outcomes.as_slice() else {
        panic!("should not parse, but gave {outcomes:?}");
    };
    assert!(!errors.render_plain().is_empty());
}

#[test]
fn ast_needs_source() {
    assert_fails(&[":ast"], "`:ast` needs source to parse");
}

// --- :optimize ---

/// What `:optimize` shows after `segments`, on a REPL starting from
/// `options`.
fn optimizations_after(options: OptimizationOptions, segments: &[&str]) -> String {
    let mut segments = segments.to_vec();
    segments.push(":optimize");
    let transcript = session_on(Repl::new().with_optimization(options), &segments);
    let texts = transcript.texts();
    let outcomes = transcript.outcomes();
    assert!(
        outcomes.iter().all(Result::is_ok),
        "every segment should succeed: {outcomes:?}"
    );
    texts
        .last()
        .expect("`:optimize` shows the settings")
        .clone()
}

#[test]
fn optimize_shows_each_optimization_and_whether_it_is_on() {
    let expected = r"constant-fold          true
constant-propagate     true
branch-eliminate       true
capture-hoist          true
dead-store-eliminate   true
discard-eliminate      true
consume-locals         true
deduplicate-constants  true";
    assert_eq!(optimizations_after(OptimizationOptions::ALL, &[]), expected);
    let mut options = OptimizationOptions::NONE;
    options.set(Optimization::CaptureHoist, true);
    let shown = optimizations_after(options, &[]);
    assert!(shown.contains("capture-hoist          true"), "{shown}");
    assert!(shown.contains("constant-fold          false"), "{shown}");
}

#[test]
fn optimize_sets_an_optimization_on_or_off() {
    let off = optimizations_after(
        OptimizationOptions::ALL,
        &[":optimize constant-fold = false"],
    );
    let on = optimizations_after(
        OptimizationOptions::NONE,
        &[":optimize consume-locals=true"],
    );
    for &optimization in Optimization::ALL {
        let name = optimization.name();
        // Only the optimization set changes.
        let expected_off = optimization != Optimization::ConstantFold;
        let expected_on = optimization == Optimization::ConsumeLocals;
        assert!(line_says(&off, name, expected_off), "{off}");
        assert!(line_says(&on, name, expected_on), "{on}");
    }
}

/// Whether `shown` says `name` is `on`.
fn line_says(shown: &str, name: &str, on: bool) -> bool {
    shown
        .lines()
        .any(|line| line.split_whitespace().eq([name, &on.to_string()]))
}

#[test]
fn optimize_sets_a_preset() {
    let none = optimizations_after(OptimizationOptions::ALL, &[":optimize preset = none"]);
    let all = optimizations_after(OptimizationOptions::NONE, &[":optimize preset = all"]);
    for &optimization in Optimization::ALL {
        assert!(line_says(&none, optimization.name(), false), "{none}");
        assert!(line_says(&all, optimization.name(), true), "{all}");
    }
}

#[test]
fn optimize_applies_several_settings_left_to_right() {
    let shown = optimizations_after(
        OptimizationOptions::ALL,
        &[":optimize preset = none, constant-fold = true ,branch-eliminate=true"],
    );
    for &optimization in Optimization::ALL {
        let on = matches!(
            optimization,
            Optimization::ConstantFold | Optimization::BranchEliminate
        );
        assert!(line_says(&shown, optimization.name(), on), "{shown}");
    }
    // A preset after a setting overrides it.
    let shown = optimizations_after(
        OptimizationOptions::ALL,
        &[":optimize constant-fold = false, preset = all"],
    );
    assert!(line_says(&shown, "constant-fold", true), "{shown}");
}

#[test]
fn optimize_shows_nothing_when_setting() {
    let transcript = session(&[":optimize preset = none"]);
    assert!(transcript.texts().is_empty(), "{:?}", transcript.texts());
    assert!(
        transcript.outcomes().is_empty(),
        "{:?}",
        transcript.outcomes()
    );
}

#[test]
fn optimize_refuses_a_bad_setting_and_applies_none() {
    // `OptimizationOptions::with_settings` reads the settings; its own tests
    // cover what it refuses and why.
    for setting in [
        "constant-fold",
        "constant-fold = yes",
        "nope = true",
        "preset = some",
        "none,",
    ] {
        let invocation = format!(":optimize consume-locals = false, {setting}");
        let refusal = OptimizationOptions::ALL
            .with_settings(&format!("consume-locals = false, {setting}"))
            .expect_err(setting)
            .to_string();
        assert_fails(&[&invocation], &refusal);
        // The valid setting before the bad one was not applied either.
        let shown = optimizations_after(OptimizationOptions::ALL, &[]);
        let transcript = session(&[&invocation, ":optimize"]);
        assert_eq!(transcript.texts().last(), Some(&shown), "{setting:?}");
    }
}

#[test]
fn optimize_takes_a_bare_preset() {
    let none = optimizations_after(OptimizationOptions::ALL, &[":optimize none"]);
    let all = optimizations_after(OptimizationOptions::NONE, &[":optimize all"]);
    for &optimization in Optimization::ALL {
        assert!(line_says(&none, optimization.name(), false), "{none}");
        assert!(line_says(&all, optimization.name(), true), "{all}");
    }
}

#[test]
fn optimize_applies_to_every_later_input() {
    // Folded, `1 + 2` compiles to its value; unfolded, to an addition.
    let transcript = session(&[
        ":disassemble 1 + 2",
        ":optimize constant-fold = false",
        ":disassemble 1 + 2",
    ]);
    let [folded, unfolded] = transcript.texts().try_into().expect("two listings");
    assert!(!folded.contains("Add"), "{folded}");
    assert!(unfolded.contains("Add"), "{unfolded}");
    // Evaluating, too: unfolded, this needs more fuel than the budget.
    let configuration = VmRuntimeConfiguration {
        fuel: NonZeroUsize::new(5),
        ..Default::default()
    };
    let foldable = "(fn f(n) -> if n == 0: 0 else: f(n - 1))(10)";
    let transcript = session_on(
        Repl::new().with_configuration(configuration),
        &[foldable, ":optimize preset = none", foldable],
    );
    let outcomes = transcript.outcomes();
    assert!(
        matches!(
            outcomes.as_slice(),
            [Ok(Value::Int(0)), Err(ReplError::Run(_))]
        ),
        "{outcomes:?}"
    );
}
