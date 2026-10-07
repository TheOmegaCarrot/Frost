//! The metacommand machinery: reading an invocation, a frontend's table of
//! metacommands, and how a session takes a frontend's metacommands in.

use std::collections::VecDeque;
use std::io;
use std::sync::LazyLock;

use frostlang::Value;
use frostlang_repl::{
    Frontend, Invocation, MetacommandError, MetacommandProblem, MetacommandSpec, MetacommandTable,
    Repl, ReplError, ScriptedFrontend, SessionError,
};

// --- Reading an invocation ---

#[test]
fn an_invocation_is_a_name_after_a_colon_then_its_argument() {
    for (segment, name, argument) in [
        (":help", "help", ""),
        (":undef x", "undef", "x"),
        (":undef  x   y  ", "undef", "x   y"),
        (":disassemble if x: 1", "disassemble", "if x: 1"),
        (":ast f(\n  1\n)", "ast", "f(\n  1\n)"),
        (":name\targument", "name", "argument"),
        (":clear-history", "clear-history", ""),
        // Not a name a metacommand could have, but read as one, to be refused.
        (":(1)", "(1)", ""),
    ] {
        let invocation =
            Invocation::parse(segment).unwrap_or_else(|| panic!("{segment:?} is a metacommand"));
        assert_eq!(invocation.name(), name, "{segment:?}");
        assert_eq!(invocation.argument(), argument, "{segment:?}");
    }
}

#[test]
fn only_a_colon_right_before_a_name_makes_a_metacommand() {
    for segment in [
        "", "help", " :help", "  :help", ": help", ":", ": ", ":\nhelp", "x :help",
    ] {
        assert_eq!(Invocation::parse(segment), None, "{segment:?}");
    }
}

// --- A frontend's table of metacommands ---

/// A frontend's state, for handlers to change.
#[derive(Default)]
struct Greeter {
    greeting: String,
    farewells: usize,
}

static GREETER: LazyLock<MetacommandTable<Greeter>> = LazyLock::new(|| {
    MetacommandTable::<Greeter>::new()
        .with(
            MetacommandSpec::new("greet", "Change the greeting").with_arguments("<greeting>"),
            |greeter, invocation| {
                greeter.greeting = invocation.argument().to_string();
                Ok(())
            },
        )
        .with(
            MetacommandSpec::new("farewell", "Say goodbye"),
            |greeter, _| {
                greeter.farewells += 1;
                Ok(())
            },
        )
        .with(MetacommandSpec::new("fail", "Fail"), |_, _| {
            Err(io::Error::other("failed on purpose"))
        })
});

/// The invocation `segment` makes, which must be one.
fn invocation(segment: &str) -> Invocation {
    Invocation::parse(segment).unwrap_or_else(|| panic!("{segment:?} is a metacommand"))
}

#[test]
fn a_table_lists_its_metacommands_in_the_order_added() {
    let names: Vec<String> = GREETER
        .specs()
        .iter()
        .map(|spec| spec.name().to_string())
        .collect();
    assert_eq!(names, ["greet", "farewell", "fail"]);
    let greet = &GREETER.specs()[0];
    assert_eq!(
        (greet.arguments(), greet.summary()),
        ("<greeting>", "Change the greeting")
    );
    assert_eq!(GREETER.specs()[1].arguments(), "", "no arguments described");
}

#[test]
fn a_table_runs_the_handler_for_the_name_invoked() {
    let mut greeter = Greeter::default();
    GREETER
        .dispatch(&mut greeter, &invocation(":greet good day"))
        .unwrap();
    GREETER
        .dispatch(&mut greeter, &invocation(":farewell"))
        .unwrap();
    GREETER
        .dispatch(&mut greeter, &invocation(":farewell"))
        .unwrap();
    assert_eq!(greeter.greeting, "good day");
    assert_eq!(greeter.farewells, 2);
}

#[test]
fn a_table_passes_on_a_handlers_failure() {
    let error = GREETER
        .dispatch(&mut Greeter::default(), &invocation(":fail"))
        .unwrap_err();
    assert_eq!(error.to_string(), "failed on purpose");
}

#[test]
fn a_table_refuses_a_name_it_does_not_have() {
    let error = GREETER
        .dispatch(&mut Greeter::default(), &invocation(":wave"))
        .unwrap_err();
    assert!(error.to_string().contains("`:wave`"), "{error}");
}

#[test]
fn an_empty_table_has_no_metacommands() {
    assert!(MetacommandTable::<Greeter>::new().specs().is_empty());
    assert!(MetacommandTable::<Greeter>::default().specs().is_empty());
}

// --- A frontend's metacommands in a session ---

/// A frontend with metacommands of its own. It reads prepared segments,
/// counting the reads, and records what it runs and is shown.
struct Console {
    segments: VecDeque<String>,
    specs: Vec<MetacommandSpec>,
    reads: usize,
    invoked: Vec<Invocation>,
    texts: Vec<String>,
    outcomes: Vec<Result<Value, ReplError>>,
}

impl Console {
    /// A console offering `specs`, reading `segments`.
    fn new(specs: Vec<MetacommandSpec>, segments: &[&str]) -> Self {
        Self {
            segments: segments.iter().map(ToString::to_string).collect(),
            specs,
            reads: 0,
            invoked: Vec::new(),
            texts: Vec::new(),
            outcomes: Vec::new(),
        }
    }
}

impl Frontend for Console {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        self.reads += 1;
        Ok(self.segments.pop_front())
    }

    fn render(&mut self, outcome: Result<&Value, &ReplError>) -> io::Result<()> {
        self.outcomes.push(outcome.cloned().map_err(Clone::clone));
        Ok(())
    }

    fn render_text(&mut self, text: &str) -> io::Result<()> {
        self.texts.push(text.to_string());
        Ok(())
    }

    fn metacommands(&self) -> Vec<MetacommandSpec> {
        self.specs.clone()
    }

    fn metacommand(&mut self, invocation: &Invocation) -> io::Result<()> {
        if invocation.name() == "explode" {
            return Err(io::Error::other("exploded"));
        }
        self.invoked.push(invocation.clone());
        Ok(())
    }
}

/// The metacommands a console offers in the tests that follow.
fn console_specs() -> Vec<MetacommandSpec> {
    vec![
        MetacommandSpec::new("greet", "Greet someone").with_arguments("<name>"),
        MetacommandSpec::new("explode", "Fail, ending the session"),
    ]
}

#[test]
fn a_frontends_metacommand_runs_on_the_frontend_with_its_argument() {
    let mut console = Console::new(console_specs(), &[":greet the world", "1", ":greet"]);
    Repl::new().run(&mut console).unwrap();
    let invoked: Vec<(&str, &str)> = console
        .invoked
        .iter()
        .map(|invocation| (invocation.name(), invocation.argument()))
        .collect();
    assert_eq!(invoked, [("greet", "the world"), ("greet", "")]);
    // The REPL shows nothing for it: that is the frontend's business.
    assert!(console.texts.is_empty(), "{:?}", console.texts);
    assert!(
        matches!(console.outcomes.as_slice(), [Ok(Value::Int(1))]),
        "{:?}",
        console.outcomes
    );
}

#[test]
fn a_frontends_metacommand_failing_ends_the_session() {
    let mut console = Console::new(console_specs(), &[":explode", "1"]);
    match Repl::new().run(&mut console) {
        Err(SessionError::Frontend(error)) => assert_eq!(error.to_string(), "exploded"),
        other => panic!("the session should end, but gave {other:?}"),
    }
    assert!(console.outcomes.is_empty(), "{:?}", console.outcomes);
}

#[test]
fn help_lists_a_frontends_metacommands_after_the_repls() {
    let mut console = Console::new(console_specs(), &[":help"]);
    Repl::new().run(&mut console).unwrap();
    let [help] = console.texts.as_slice() else {
        panic!("`:help` should show one text, but gave {:?}", console.texts);
    };
    let lines: Vec<&str> = help.lines().collect();
    let position = |usage: &str| {
        lines
            .iter()
            .position(|line| line.starts_with(usage))
            .unwrap_or_else(|| panic!("`:help` should list {usage:?}:\n{help}"))
    };
    let quit = position(":quit ");
    let greet = position(":greet <name> ");
    let explode = position(":explode ");
    assert!(quit < greet && greet < explode, "{help}");
    assert!(lines[greet].ends_with("  Greet someone"), "{help}");
}

/// What a session refuses of a frontend offering `specs`, having read
/// nothing.
fn refused(specs: Vec<MetacommandSpec>) -> (String, MetacommandProblem) {
    let mut console = Console::new(specs, &["1"]);
    let refusal = match Repl::new().run(&mut console) {
        Err(SessionError::Metacommand(invalid)) => (invalid.name().to_string(), invalid.problem()),
        other => panic!("the session should be refused, but gave {other:?}"),
    };
    assert_eq!(console.reads, 0, "a refused session reads nothing");
    refusal
}

#[test]
fn a_frontend_may_not_replace_a_built_in_metacommand() {
    for name in [
        "help",
        "quit",
        "bindings",
        "undef",
        "disassemble",
        "ast",
        "optimize",
    ] {
        let specs = vec![MetacommandSpec::new(name, "Mine now")];
        assert_eq!(
            refused(specs),
            (name.to_string(), MetacommandProblem::Builtin),
            "{name}"
        );
    }
}

#[test]
fn a_frontend_may_not_give_two_metacommands_one_name() {
    let specs = vec![
        MetacommandSpec::new("greet", "Greet"),
        MetacommandSpec::new("wave", "Wave"),
        MetacommandSpec::new("greet", "Greet again"),
    ];
    assert_eq!(
        refused(specs),
        ("greet".to_string(), MetacommandProblem::Duplicate)
    );
}

#[test]
fn a_frontends_metacommand_must_have_a_valid_name() {
    for name in [
        "", "Greet", "greet!", "1greet", "-greet", "gr eet", ":greet", "grëet",
    ] {
        let specs = vec![MetacommandSpec::new(name, "Greet")];
        assert_eq!(
            refused(specs),
            (name.to_string(), MetacommandProblem::InvalidName),
            "{name:?}"
        );
    }
}

#[test]
fn a_valid_name_is_lowercase_letters_digits_and_hyphens_after_a_letter() {
    let specs = ["g", "greet", "greet-twice", "greet2", "g-2-x"]
        .into_iter()
        .map(|name| MetacommandSpec::new(name, "Greet"))
        .collect();
    let mut console = Console::new(specs, &[]);
    Repl::new().run(&mut console).expect("every name is valid");
}

#[test]
fn a_refusal_explains_itself() {
    let specs = vec![MetacommandSpec::new("help", "Mine")];
    let mut console = Console::new(specs, &[]);
    let error = Repl::new().run(&mut console).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the frontend's metacommand `:help` would replace a built-in metacommand"
    );
}

#[test]
fn a_frontend_without_metacommands_of_its_own_has_only_the_repls() {
    // `ScriptedFrontend` keeps the trait's defaults.
    let mut frontend = ScriptedFrontend::new([":greet"]);
    Repl::new().run(&mut frontend).unwrap();
    let outcomes = frontend.transcript().outcomes();
    let [Err(ReplError::Metacommand(error))] = outcomes.as_slice() else {
        panic!("`:greet` should fail, but gave {outcomes:?}");
    };
    assert_eq!(*error, MetacommandError::Unknown("greet".to_string()));
}

// --- How a failure shows ---

#[test]
fn a_metacommand_error_explains_itself() {
    let cases = [
        (
            MetacommandError::Unknown("nope".to_string()),
            "there is no metacommand `:nope`; `:help` lists them",
        ),
        (
            MetacommandError::InvalidArgument {
                metacommand: "undef".to_string(),
                reason: "`x` is not bound".to_string(),
            },
            "`:undef`: `x` is not bound",
        ),
        (
            MetacommandError::Failed {
                metacommand: "history".to_string(),
                reason: "cannot read the history".to_string(),
            },
            "`:history`: cannot read the history",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
        // A REPL failure shows it as it shows a runtime error.
        assert_eq!(
            ReplError::Metacommand(error).to_string(),
            format!("Error: {expected}")
        );
    }
}

#[test]
fn a_metacommand_failures_source_is_the_metacommand_error() {
    use std::error::Error;
    let error = ReplError::Metacommand(MetacommandError::Unknown("nope".to_string()));
    let source = error.source().expect("a source");
    assert_eq!(
        source.downcast_ref::<MetacommandError>(),
        Some(&MetacommandError::Unknown("nope".to_string()))
    );
}
