//! The config file through a session's public API: what a `Repl` reads from
//! it, what it hands the frontend, what it does about each problem, and
//! `:config reload`.

use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use frostlang::stdlib;
use frostlang::{ImporterBuilder, MapKey, Stdlib, Value};
use frostlang_repl::{
    CancelHandle, ConfigError, Frontend, MetacommandError, Repl, ReplError, ScriptedFrontend,
};

/// A temporary directory, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "frostlang-repl-config-{}-{}",
            process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("the scratch directory can be made");
        Self(path)
    }

    /// The path of a file in the directory, which may not exist.
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// The path of `repl.frst` in the directory, written with `source`.
    fn config(&self, source: &str) -> PathBuf {
        let path = self.file("repl.frst");
        fs::write(&path, source).expect("the config can be written");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// What a `Probe` does when asked for its next segment.
enum Step {
    /// Hand the REPL this segment.
    Say(&'static str),
    /// Overwrite the config file with this source, then go on to the next step.
    Write(&'static str),
    /// Replace the config file with a directory, which cannot be read as a
    /// file, then go on to the next step.
    MakeUnreadable,
    /// Remove the config file, then go on to the next step.
    Remove,
}

/// One thing a `Probe` saw.
#[derive(Debug, Clone, PartialEq)]
enum Event {
    CancelHandle,
    Configure(Option<Value>),
    Render,
}

/// A frontend that follows its steps, and records what it is shown and every
/// section it is configured with.
struct Probe {
    config_path: PathBuf,
    steps: VecDeque<Step>,
    key: Option<&'static str>,
    ansi: bool,
    events: Vec<Event>,
    outcomes: Vec<Result<Value, ReplError>>,
    texts: Vec<String>,
}

impl Probe {
    fn new(config_path: impl Into<PathBuf>, steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            config_path: config_path.into(),
            steps: steps.into_iter().collect(),
            key: None,
            ansi: false,
            events: Vec::new(),
            outcomes: Vec::new(),
            texts: Vec::new(),
        }
    }

    /// A probe that only says `segments`, taking the section `key`.
    fn saying(segments: &[&'static str], key: Option<&'static str>) -> Self {
        Self::new(PathBuf::new(), segments.iter().map(|&s| Step::Say(s))).with_key(key)
    }

    fn with_key(mut self, key: Option<&'static str>) -> Self {
        self.key = key;
        self
    }

    fn with_ansi(mut self, ansi: bool) -> Self {
        self.ansi = ansi;
        self
    }

    /// Every section the probe was configured with, in order.
    fn configured(&self) -> Vec<Option<Value>> {
        self.events
            .iter()
            .filter_map(|event| match event {
                Event::Configure(section) => Some(section.clone()),
                _ => None,
            })
            .collect()
    }

    /// The config problems rendered, as `Display` shows them.
    fn problems(&self) -> Vec<String> {
        self.outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                Err(ReplError::Config(error)) => Some(error.to_string()),
                _ => None,
            })
            .collect()
    }

    /// The values rendered, leaving out every failure.
    fn values(&self) -> Vec<Value> {
        self.outcomes
            .iter()
            .filter_map(|outcome| outcome.as_ref().ok().cloned())
            .collect()
    }

    /// The last value rendered, which there must be.
    fn last_value(&self) -> Value {
        self.values().pop().expect("a value was rendered")
    }
}

impl Frontend for Probe {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        loop {
            match self.steps.pop_front() {
                Some(Step::Say(segment)) => return Ok(Some(segment.to_string())),
                Some(Step::Write(source)) => fs::write(&self.config_path, source)?,
                Some(Step::MakeUnreadable) => {
                    fs::remove_file(&self.config_path)?;
                    fs::create_dir(&self.config_path)?;
                }
                Some(Step::Remove) => fs::remove_file(&self.config_path)?,
                None => return Ok(None),
            }
        }
    }

    fn render(&mut self, outcome: Result<&Value, &ReplError>) -> io::Result<()> {
        self.events.push(Event::Render);
        self.outcomes.push(outcome.cloned().map_err(Clone::clone));
        Ok(())
    }

    fn render_text(&mut self, text: &str) -> io::Result<()> {
        self.texts.push(text.to_string());
        Ok(())
    }

    fn ansi_styling(&self) -> bool {
        self.ansi
    }

    fn set_cancel_handle(&mut self, _handle: CancelHandle) {
        self.events.push(Event::CancelHandle);
    }

    fn config_key(&self) -> Option<&str> {
        self.key
    }

    fn configure(&mut self, section: Option<Value>) -> io::Result<()> {
        self.events.push(Event::Configure(section));
        Ok(())
    }
}

/// Run a session of `repl` on `probe`, returning the probe.
fn run(mut repl: Repl, mut probe: Probe) -> Probe {
    repl.run(&mut probe).expect("a probe never fails");
    probe
}

/// A session on a REPL reading `config`, over `segments`, taking the section
/// `key`.
fn session(config: &Path, segments: &[&'static str], key: Option<&'static str>) -> Probe {
    run(
        Repl::new().with_config_file(config),
        Probe::saying(segments, key),
    )
}

/// A REPL that can import `std.math`, reading `config`.
fn math_repl(config: &Path) -> Repl {
    let stdlib = Stdlib::new()
        .with_module(stdlib::math())
        .expect("one module is distinct");
    let importer = ImporterBuilder::new().with_stdlib(stdlib).build();
    Repl::new().with_importer(importer).with_config_file(config)
}

/// The value `section` holds under `key`.
#[track_caller]
fn entry(section: &Value, key: &str) -> Value {
    let Value::Map(map) = section else {
        panic!("expected a Map, got {section:?}");
    };
    map.clone()
        .into_map()
        .get(&MapKey::from(key))
        .unwrap_or_else(|| panic!("{key:?} missing from {section:?}"))
        .clone()
}

/// The message of a rendered metacommand failure.
#[track_caller]
fn metacommand_error(outcome: &Result<Value, ReplError>) -> &MetacommandError {
    match outcome {
        Err(ReplError::Metacommand(error)) => error,
        other => panic!("expected a metacommand failure, got {other:?}"),
    }
}

/// Assert that `probe` rendered exactly one config problem, whose text holds
/// each of `fragments`.
#[track_caller]
fn assert_one_problem(probe: &Probe, fragments: &[&str]) {
    let problems = probe.problems();
    let [problem] = problems.as_slice() else {
        panic!("expected one config problem, got {problems:?}");
    };
    for fragment in fragments {
        assert!(
            problem.contains(fragment),
            "{problem:?} should contain {fragment:?}"
        );
    }
}

// --- Whether a file is read ---

#[test]
fn a_repl_with_no_config_file_reads_none_and_configures_nothing() {
    let probe = run(Repl::new(), Probe::saying(&["1"], Some("mine")));
    assert!(
        probe.configured().is_empty(),
        "no file is read, so there is nothing to configure with: {:?}",
        probe.events
    );
    assert!(probe.problems().is_empty());
}

#[test]
fn evaluate_alone_never_reads_the_config_file() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: 0}");
    let mut repl = Repl::new().with_config_file(path);
    repl.evaluate("1").unwrap();
    assert_eq!(
        repl.evaluate("results").unwrap(),
        Value::array([1]),
        "results_kept: 0 would bind no `results`, so the file was read"
    );
}

#[test]
fn the_file_is_read_after_the_frontend_is_given_the_cancel_handle() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: oops}");
    let probe = session(&path, &[], None);
    assert_eq!(
        probe.events.first(),
        Some(&Event::CancelHandle),
        "{:?}",
        probe.events
    );
    assert_eq!(probe.events.len(), 2, "a problem is shown, not a section");
}

#[test]
fn a_missing_file_sets_nothing_and_shows_nothing() {
    let scratch = Scratch::new();
    let probe = session(
        &scratch.file("absent.frst"),
        &["1", "results"],
        Some("mine"),
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.outcomes);
    assert_eq!(probe.last_value(), Value::array([1]));
    assert_eq!(
        probe.configured(),
        [None],
        "a missing file has no exports, so the frontend is asked for its defaults"
    );
}

#[test]
fn an_unreadable_path_shows_one_error_before_the_first_segment_and_sets_nothing() {
    let scratch = Scratch::new();
    // A directory cannot be read as a file.
    let path = scratch.file("a_directory");
    fs::create_dir(&path).unwrap();
    let probe = session(&path, &["1", "results"], Some("mine"));
    let first = probe.outcomes.first().expect("an outcome was rendered");
    let Err(ReplError::Config(ConfigError::Unreadable { path: shown, .. })) = first else {
        panic!(
            "expected an unreadable config first, got {:?}",
            probe.outcomes
        );
    };
    assert_eq!(*shown, path);
    assert_eq!(probe.problems().len(), 1, "{:?}", probe.problems());
    assert!(
        probe.problems()[0].starts_with("Error: "),
        "{:?}",
        probe.problems()
    );
    assert!(probe.configured().is_empty(), "{:?}", probe.events);
    // The session carried on, with the default results.
    assert_eq!(probe.last_value(), Value::array([1]));
}

#[test]
fn a_file_that_does_not_compile_shows_one_error_naming_the_file_and_sets_nothing() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: 0}\nexport def = ");
    let probe = session(&path, &["1", "results"], Some("mine"));
    let [Err(ReplError::Config(ConfigError::Compile(_))), ..] = probe.outcomes.as_slice() else {
        panic!("expected a compile error first, got {:?}", probe.outcomes);
    };
    assert_one_problem(&probe, &[&path.display().to_string()]);
    assert!(probe.configured().is_empty(), "{:?}", probe.events);
    assert_eq!(
        probe.last_value(),
        Value::array([1]),
        "results_kept stays the default"
    );
}

#[test]
fn a_file_that_raises_shows_one_error_and_sets_nothing() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def session = {results_kept: 0}
defn broken(x) -> x + 'text'
export def other = broken(1)
",
    );
    let probe = session(&path, &["1", "results"], Some("mine"));
    let [Err(ReplError::Config(ConfigError::Run(_))), ..] = probe.outcomes.as_slice() else {
        panic!("expected a run error first, got {:?}", probe.outcomes);
    };
    assert_one_problem(&probe, &["Error: the REPL configuration failed"]);
    assert!(probe.configured().is_empty(), "{:?}", probe.events);
    assert_eq!(probe.last_value(), Value::array([1]));
}

#[test]
fn a_config_error_never_stops_the_session() {
    let scratch = Scratch::new();
    let path = scratch.config("export def = ");
    let probe = session(&path, &["def x = 20", "x + 1"], None);
    assert_eq!(probe.last_value(), Value::Int(21));
}

// --- Sections ---

#[test]
fn the_frontend_gets_the_export_its_key_names() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def mine = {a: 1}
export def theirs = {b: 2}
",
    );
    let probe = session(&path, &["1"], Some("mine"));
    assert_eq!(
        probe.configured(),
        [Some(Value::map([("a", Value::Int(1))]))]
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
}

#[test]
fn a_section_need_not_be_a_map() {
    let scratch = Scratch::new();
    let path = scratch.config("export def mine = 7");
    let probe = session(&path, &[], Some("mine"));
    assert_eq!(probe.configured(), [Some(Value::Int(7))]);
}

#[test]
fn the_frontend_gets_none_when_the_file_has_no_export_under_its_key() {
    let scratch = Scratch::new();
    let path = scratch.config("export def theirs = {b: 2}");
    let probe = session(&path, &[], Some("mine"));
    assert_eq!(probe.configured(), [None]);
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
}

#[test]
fn a_frontend_with_no_key_gets_none_whatever_the_file_exports() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def mine = {a: 1}
export def theirs = {b: 2}
",
    );
    let probe = session(&path, &[], None);
    assert_eq!(probe.configured(), [None]);
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
}

#[test]
fn exports_that_are_nobodys_section_are_ignored_silently() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def stray = 'unclaimed'
export def other_frontend = {color: 'red'}
export def session = {results_kept: 2}
",
    );
    let probe = session(&path, &["1"], Some("mine"));
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(probe.configured(), [None]);
}

#[test]
fn definitions_in_the_file_are_not_bound_in_the_session() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def mine = {a: 1}
def helper = 5
",
    );
    let probe = session(&path, &["helper"], Some("mine"));
    assert!(
        matches!(probe.outcomes.as_slice(), [Err(ReplError::Compile(_))]),
        "{:?}",
        probe.outcomes
    );
}

#[test]
fn a_section_reaches_the_frontend_as_the_script_made_it_functions_included() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def mine = {
    number: 3,
    text: 'hi',
    list: [1, 2],
    callback: fn x -> x + 1,
}
",
    );
    let probe = session(&path, &[], Some("mine"));
    let configured = probe.configured();
    let [Some(section)] = configured.as_slice() else {
        panic!("expected one section, got {configured:?}");
    };
    assert_eq!(entry(section, "number"), Value::Int(3));
    assert_eq!(entry(section, "text"), Value::from("hi"));
    assert_eq!(entry(section, "list"), Value::array([1, 2]));
    assert!(
        matches!(entry(section, "callback"), Value::Closure(_)),
        "{section:?}"
    );
}

#[test]
fn repl_colors_and_style_color_only_when_the_frontend_takes_ansi_styling() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
def repl = import('repl')
export def mine = {
    color: repl.colors.red('x'),
    bright: repl.colors.bright_cyan('x'),
    style: repl.style.bold('x'),
}
",
    );
    for (ansi, styled) in [(true, true), (false, false)] {
        let probe = run(
            Repl::new().with_config_file(&path),
            Probe::saying(&[], Some("mine")).with_ansi(ansi),
        );
        assert!(probe.problems().is_empty(), "{:?}", probe.problems());
        let configured = probe.configured();
        let Some(Some(section)) = configured.first() else {
            panic!("expected a section, got {configured:?}");
        };
        for key in ["color", "bright", "style"] {
            let text = entry(section, key);
            let text = text.as_str().expect("a String");
            if styled {
                assert!(
                    text.contains('\u{1b}') && text.contains('x'),
                    "{key}: {text:?}"
                );
            } else {
                assert_eq!(text, "x", "{key} with no ANSI styling");
            }
        }
    }
}

// --- session.results_kept ---

#[test]
fn results_kept_replaces_the_count_from_with_results_kept() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: 2}");
    let probe = run(
        Repl::new().with_results_kept(10).with_config_file(path),
        Probe::saying(&["1", "2", "3", "results"], None),
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(probe.last_value(), Value::array([2, 3]));
}

#[test]
fn results_kept_of_zero_binds_no_results() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: 0}");
    let probe = session(&path, &["1", "results"], None);
    assert!(
        matches!(
            probe.outcomes.as_slice(),
            [Ok(Value::Int(1)), Err(ReplError::Compile(_))]
        ),
        "{:?}",
        probe.outcomes
    );
}

#[test]
fn a_results_kept_of_the_wrong_type_warns_and_leaves_the_default() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: 'many'}");
    let probe = run(
        Repl::new().with_results_kept(3).with_config_file(path),
        Probe::saying(&["1", "2", "3", "4", "results"], None),
    );
    assert_one_problem(&probe, &["Warning", "sets nothing", "results_kept"]);
    assert!(matches!(
        probe.outcomes[0],
        Err(ReplError::Config(ConfigError::Session(_)))
    ));
    assert_eq!(probe.last_value(), Value::array([2, 3, 4]));
}

#[test]
fn a_negative_results_kept_warns_and_leaves_the_default() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: -1}");
    let probe = run(
        Repl::new().with_results_kept(3).with_config_file(path),
        Probe::saying(&["1", "2", "3", "4", "results"], None),
    );
    assert_one_problem(&probe, &["Warning", "sets nothing", "results_kept", "-1"]);
    assert_eq!(probe.last_value(), Value::array([2, 3, 4]));
}

#[test]
fn a_null_results_kept_has_no_opinion_and_is_no_problem() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: null}");
    let probe = run(
        Repl::new().with_results_kept(3).with_config_file(path),
        Probe::saying(&["1", "2", "3", "4", "results"], None),
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(
        probe.last_value(),
        Value::array([2, 3, 4]),
        "the Repl's own count applies"
    );
}

#[test]
fn a_null_results_kept_does_not_stop_preload_from_applying() {
    let scratch = Scratch::new();
    let path =
        scratch.config("export def session = {results_kept: null, preload: {m: 'std.math'}}");
    let probe = run(math_repl(&path), Probe::saying(&["m.abs(-3)"], None));
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(probe.last_value(), Value::Int(3));
}

/// A Null `preload` has no opinion: nothing is preloaded, there is no
/// problem, and the settings beside it still apply.
#[test]
fn a_null_preload_has_no_opinion_and_is_no_problem() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: 1, preload: null}");
    let probe = session(&path, &["1", "2", "results"], None);
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(
        probe.last_value(),
        Value::array([2]),
        "the results_kept beside it applies"
    );
}

// --- session.preload ---

#[test]
fn preload_binds_each_name_to_its_module() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}}");
    let probe = run(
        math_repl(&path),
        Probe::saying(&["m.abs(-3)", ":bindings"], None),
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(probe.values(), [Value::Int(3)]);
    let [listing] = probe.texts.as_slice() else {
        panic!("expected one listing, got {:?}", probe.texts);
    };
    assert!(listing.contains('m'), "{listing:?}");
}

#[test]
fn preload_replaces_an_existing_binding_of_the_same_name() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}}");
    let repl = math_repl(&path).with_binding("m", Value::Int(0)).unwrap();
    let probe = run(repl, Probe::saying(&["m.abs(-3)"], None));
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(probe.last_value(), Value::Int(3));
}

#[test]
fn a_preload_of_a_missing_module_is_an_error_naming_the_binding_and_the_rest_bind() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def session = {preload: {gone: 'std.nonexistent', m: 'std.math'}}
",
    );
    let probe = run(
        math_repl(&path),
        Probe::saying(&["m.abs(-3)", "gone"], None),
    );
    assert_one_problem(&probe, &["Error", "preload.gone"]);
    assert!(matches!(
        probe.outcomes[0],
        Err(ReplError::Config(ConfigError::Session(_)))
    ));
    let rest = &probe.outcomes[1..];
    assert!(
        matches!(rest, [Ok(Value::Int(3)), Err(ReplError::Compile(_))]),
        "`m` binds and `gone` does not: {rest:?}"
    );
}

#[test]
fn a_bad_preload_entry_makes_the_whole_session_set_nothing() {
    let bad_entries = [
        ("a key that is not a String", "[1]: 'std.math'", "a key"),
        ("a keyword as a name", "['if']: 'std.math'", "if"),
        ("a malformed name", "['1x']: 'std.math'", "1x"),
        ("a path that is not a String", "bad_path: 5", "preload"),
    ];
    for (what, entry, fragment) in bad_entries {
        let scratch = Scratch::new();
        let path = scratch.config(&format!(
            r"
export def session = {{
    results_kept: 1,
    preload: {{
        m: 'std.math',
        {entry},
    }},
}}
"
        ));
        let probe = run(
            math_repl(&path).with_results_kept(3),
            Probe::saying(&["1", "2", "3", "results", "m"], None),
        );
        let problems = probe.problems();
        let [problem] = problems.as_slice() else {
            panic!("{what}: expected one problem, got {problems:?}");
        };
        for expected in ["Warning", "sets nothing", fragment] {
            assert!(
                problem.contains(expected),
                "{what}: {problem:?} should contain {expected:?}"
            );
        }
        let rest = &probe.outcomes[1..];
        assert!(
            matches!(
                rest,
                [Ok(_), Ok(_), Ok(_), Ok(results), Err(ReplError::Compile(_))]
                    if *results == Value::array([1, 2, 3])
            ),
            "{what}: results_kept stays 3 and `m` is not bound: {rest:?}"
        );
    }
}

#[test]
fn a_preload_that_is_not_a_map_warns_and_binds_nothing() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: ['std.math']}");
    let probe = run(math_repl(&path), Probe::saying(&[":bindings"], None));
    assert_one_problem(&probe, &["Warning", "sets nothing", "preload"]);
    assert_eq!(
        probe.texts,
        ["results  Array"],
        "nothing is bound but the REPL's own `results`"
    );
}

// --- The session section as a whole ---

#[test]
fn a_session_that_is_not_a_map_warns_once_and_sets_nothing() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = 5");
    let probe = run(
        Repl::new().with_results_kept(3).with_config_file(path),
        Probe::saying(&["1", "2", "3", "4", "results"], None),
    );
    assert_one_problem(&probe, &["Warning", "sets nothing", "session"]);
    assert_eq!(probe.last_value(), Value::array([2, 3, 4]));
}

#[test]
fn an_unknown_session_key_warns_naming_it_and_the_known_ones_and_nothing_applies() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def session = {
    results_kept: 1,
    preload: {m: 'std.math'},
    colour: 'red',
}
",
    );
    let probe = run(
        math_repl(&path),
        Probe::saying(&["1", "2", "results", "m"], None),
    );
    assert_one_problem(
        &probe,
        &[
            "Warning",
            "sets nothing",
            "colour",
            "results_kept",
            "preload",
        ],
    );
    let rest = &probe.outcomes[1..];
    assert!(
        matches!(
            rest,
            [Ok(_), Ok(_), Ok(results), Err(ReplError::Compile(_))]
                if *results == Value::array([1, 2])
        ),
        "the valid settings beside the unknown key do not apply: {rest:?}"
    );
}

#[test]
fn a_session_with_several_bad_settings_is_one_problem() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {one: 1, two: 2, results_kept: 'x'}");
    let probe = session(&path, &[], None);
    assert_one_problem(&probe, &["Warning", "sets nothing"]);
    assert_eq!(probe.outcomes.len(), 1, "{:?}", probe.outcomes);
}

#[test]
fn each_problem_is_a_separate_render() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def session = {preload: {
    a: 'std.gone_a',
    b: 'std.gone_b',
    c: 'std.gone_c',
}}
",
    );
    let probe = session(&path, &[], None);
    let problems = probe.problems();
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert_eq!(probe.outcomes.len(), 3, "one render per problem");
    for (name, problem) in ["a", "b", "c"].iter().zip(&problems) {
        assert!(
            problem.starts_with("Error") && problem.contains(&format!("preload.{name}")),
            "{problem:?} should be the Error for `{name}`"
        );
    }
}

#[test]
fn a_session_problem_and_a_failing_preload_cannot_both_arise() {
    // A bad section sets nothing, so no preload is attempted to fail.
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def session = {preload: {gone: 'std.gone'}, results_kept: -1}
",
    );
    let probe = session(&path, &[], None);
    assert_one_problem(&probe, &["Warning", "sets nothing", "results_kept"]);
}

// --- :config reload ---

#[test]
fn reload_reads_the_file_again_applying_results_kept_and_the_section() {
    let scratch = Scratch::new();
    let path = scratch.config(
        r"
export def session = {results_kept: 5}
export def mine = {n: 1}
",
    );
    let probe = run(
        Repl::new().with_config_file(&path),
        Probe::new(
            &path,
            [
                Step::Say("1"),
                Step::Say("2"),
                Step::Write(
                    r"
export def session = {results_kept: 1}
export def mine = {n: 2}
",
                ),
                Step::Say(":config reload"),
                Step::Say("results"),
            ],
        )
        .with_key(Some("mine")),
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(
        probe.configured(),
        [
            Some(Value::map([("n", Value::Int(1))])),
            Some(Value::map([("n", Value::Int(2))])),
        ]
    );
    assert_eq!(
        probe.last_value(),
        Value::array([2]),
        "the reload trims the results held to the new count"
    );
}

#[test]
fn reload_configures_none_when_the_section_is_gone() {
    let scratch = Scratch::new();
    let path = scratch.config("export def mine = {n: 1}");
    let probe = run(
        Repl::new().with_config_file(&path),
        Probe::new(
            &path,
            [
                Step::Write("export def other = 1"),
                Step::Say(":config reload"),
            ],
        )
        .with_key(Some("mine")),
    );
    assert_eq!(
        probe.configured(),
        [Some(Value::map([("n", Value::Int(1))])), None]
    );
}

#[test]
fn reload_does_not_preload_again() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}}");
    let probe = run(
        math_repl(&path),
        Probe::new(
            &path,
            [
                Step::Say(":undef m"),
                Step::Write("export def session = {preload: {m: 'std.math', n: 'std.math'}}"),
                Step::Say(":config reload"),
                Step::Say("m"),
                Step::Say("n"),
            ],
        ),
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    let outcomes = &probe.outcomes;
    assert!(
        matches!(
            outcomes.as_slice(),
            [Err(ReplError::Compile(_)), Err(ReplError::Compile(_))]
        ),
        "neither the removed `m` nor the added `n` is bound: {outcomes:?}"
    );
}

#[test]
fn a_failed_reload_shows_its_error_and_keeps_the_previous_settings() {
    let broken: [(&'static str, &[&str]); 3] = [
        ("export def = ", &["repl.frst"]),
        (
            "defn broken(x) -> x + 'text'\nexport def session = broken(1)",
            &["Error: the REPL configuration failed"],
        ),
        (
            "export def session = {results_kept: 0}\nexport def x = 1 +",
            &[],
        ),
    ];
    for (source, fragments) in broken {
        let scratch = Scratch::new();
        let path = scratch.config("export def session = {results_kept: 2}\nexport def mine = 1");
        let probe = run(
            Repl::new().with_config_file(&path),
            Probe::new(
                &path,
                [
                    Step::Say("1"),
                    Step::Say("2"),
                    Step::Say("3"),
                    Step::Write(source),
                    Step::Say(":config reload"),
                    Step::Say("results"),
                ],
            )
            .with_key(Some("mine")),
        );
        assert_one_problem(&probe, fragments);
        assert_eq!(
            probe.configured(),
            [Some(Value::Int(1))],
            "{source:?}: a failed reload does not configure"
        );
        assert_eq!(
            probe.last_value(),
            Value::array([2, 3]),
            "{source:?}: results_kept stays 2"
        );
    }
}

#[test]
fn a_reload_of_an_unreadable_path_shows_its_error_and_keeps_the_settings() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: 2}");
    let probe = run(
        Repl::new().with_config_file(&path),
        Probe::new(
            &path,
            [
                Step::Say("1"),
                Step::Say("2"),
                Step::Say("3"),
                Step::MakeUnreadable,
                Step::Say(":config reload"),
                Step::Say("results"),
            ],
        )
        .with_key(Some("mine")),
    );
    assert_one_problem(&probe, &["Error", "cannot read"]);
    assert!(
        probe.outcomes.iter().any(|outcome| matches!(
            outcome,
            Err(ReplError::Config(ConfigError::Unreadable { .. }))
        )),
        "{:?}",
        probe.outcomes
    );
    assert_eq!(probe.configured(), [None], "only the start configured");
    assert_eq!(probe.last_value(), Value::array([2, 3]));
}

#[test]
fn a_reload_after_a_clean_reload_still_works() {
    let scratch = Scratch::new();
    let path = scratch.config("export def mine = 1");
    let probe = run(
        Repl::new().with_config_file(&path),
        Probe::new(
            &path,
            [
                Step::Write("export def mine = 2"),
                Step::Say(":config reload"),
                Step::Write("export def mine = 3"),
                Step::Say(":config reload"),
            ],
        )
        .with_key(Some("mine")),
    );
    assert_eq!(
        probe.configured(),
        [
            Some(Value::Int(1)),
            Some(Value::Int(2)),
            Some(Value::Int(3))
        ]
    );
}

// --- :reset ---

#[test]
fn reset_preloads_again_and_keeps_the_files_settings() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {results_kept: 1, preload: {m: 'std.math'}}");
    let probe = run(
        math_repl(&path),
        Probe::saying(
            &["def m = 0", ":reset", "m.abs(-3)", "4", "5", "results"],
            None,
        ),
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert!(
        matches!(
            probe.outcomes.as_slice(),
            [
                Ok(Value::Null),
                Ok(Value::Int(3)),
                Ok(Value::Int(4)),
                Ok(Value::Int(5)),
                Ok(results),
            ] if *results == Value::array([5])
        ),
        "`m` is the preloaded module again, and the file's `results_kept` of 1 \
         still applies: {:?}",
        probe.outcomes
    );
}

/// `:reset` reads the file as it now is and preloads as a session start does,
/// but leaves the frontend's settings as the session has them.
#[test]
fn reset_reads_the_file_again() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}}");
    let probe = Probe::new(
        &path,
        [
            Step::Write("export def session = {preload: {n: 'std.math'}}\nexport def mine = 2"),
            Step::Say(":reset"),
            Step::Say("n.abs(-3)"),
            Step::Say("m"),
        ],
    )
    .with_key(Some("mine"));
    let probe = run(math_repl(&path), probe);
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert!(
        matches!(
            probe.outcomes.as_slice(),
            [Ok(Value::Int(3)), Err(ReplError::Compile(_))]
        ),
        "the file's new preload is bound, and its old one is not: {:?}",
        probe.outcomes
    );
    assert_eq!(
        probe.configured(),
        [None],
        "the frontend is configured at the start only, not at `:reset`"
    );
}

#[test]
fn reset_preloads_over_a_seeded_binding_of_the_same_name() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}}");
    let repl = math_repl(&path).with_binding("m", Value::Int(0)).unwrap();
    let probe = run(
        repl,
        Probe::saying(&["def m = 1", ":reset", "m.abs(-3)"], None),
    );
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert_eq!(
        probe.last_value(),
        Value::Int(3),
        "the preload replaces the seeded `m`, as at the start"
    );
}

/// A file that no longer loads at `:reset` sets nothing, as at `:config reload`:
/// its problem is shown, only the seeded bindings are bound again, and the
/// earlier file's `results_kept` still applies.
#[test]
fn reset_with_a_file_that_no_longer_loads_shows_why_and_binds_only_seeded_bindings() {
    for (what, step) in [
        ("does not compile", Step::Write("export def = ")),
        ("unreadable", Step::MakeUnreadable),
    ] {
        let scratch = Scratch::new();
        let path =
            scratch.config("export def session = {results_kept: 1, preload: {m: 'std.math'}}");
        let repl = math_repl(&path)
            .with_binding("seeded", Value::Int(7))
            .unwrap();
        let probe = Probe::new(
            &path,
            [
                Step::Say("def mine = 1"),
                step,
                Step::Say(":reset"),
                Step::Say("seeded"),
                Step::Say("mine"),
                Step::Say("m"),
                Step::Say("2"),
                Step::Say("results"),
            ],
        );
        let probe = run(repl, probe);
        assert_eq!(probe.problems().len(), 1, "{what}: {:?}", probe.problems());
        assert!(
            matches!(
                probe.outcomes.as_slice(),
                [
                    Ok(Value::Null),
                    Err(ReplError::Config(_)),
                    Ok(Value::Int(7)),
                    Err(ReplError::Compile(_)),
                    Err(ReplError::Compile(_)),
                    Ok(Value::Int(2)),
                    Ok(results),
                ] if *results == Value::array([2])
            ),
            "{what}: the problem shown at `:reset`, `seeded` bound again, `mine` and `m` \
             not, and `results_kept` stays 1: {:?}",
            probe.outcomes
        );
    }
}

/// A file gone at `:reset` is no problem, as at a session start without one:
/// the seeded bindings are bound again, and the file's preloads are not.
#[test]
fn reset_with_the_file_gone_binds_only_seeded_bindings() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}}");
    let repl = math_repl(&path)
        .with_binding("seeded", Value::Int(7))
        .unwrap();
    let probe = Probe::new(
        &path,
        [
            Step::Say("m.abs(-3)"),
            Step::Remove,
            Step::Say(":reset"),
            Step::Say("seeded"),
            Step::Say("m"),
        ],
    )
    .with_key(Some("mine"));
    let probe = run(repl, probe);
    assert!(probe.problems().is_empty(), "{:?}", probe.problems());
    assert!(
        matches!(
            probe.outcomes.as_slice(),
            [
                Ok(Value::Int(3)),
                Ok(Value::Int(7)),
                Err(ReplError::Compile(_))
            ]
        ),
        "`m` was preloaded at the start, and is unbound after `:reset`: {:?}",
        probe.outcomes
    );
    assert_eq!(
        probe.configured(),
        [None],
        "the frontend is configured at the start only, not at `:reset`"
    );
}

#[test]
fn a_refused_reset_reads_no_file() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}}");
    let probe = run(
        math_repl(&path),
        Probe::saying(&["def m = 1", ":reset please", "m"], Some("mine")),
    );
    assert_eq!(
        probe.configured(),
        [None],
        "only the session start configured the frontend"
    );
    assert_eq!(
        probe.last_value(),
        Value::Int(1),
        "`m` keeps its redefinition"
    );
}

#[test]
fn reset_shows_a_preload_that_now_fails_and_binds_the_rest() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}}");
    let probe = Probe::new(
        &path,
        [
            Step::Write("export def session = {preload: {gone: 'std.nonexistent', m: 'std.math'}}"),
            Step::Say(":reset"),
            Step::Say("m.abs(-3)"),
            Step::Say("gone"),
        ],
    );
    let probe = run(math_repl(&path), probe);
    assert_one_problem(&probe, &["session.preload.gone", "failed to import"]);
    let values: Vec<_> = probe
        .outcomes
        .iter()
        .filter(|outcome| !matches!(outcome, Err(ReplError::Config(_))))
        .collect();
    assert!(
        matches!(
            values.as_slice(),
            [Ok(Value::Int(3)), Err(ReplError::Compile(_))]
        ),
        "`m`, the other preload, still binds, and `gone` does not: {values:?}"
    );
}

// --- The :config metacommand itself ---

#[test]
fn config_needs_exactly_reload() {
    let scratch = Scratch::new();
    let path = scratch.config("");
    for segment in [
        ":config",
        ":config please",
        ":config reload now",
        ":config Reload",
    ] {
        let probe = run(
            Repl::new().with_config_file(&path),
            Probe::saying(&[segment, "1"], None),
        );
        let outcome = probe
            .outcomes
            .iter()
            .find(|outcome| matches!(outcome, Err(ReplError::Metacommand(_))))
            .unwrap_or_else(|| panic!("{segment:?} should fail: {:?}", probe.outcomes));
        let MetacommandError::InvalidArgument {
            metacommand,
            reason,
        } = metacommand_error(outcome)
        else {
            panic!("{segment:?} should be an invalid argument: {outcome:?}");
        };
        assert_eq!(metacommand, "config", "{segment:?}");
        assert!(reason.contains("reload"), "{segment:?}: {reason:?}");
        assert_eq!(
            probe.last_value(),
            Value::Int(1),
            "{segment:?}: the session carries on"
        );
    }
}

#[test]
fn reload_without_a_config_file_fails_saying_so() {
    let probe = run(Repl::new(), Probe::saying(&[":config reload", "1"], None));
    let outcome = &probe.outcomes[0];
    let MetacommandError::Failed {
        metacommand,
        reason,
    } = metacommand_error(outcome)
    else {
        panic!("expected a failure, got {outcome:?}");
    };
    assert_eq!(metacommand, "config");
    assert!(reason.contains("reads no config file"), "{reason:?}");
    assert_eq!(probe.last_value(), Value::Int(1));
    assert!(probe.configured().is_empty());
}

#[test]
fn help_lists_config_reload() {
    let mut frontend = ScriptedFrontend::new([":help"]);
    Repl::new().run(&mut frontend).unwrap();
    let texts = frontend.transcript().texts();
    let [help] = texts.as_slice() else {
        panic!("expected one text, got {texts:?}");
    };
    let line = help
        .lines()
        .find(|line| line.starts_with(":config"))
        .unwrap_or_else(|| panic!("`:help` should list `:config`:\n{help}"));
    assert!(line.contains("reload"), "{line:?}");
}

// --- Running twice ---

#[test]
fn each_run_reads_the_file_as_it_starts() {
    let scratch = Scratch::new();
    let path = scratch.config("export def session = {preload: {m: 'std.math'}, results_kept: 1}");
    let mut repl = math_repl(&path);
    let mut first = Probe::saying(&[":undef m", "1"], Some("mine"));
    repl.run(&mut first).unwrap();
    assert_eq!(first.configured(), [None]);
    let mut second = Probe::saying(&["m.abs(-3)", "results"], Some("mine"));
    repl.run(&mut second).unwrap();
    assert_eq!(
        second.configured(),
        [None],
        "the second run configures its frontend too"
    );
    assert_eq!(
        second.values(),
        [Value::Int(3), Value::array([3])],
        "preload ran again at the second start, rebinding `m`"
    );
}
