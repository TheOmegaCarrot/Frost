//! The driver end to end: command lines in; exit statuses, a script's printed
//! output, and the driver's own output out.

use std::fs;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use frostlang_compile::{Optimization, OptimizationOptions};
use frostlang_driver::{Driver, Exit, ReplSettings};
use frostlang_repl::{
    Frontend, InvalidName, MetacommandSpec, ReplError, ScriptedFrontend, Transcript,
};
use frostlang_runtime::{Extension, ImporterBuilder, Value, VmRuntimeConfiguration};

/// What one run produced.
#[derive(Debug)]
struct Ran {
    exit: Exit,
    stdout: String,
    stderr: String,
}

impl Ran {
    /// Each line written to stdout: for a script, the text of each `print`.
    fn printed(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

/// An in-memory output the test reads back once the driver is done with it.
#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

impl Buffer {
    fn contents(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Run `driver` on `args` (after the program name), with `stdin` as its
/// standard input, capturing its output. `configure` adjusts the
/// configuration it runs scripts under.
fn run_fed(
    driver: Driver,
    configure: impl FnOnce(&mut VmRuntimeConfiguration),
    stdin: &str,
    args: &[&str],
) -> Ran {
    let mut configuration = VmRuntimeConfiguration::default();
    configure(&mut configuration);
    let (stdout, stderr) = (Buffer::default(), Buffer::default());
    let exit = driver.with_configuration(configuration).run(
        ["frost"].iter().chain(args),
        stdin.as_bytes(),
        stdout.clone(),
        stderr.clone(),
    );
    Ran {
        exit,
        stdout: stdout.contents(),
        stderr: stderr.contents(),
    }
}

/// [`run_fed`] with nothing on standard input.
fn run_configured(
    driver: Driver,
    configure: impl FnOnce(&mut VmRuntimeConfiguration),
    args: &[&str],
) -> Ran {
    run_fed(driver, configure, "", args)
}

fn run(args: &[&str]) -> Ran {
    run_configured(Driver::new(), |_| {}, args)
}

/// [`run`] with `stdin` as standard input.
fn run_with_stdin(stdin: &str, args: &[&str]) -> Ran {
    run_fed(Driver::new(), |_| {}, stdin, args)
}

/// Write `source` to a script file named `name`, returning its path.
fn script(name: &str, source: &str) -> String {
    let path = scratch_path(name);
    fs::write(&path, source).unwrap();
    path
}

/// A path named `name` in the tests' scratch directory, with nothing written
/// there yet.
fn scratch_path(name: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("frostlang-driver-tests");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let _ = fs::remove_file(&path);
    path.to_string_lossy().into_owned()
}

/// Compile `source` to an image named `name`, returning the image's path.
fn image(name: &str, source: &str, options: &[&str]) -> String {
    let script = script(&format!("{name}.frst"), source);
    let image = scratch_path(name);
    let args: Vec<&str> = ["compile", script.as_str(), "-o", image.as_str()]
        .into_iter()
        .chain(options.iter().copied())
        .collect();
    let ran = run(&args);
    assert_eq!(ran.exit, Exit::Success, "{args:?}: {ran:?}");
    image
}

// --- Running scripts ---

#[test]
fn run_runs_a_script_file() {
    let source = r#"
        print("hello")
        print(1 + 2)
    "#;
    let path = script("run.frst", source);
    for args in [vec!["run", path.as_str()], vec![path.as_str()]] {
        let ran = run(&args);
        assert_eq!(ran.exit, Exit::Success, "{args:?}: {ran:?}");
        assert_eq!(ran.printed(), ["hello", "3"], "{args:?}");
        assert_eq!(ran.stderr, "", "{args:?}");
    }
}

#[test]
fn eval_runs_code_from_the_command_line() {
    for flag in ["-e", "--eval"] {
        let ran = run(&[flag, "print([1, 2] + [3])"]);
        assert_eq!(ran.exit, Exit::Success, "{flag}: {ran:?}");
        assert_eq!(ran.printed(), ["[ 1, 2, 3 ]"], "{flag}");
    }
}

#[test]
fn a_scripts_value_is_not_printed() {
    let ran = run(&["-e", "42"]);
    assert_eq!(ran.exit, Exit::Success);
    assert_eq!(ran.stdout, "");
}

#[test]
fn a_runtime_error_fails_the_script_with_its_message_and_backtrace() {
    let source = r#"
        print("before")
        defn f() -> 1 / 0
        f()
        print("after")
    "#;
    let ran = run(&["-e", source]);
    assert_eq!(ran.exit, Exit::ScriptFailed);
    assert_eq!(ran.printed(), ["before"], "output before the error stands");
    assert!(
        ran.stderr.starts_with("Error: Division by zero"),
        "{}",
        ran.stderr
    );
    assert!(ran.stderr.contains("  in f\n"), "{}", ran.stderr);
    assert!(ran.stderr.contains("  in <main>\n"), "{}", ran.stderr);
}

#[test]
fn a_compile_error_fails_the_script_with_its_diagnostics() {
    let path = script("broken.frst", "def x = ; 1");
    let unbound = r"
        print(1)
        nope
    ";
    for args in [
        vec!["--color", "never", path.as_str()],
        vec!["--color", "never", "-e", unbound],
    ] {
        let ran = run(&args);
        assert_eq!(ran.exit, Exit::ScriptFailed, "{args:?}");
        assert!(ran.printed().is_empty(), "nothing runs: {ran:?}");
        assert!(!ran.stderr.is_empty(), "{args:?}");
    }
    let ran = run(&["--color", "never", "-e", unbound]);
    assert!(
        ran.stderr.contains("`nope` is not defined"),
        "{}",
        ran.stderr
    );
    assert!(
        ran.stderr.contains("<eval>"),
        "named for -e: {}",
        ran.stderr
    );
}

#[test]
fn a_diagnostic_names_its_file() {
    let path = script("named.frst", "nope");
    let ran = run(&["--color", "never", &path]);
    assert!(ran.stderr.contains("named.frst"), "{}", ran.stderr);
}

#[test]
fn diagnostics_color_on_request_only() {
    let escape = '\u{1b}';
    let ran = run(&["--color", "always", "-e", "nope"]);
    assert!(ran.stderr.contains(escape), "colored: {:?}", ran.stderr);
    let ran = run(&["--color", "never", "-e", "nope"]);
    assert!(!ran.stderr.contains(escape), "plain: {:?}", ran.stderr);
}

/// `--color auto` goes by the streams the driver writes to, which here are
/// buffers, not by whether the test process itself is attached to a terminal.
#[test]
fn auto_color_writes_no_color_to_a_non_terminal() {
    let escape = '\u{1b}';
    let broken = script("auto-color.frst", "nope");
    for args in [vec!["-e", "nope"], vec!["--color", "auto", "-e", "nope"]] {
        let ran = run(&args);
        assert!(!ran.stderr.is_empty(), "{args:?}: {ran:?}");
        assert!(!ran.stderr.contains(escape), "{args:?}: {:?}", ran.stderr);
    }
    let ran = run(&["list", "--color", "auto", &broken]);
    assert!(!ran.stderr.contains(escape), "list: {:?}", ran.stderr);
    let listed = script("auto-color-list.frst", "1");
    let ran = run(&["list", "--color", "auto", &listed]);
    assert!(!ran.stdout.is_empty(), "{ran:?}");
    assert!(!ran.stdout.contains(escape), "list: {:?}", ran.stdout);
}

#[test]
fn an_unreadable_file_is_a_usage_error() {
    let ran = run(&["no-such-script.frst"]);
    assert_eq!(ran.exit, Exit::UsageError);
    assert!(
        ran.stderr.contains("cannot read no-such-script.frst"),
        "{}",
        ran.stderr
    );
}

// --- Checking scripts ---

#[test]
fn check_compiles_without_running() {
    let path = script("check.frst", r#"print("should not print")"#);
    let ran = run(&["check", &path]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert!(ran.printed().is_empty(), "{ran:?}");
    assert_eq!(ran.stderr, "");
}

#[test]
fn check_reports_diagnostics() {
    let path = script("check-broken.frst", "nope");
    let ran = run(&["check", "--color", "never", &path]);
    assert_eq!(ran.exit, Exit::ScriptFailed);
    assert!(
        ran.stderr.contains("`nope` is not defined"),
        "{}",
        ran.stderr
    );
    // A runtime error is not a diagnostic: check passes it.
    let path = script("check-raises.frst", "1 / 0");
    assert_eq!(run(&["check", &path]).exit, Exit::Success);
}

// --- Compiled images ---

#[test]
fn an_image_runs_as_its_script_does() {
    let source = r#"
        defn fib(n) -> if n < 2: n else: fib(n - 1) + fib(n - 2)
        print(fib(15))
        print([0.0, -0.0, {a: [1, "two"]}, x'00ff'])
    "#;
    let script = script("image-source.frst", source);
    let image = image("image-runs", source, &[]);
    let from_script = run(&[&script]);
    assert_eq!(from_script.exit, Exit::Success, "{from_script:?}");
    for args in [vec!["run", image.as_str()], vec![image.as_str()]] {
        let ran = run(&args);
        assert_eq!(ran.exit, Exit::Success, "{args:?}: {ran:?}");
        assert_eq!(ran.stdout, from_script.stdout, "{args:?}");
        assert_eq!(ran.stderr, "", "{args:?}");
    }
}

#[test]
fn compile_writes_an_image_without_running_the_script() {
    let image = image("image-quiet", r#"print("should not print")"#, &[]);
    assert!(fs::metadata(&image).is_ok_and(|file| file.len() > 0));
    // Only running the image prints.
    assert_eq!(run(&[&image]).printed(), ["should not print"]);
}

#[test]
fn an_image_runs_as_compiled_whatever_the_optimizations_asked_for() {
    let run_under_small_budget = |args: &[&str]| {
        run_configured(
            Driver::new(),
            |configuration| configuration.fuel = NonZeroUsize::new(5),
            args,
        )
        .exit
    };
    // Unfolded, FOLDABLE exceeds the budget; folded, it fits.
    let unfolded = image("image-unfolded", FOLDABLE, &["-O", "none"]);
    let folded = image("image-folded", FOLDABLE, &["-O", "all"]);
    assert_eq!(
        run_under_small_budget(&["run", "-O", "all", &unfolded]),
        Exit::ScriptFailed
    );
    assert_eq!(
        run_under_small_budget(&["run", "-O", "none", &folded]),
        Exit::Success
    );
}

#[test]
fn a_runtime_error_in_an_image_fails_it() {
    let image = image("image-raises", "1 / 0", &[]);
    let ran = run(&[&image]);
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
    assert!(ran.stderr.contains("Division by zero"), "{}", ran.stderr);
}

#[test]
fn compile_reports_diagnostics_and_writes_nothing() {
    let script = script("image-broken.frst", "nope");
    let output = scratch_path("image-broken");
    let ran = run(&["compile", "--color", "never", &script, "-o", &output]);
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
    assert!(
        ran.stderr.contains("`nope` is not defined"),
        "{}",
        ran.stderr
    );
    assert!(fs::metadata(&output).is_err(), "no image is written");
}

#[test]
fn compile_needs_an_output() {
    let script = script("image-no-output.frst", "1");
    let ran = run(&["compile", &script]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
}

#[test]
fn an_unwritable_output_is_a_usage_error() {
    let script = script("image-unwritable.frst", "1");
    let output = scratch_path("no-such-dir/image");
    let ran = run(&["compile", &script, "-o", &output]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    assert!(ran.stderr.contains("cannot write"), "{}", ran.stderr);
}

#[test]
fn check_and_compile_take_scripts_not_images() {
    let image = image("image-not-a-script", "1", &[]);
    let output = scratch_path("image-recompiled");
    for args in [
        vec!["check", image.as_str()],
        vec!["compile", image.as_str(), "-o", output.as_str()],
    ] {
        let ran = run(&args);
        assert_eq!(ran.exit, Exit::UsageError, "{args:?}: {ran:?}");
        assert!(
            ran.stderr.contains("is a compiled image, not a script"),
            "{args:?}: {}",
            ran.stderr
        );
    }
}

#[test]
fn an_image_from_another_version_is_a_usage_error() {
    let path = image("image-other-version", "1", &[]);
    // Swap the version the image records for another of the same length.
    let version = env!("CARGO_PKG_VERSION").as_bytes();
    let other: Vec<u8> = version
        .iter()
        .map(|&byte| if byte.is_ascii_digit() { b'9' } else { byte })
        .collect();
    assert_ne!(
        version,
        other.as_slice(),
        "the test needs a different version"
    );
    let mut bytes = fs::read(&path).unwrap();
    let at = bytes
        .windows(version.len())
        .position(|window| window == version)
        .expect("an image records its version");
    bytes[at..at + version.len()].copy_from_slice(&other);
    fs::write(&path, bytes).unwrap();

    let ran = run(&[&path]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    let other = String::from_utf8(other).unwrap();
    assert!(
        ran.stderr.contains(&format!("compiled by Frost {other}")),
        "{}",
        ran.stderr
    );
}

#[test]
fn a_damaged_image_is_a_usage_error() {
    let path = image("image-damaged", r#"print("a fairly long script")"#, &[]);
    let bytes = fs::read(&path).unwrap();
    fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
    let ran = run(&[&path]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    assert!(ran.stderr.contains("it is damaged"), "{}", ran.stderr);
}

#[test]
fn a_script_that_is_not_text_is_a_usage_error() {
    let path = scratch_path("not-text.frst");
    fs::write(&path, [0x66, 0xff, 0xfe]).unwrap();
    let ran = run(&[&path]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    assert!(ran.stderr.contains("not UTF-8 text"), "{}", ran.stderr);
}

// --- Interactive sessions ---

/// What one interactive session produced: the run, and each outcome its
/// frontend was shown.
#[derive(Debug)]
struct Session {
    ran: Ran,
    outcomes: Vec<Result<Value, ReplError>>,
}

impl Session {
    /// Each outcome's value. Every input must have succeeded.
    fn values(&self) -> Vec<Value> {
        self.outcomes
            .iter()
            .map(|outcome| {
                outcome
                    .clone()
                    .unwrap_or_else(|error| panic!("an input failed: {error}\n{self:?}"))
            })
            .collect()
    }
}

/// Run the command line `args` on `driver`, its configuration adjusted by
/// `configure`, with sessions starting from `settings` on a scripted frontend
/// reading `segments`.
fn run_session_configured(
    driver: Driver,
    settings: ReplSettings,
    segments: &[&str],
    configure: impl FnOnce(&mut VmRuntimeConfiguration),
    args: &[&str],
) -> Session {
    let transcript = Transcript::default();
    let settings = {
        let segments: Vec<String> = segments.iter().map(ToString::to_string).collect();
        let transcript = transcript.clone();
        settings.with_frontend(move || {
            Box::new(ScriptedFrontend::new(segments.clone()).with_transcript(transcript.clone()))
        })
    };
    let ran = run_configured(driver.with_repl(settings), configure, args);
    Session {
        ran,
        outcomes: transcript.outcomes(),
    }
}

/// [`run_session_configured`] on the default driver and settings.
fn run_session(segments: &[&str], args: &[&str]) -> Session {
    run_session_configured(Driver::new(), ReplSettings::new(), segments, |_| {}, args)
}

#[test]
fn repl_starts_an_interactive_session() {
    let session = run_session(&["def x = 20", "x + 1"], &["repl"]);
    assert_eq!(session.ran.exit, Exit::Success, "{session:?}");
    assert_eq!(session.values(), [Value::Null, Value::Int(21)]);
    assert_eq!(
        (session.ran.stdout.as_str(), session.ran.stderr.as_str()),
        ("", ""),
        "the session's outcomes go to its frontend"
    );
}

#[test]
fn no_arguments_away_from_a_terminal_run_the_script_on_standard_input() {
    // `Driver::run` takes standard input not to be a terminal.
    let session = run_session_configured(Driver::new(), ReplSettings::new(), &["1"], |_| {}, &[]);
    assert!(session.outcomes.is_empty(), "no session: {session:?}");
    assert_eq!(session.ran.exit, Exit::Success, "{session:?}");
    let ran = run_with_stdin("print(1 + 2)\nprint(args)", &[]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert_eq!(ran.printed(), ["3", "[]"]);
}

#[test]
fn a_sessions_prints_go_to_stdout_and_its_results_to_the_frontend() {
    let session = run_session(&["print('a')", "1", "print('b'); 2"], &["repl"]);
    assert_eq!(session.ran.printed(), ["a", "b"], "{session:?}");
    assert_eq!(
        session.values(),
        [Value::Null, Value::Int(1), Value::Int(2)]
    );
}

#[test]
fn a_failed_input_is_shown_on_the_frontend_and_the_session_carries_on() {
    let session = run_session(&["nope", "1 / 0", "3"], &["repl"]);
    assert_eq!(session.ran.exit, Exit::Success, "{session:?}");
    assert_eq!(session.ran.stderr, "", "{session:?}");
    assert!(
        matches!(
            session.outcomes.as_slice(),
            [
                Err(ReplError::Compile(_)),
                Err(ReplError::Run(_)),
                Ok(Value::Int(3)),
            ]
        ),
        "{session:?}"
    );
}

#[test]
fn each_session_starts_afresh() {
    // The frontend is made anew for each session, or the second would read
    // nothing; and bindings do not carry over, or its `x` would be bound.
    let transcript = Transcript::default();
    let settings = {
        let transcript = transcript.clone();
        ReplSettings::new().with_frontend(move || {
            Box::new(ScriptedFrontend::new(["x", "def x = 1"]).with_transcript(transcript.clone()))
        })
    };
    let driver = Driver::new().with_repl(settings);
    for _ in 0..2 {
        run_configured(driver.clone(), |_| {}, &["repl"]);
    }
    let outcomes = transcript.outcomes();
    assert!(
        matches!(
            outcomes.as_slice(),
            [
                Err(ReplError::Compile(_)),
                Ok(Value::Null),
                Err(ReplError::Compile(_)),
                Ok(Value::Null),
            ]
        ),
        "{outcomes:?}"
    );
}

#[test]
fn a_session_uses_the_drivers_configuration_and_the_chosen_optimizations() {
    let importer = ImporterBuilder::new()
        .with_extension(Extension::new("answer", Value::Int(42)).unwrap())
        .unwrap()
        .build();
    let driver = Driver::new().with_importer(importer);
    let segments = ["import('ext.answer')", FOLDABLE];
    let small_budget = |configuration: &mut VmRuntimeConfiguration| {
        configuration.fuel = NonZeroUsize::new(5);
    };
    // Folded, FOLDABLE fits the budget; unfolded, it runs out.
    let folded = run_session_configured(
        driver.clone(),
        ReplSettings::new(),
        &segments,
        small_budget,
        &["repl"],
    );
    assert_eq!(folded.values(), [Value::Int(42), Value::Int(0)]);
    let unfolded = run_session_configured(
        driver,
        ReplSettings::new(),
        &segments,
        small_budget,
        &["repl", "-O", "none"],
    );
    match unfolded.outcomes.as_slice() {
        [Ok(Value::Int(42)), Err(ReplError::Run(error))] => {
            assert!(error.message().contains("fuel"), "{}", error.message());
        }
        other => panic!("the budget should run out, but gave {other:?}"),
    }
}

#[test]
fn a_session_whose_frontend_fails_is_a_usage_error() {
    struct Broken;
    impl Frontend for Broken {
        fn read_segment(&mut self) -> io::Result<Option<String>> {
            Err(io::Error::other("input is gone"))
        }
        fn render(&mut self, _: Result<&Value, &ReplError>) -> io::Result<()> {
            panic!("nothing was read, so nothing should be rendered");
        }
        fn render_text(&mut self, _: &str) -> io::Result<()> {
            panic!("nothing was read, so nothing should be rendered");
        }
    }
    let driver = Driver::new().with_repl(ReplSettings::new().with_frontend(|| Box::new(Broken)));
    let ran = run_configured(driver, |_| {}, &["repl"]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    assert!(ran.stderr.contains("input is gone"), "{}", ran.stderr);
}

#[test]
fn a_session_whose_frontend_brings_a_refused_metacommand_is_a_usage_error() {
    /// Claims `:help`, which is the REPL's own.
    struct Usurper;
    impl Frontend for Usurper {
        fn read_segment(&mut self) -> io::Result<Option<String>> {
            panic!("a refused frontend should read nothing");
        }
        fn render(&mut self, _: Result<&Value, &ReplError>) -> io::Result<()> {
            panic!("a refused frontend should be shown nothing");
        }
        fn render_text(&mut self, _: &str) -> io::Result<()> {
            panic!("a refused frontend should be shown nothing");
        }
        fn metacommands(&self) -> Vec<MetacommandSpec> {
            vec![MetacommandSpec::new("help", "Help, but mine")]
        }
    }
    let driver = Driver::new().with_repl(ReplSettings::new().with_frontend(|| Box::new(Usurper)));
    let ran = run_configured(driver, |_| {}, &["repl"]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    assert!(
        ran.stderr
            .contains("`:help` would replace a built-in metacommand"),
        "{}",
        ran.stderr
    );
}

#[test]
fn a_session_ends_at_quit() {
    let session = run_session(&["1", ":quit", "2"], &["repl"]);
    assert_eq!(session.ran.exit, Exit::Success, "{session:?}");
    assert_eq!(session.values(), [Value::Int(1)]);
}

#[test]
fn every_session_starts_with_the_seeded_bindings() -> Result<(), InvalidName> {
    // A seed shadows a global, and a session may rebind it; the next session
    // starts from the seed again.
    let settings = ReplSettings::new()
        .with_binding("answer", Value::Int(41))?
        .with_bindings([("id", Value::from("seeded"))])?;
    for _ in 0..2 {
        let session = run_session_configured(
            Driver::new(),
            settings.clone(),
            &["[answer, id]", "def answer = answer + 1", "answer"],
            |_| {},
            &["repl"],
        );
        assert_eq!(
            session.values(),
            [
                Value::array([Value::Int(41), Value::from("seeded")]),
                Value::Null,
                Value::Int(42),
            ]
        );
    }
    Ok(())
}

#[test]
fn every_session_keeps_the_chosen_number_of_recent_results() {
    let segments = ["1", "2", "results"];
    let chosen = run_session_configured(
        Driver::new(),
        ReplSettings::new().with_results_kept(1),
        &segments,
        |_| {},
        &["repl"],
    );
    assert_eq!(
        chosen.values(),
        [Value::Int(1), Value::Int(2), Value::array([2])]
    );
    // Unchosen, sessions keep the REPL's default.
    let default = run_session(&segments, &["repl"]);
    assert_eq!(
        default.values(),
        [Value::Int(1), Value::Int(2), Value::array([1, 2])]
    );
}

#[test]
fn a_seed_name_frost_cannot_refer_to_is_refused() {
    for name in ["if", "and", "$1", "two words", "my-name", "1x", ""] {
        let refused = ReplSettings::new()
            .with_binding(name, Value::Null)
            .expect_err(name);
        assert_eq!(refused.name(), name);
    }
}

// --- Listing bytecode ---

#[test]
fn list_shows_a_scripts_bytecode_without_running_it() {
    let source = r"
        def x = [1, 2]
        print(x)
    ";
    let path = script("list.frst", source);
    let ran = run(&["list", &path]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert!(
        ran.stdout.starts_with("function <main>\n"),
        "{}",
        ran.stdout
    );
    assert!(ran.stdout.contains("; print"), "{}", ran.stdout);
    assert!(
        !ran.stdout.contains('\x1b'),
        "no color by default: {}",
        ran.stdout
    );
    assert_eq!(ran.stderr, "");
}

#[test]
fn list_follows_the_optimization_switches() {
    // Folded, FOLDABLE compiles to its value alone; unfolded, it calls.
    let path = script("list-foldable.frst", FOLDABLE);
    let folded = run(&["list", &path]);
    let unfolded = run(&["list", "-O", "none", &path]);
    assert!(!folded.stdout.contains("Call"), "{}", folded.stdout);
    assert!(unfolded.stdout.contains("Call"), "{}", unfolded.stdout);
}

#[test]
fn list_shows_an_images_bytecode_as_compiled() {
    let source = r"
        defn f(n) -> n + 1
        f(2)
    ";
    let script = script("list-image.frst", source);
    let image = image("list-image", source, &["-O", "none"]);
    // Whatever the command line now asks for, the image lists as it was compiled.
    let from_image = run(&["list", &image]);
    let from_script = run(&["list", "-O", "none", &script]);
    assert_eq!(from_image.exit, Exit::Success, "{from_image:?}");
    assert_eq!(from_image.stdout, from_script.stdout);
}

#[test]
fn list_colors_on_request() {
    let path = script("list-color.frst", "1");
    let ran = run(&["list", "--color", "always", &path]);
    assert!(ran.stdout.contains("\x1b["), "{:?}", ran.stdout);
}

#[test]
fn list_reports_a_script_that_does_not_compile() {
    let path = script("list-broken.frst", "nope");
    let ran = run(&["list", "--color", "never", &path]);
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
    assert!(
        ran.stderr.contains("`nope` is not defined"),
        "{}",
        ran.stderr
    );
    assert_eq!(ran.stdout, "");
}

#[test]
fn list_refuses_a_damaged_image() {
    let path = image("list-damaged", r#"print("a fairly long script")"#, &[]);
    let bytes = fs::read(&path).unwrap();
    fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
    let ran = run(&["list", &path]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    assert!(ran.stderr.contains("cannot list"), "{}", ran.stderr);
}

// --- Optimizations ---

/// A pure recursion that folds away entirely when constant folding is on, and
/// otherwise makes eleven calls at runtime.
const FOLDABLE: &str = "(fn f(n) -> if n == 0: 0 else: f(n - 1))(10)";

/// Run [`FOLDABLE`] with `args` under a call budget too small to run it
/// unfolded: it succeeds only if it compiled with constant folding.
fn folds(args: &[&str]) -> bool {
    let args: Vec<&str> = args.iter().copied().chain(["-e", FOLDABLE]).collect();
    let ran = run_configured(
        Driver::new(),
        |configuration| configuration.fuel = NonZeroUsize::new(5),
        &args,
    );
    match ran.exit {
        Exit::Success => true,
        Exit::ScriptFailed => false,
        Exit::UsageError => panic!("{args:?}: {ran:?}"),
    }
}

#[test]
fn optimizations_follow_the_settings() {
    assert!(folds(&[]), "every optimization is on by default");
    assert!(folds(&["-O", "all"]));
    assert!(!folds(&["-O", "none"]));
    assert!(!folds(&["--optimize", "none"]));
    assert!(!folds(&["-O", "preset=none"]));
    assert!(folds(&["-O", "none,constant-fold=true"]));
    assert!(!folds(&["-O", "constant-fold=false"]));
    assert!(!folds(&[
        "-O",
        "branch-eliminate=false,constant-fold=false"
    ]));
    // `OptimizationOptions::with_settings` reads each `-O`; its own tests cover
    // the settings it takes.
}

#[test]
fn optimize_may_be_given_more_than_once_each_applying_in_turn() {
    assert!(folds(&["-O", "none", "-O", "constant-fold=true"]));
    assert!(!folds(&["-O", "constant-fold=true", "-O", "none"]));
    assert!(folds(&["-O", "none", "--optimize", "all"]));
}

#[test]
fn optimizations_apply_to_a_subcommand_and_before_it() {
    let path = script("foldable.frst", FOLDABLE);
    for args in [["run", "-O", "none", &path], ["-O", "none", "run", &path]] {
        let ran = run_configured(
            Driver::new(),
            |configuration| configuration.fuel = NonZeroUsize::new(5),
            &args,
        );
        assert_eq!(ran.exit, Exit::ScriptFailed, "{args:?}: {ran:?}");
    }
}

#[test]
fn an_invalid_optimization_setting_is_a_usage_error() {
    for setting in [
        "some",
        "constant-fold",
        "nope=true",
        "constant-fold=yes",
        "preset=x",
    ] {
        let ran = run(&["-O", setting, "-e", "print(1)"]);
        assert_eq!(ran.exit, Exit::UsageError, "{setting:?}: {ran:?}");
        let refusal = OptimizationOptions::ALL.with_settings(setting).unwrap_err();
        assert_eq!(ran.stderr, format!("error: {refusal}\n"), "{setting:?}");
        assert!(ran.printed().is_empty(), "nothing runs: {setting:?}");
    }
}

#[test]
fn the_drivers_default_optimizations_apply_without_a_preset() {
    let ran = run_configured(
        Driver::new().with_optimization(OptimizationOptions::NONE),
        |configuration| configuration.fuel = NonZeroUsize::new(5),
        &["-e", FOLDABLE],
    );
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
}

#[test]
fn every_optimization_can_be_set_by_name() {
    for optimization in Optimization::ALL {
        for value in ["true", "false"] {
            let setting = format!("{}={value}", optimization.name());
            let ran = run(&["-O", &setting, "-e", "1"]);
            assert_eq!(ran.exit, Exit::Success, "{setting}: {ran:?}");
        }
    }
}

// --- The command line itself ---

#[test]
fn help_and_version_go_to_stdout() {
    let ran = run_configured(
        Driver::new().with_name("my-frost").with_version("9.8.7"),
        |_| {},
        &["--version"],
    );
    assert_eq!(ran.exit, Exit::Success);
    assert_eq!(ran.stdout, "my-frost 9.8.7\n");
    assert_eq!(ran.stderr, "");

    let ran = run_configured(Driver::new().with_name("my-frost"), |_| {}, &["--help"]);
    assert_eq!(ran.exit, Exit::Success);
    assert!(ran.stdout.contains("Usage: my-frost"), "{}", ran.stdout);
}

#[test]
fn help_states_the_default_optimizations() {
    let custom = OptimizationOptions::NONE
        .with(Optimization::ConstantFold, true)
        .with(Optimization::CaptureHoist, true);
    for (driver, default) in [
        (Driver::new(), "[default: all]"),
        (
            Driver::new().with_optimization(OptimizationOptions::NONE),
            "[default: none]",
        ),
        (
            Driver::new().with_optimization(custom),
            "[default: none,constant-fold=true,capture-hoist=true]",
        ),
    ] {
        let help = run_configured(driver.clone(), |_| {}, &["--help"]).stdout;
        let start = help.find("--optimize").expect("the help lists --optimize");
        let end = help.find("--no-args").expect("the help lists --no-args");
        let entry = &help[start..end];
        assert!(entry.contains(default), "{entry}");
        // The default is itself a setting `-O` takes.
        let setting = default
            .trim_start_matches("[default: ")
            .trim_end_matches(']');
        let ran = run(&["-O", setting, "-e", "1"]);
        assert_eq!(ran.exit, Exit::Success, "{setting}: {ran:?}");
    }
}

#[test]
fn advanced_options_appear_only_in_the_long_help() {
    let short = run(&["-h"]).stdout;
    let long = run(&["--help"]).stdout;
    for option in ["--optimize", "--no-args"] {
        assert!(!short.contains(option), "`-h` hides {option}:\n{short}");
        assert!(long.contains(option), "`--help` shows {option}:\n{long}");
    }
    let advanced = long
        .find("Advanced options:")
        .expect("`--help` has an advanced section");
    assert!(long[advanced..].contains("--optimize"), "{long}");
    assert!(long[advanced..].contains("--no-args"), "{long}");
    // The rest stay in the short help.
    for option in ["--eval", "--color"] {
        assert!(short.contains(option), "`-h` shows {option}:\n{short}");
    }
}

#[test]
fn usage_names_the_driver() {
    let ran = run_configured(Driver::new().with_name("my-frost"), |_| {}, &["--help"]);
    let expected = "Usage: my-frost [OPTIONS] [SCRIPT [ARGS]...]
       my-frost [OPTIONS] -e CODE [ARGS]...
       my-frost [OPTIONS] <COMMAND>";
    assert!(ran.stdout.contains(expected), "{}", ran.stdout);
    let ran = run_configured(
        Driver::new().with_name("my-frost"),
        |_| {},
        &["run", "--help"],
    );
    assert!(
        ran.stdout
            .contains("Usage: my-frost run [OPTIONS] SCRIPT [ARGS]..."),
        "{}",
        ran.stdout
    );
}

#[test]
fn global_options_may_come_before_a_subcommand() {
    let path = script("before-subcommand.frst", "nope");
    let ran = run(&["--color", "never", "check", &path]);
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
    assert!(
        ran.stderr.contains("`nope` is not defined"),
        "{}",
        ran.stderr
    );
    let ran = run(&["--color", "always", "check", &path]);
    assert!(ran.stderr.contains('\x1b'), "colored: {:?}", ran.stderr);
}

#[test]
fn a_bad_command_line_is_a_usage_error() {
    for args in [
        vec!["--no-such-flag"],
        vec!["-O", "some"],
        vec!["-O", "no-such-optimization=true", "-e", "1"],
        vec!["-e", "1", "run", "script.frst"],
        vec!["run"],
        vec!["check"],
        vec!["repl", "script.frst"],
        vec!["check", "a.frst", "b.frst"],
    ] {
        let ran = run(&args);
        assert_eq!(ran.exit, Exit::UsageError, "{args:?}: {ran:?}");
        assert!(!ran.stderr.is_empty(), "{args:?}");
        assert!(ran.printed().is_empty(), "{args:?}");
    }
}

#[test]
fn exit_statuses_are_conventional() {
    assert_eq!(Exit::Success.code(), 0);
    assert_eq!(Exit::ScriptFailed.code(), 1);
    assert_eq!(Exit::UsageError.code(), 2);
}

// --- A script's arguments ---

#[test]
fn a_script_gets_the_arguments_after_it_as_args() {
    let path = script("args.frst", "print(args)");
    for args in [
        vec![path.as_str(), "a", "b c"],
        vec!["run", path.as_str(), "a", "b c"],
    ] {
        let ran = run(&args);
        assert_eq!(ran.exit, Exit::Success, "{args:?}: {ran:?}");
        assert_eq!(ran.printed(), [r#"[ "a", "b c" ]"#], "{args:?}");
    }
}

#[test]
fn a_script_given_no_arguments_gets_an_empty_args() {
    let path = script("no-args.frst", "print(args)");
    assert_eq!(run(&[&path]).printed(), ["[]"]);
    assert_eq!(run(&["-e", "print(args)"]).printed(), ["[]"]);
}

#[test]
fn eval_gets_the_arguments_after_it_as_args() {
    let ran = run(&["-e", "print(args)", "a", "b"]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert_eq!(ran.printed(), [r#"[ "a", "b" ]"#]);
}

#[test]
fn everything_after_the_script_is_its_even_what_looks_like_an_option() {
    let path = script("option-like-args.frst", "print(args)");
    for (args, expected) in [
        (vec![path.as_str(), "-e", "1"], r#"[ "-e", "1" ]"#),
        (
            vec![path.as_str(), "--color", "never"],
            r#"[ "--color", "never" ]"#,
        ),
        (
            vec!["run", path.as_str(), "-O", "none", "x"],
            r#"[ "-O", "none", "x" ]"#,
        ),
        (vec![path.as_str(), "run", "-"], r#"[ "run", "-" ]"#),
    ] {
        let ran = run(&args);
        assert_eq!(ran.exit, Exit::Success, "{args:?}: {ran:?}");
        assert_eq!(ran.printed(), [expected], "{args:?}");
    }
}

#[test]
fn a_script_may_bind_args_itself() {
    let ran = run(&["-e", "def args = 'mine'\nprint(args)", "a"]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert_eq!(ran.printed(), ["mine"]);
}

#[test]
fn an_image_gets_its_arguments_as_args() {
    let image = image("args-image", "print(args)", &[]);
    let ran = run(&["run", &image, "x"]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert_eq!(ran.printed(), [r#"[ "x" ]"#]);
}

#[test]
fn no_args_leaves_the_name_unbound() {
    let ran = run(&["--no-args", "--color", "never", "-e", "print(args)"]);
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
    assert!(
        ran.stderr.contains("`args` is not defined"),
        "{}",
        ran.stderr
    );
    // So a script may give it a meaning of its own.
    let ran = run(&["--no-args", "-e", "def args = 1\nprint(args)"]);
    assert_eq!(ran.printed(), ["1"], "{ran:?}");
}

#[test]
fn no_args_refuses_arguments_for_the_script() {
    let ran = run(&["--no-args", "-e", "print(1)", "a"]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    assert_eq!(
        ran.stderr,
        "error: `--no-args` takes no arguments for the script\n"
    );
    assert!(ran.printed().is_empty(), "nothing runs");
}

#[test]
fn an_image_runs_as_compiled_whatever_no_args_says() {
    let image = image("no-args-image", "print(args)", &[]);
    let ran = run(&["run", "--no-args", &image]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert_eq!(ran.printed(), ["[]"]);
}

// --- Standard input ---

#[test]
fn a_script_of_dash_is_read_from_standard_input() {
    let source = "print(1 + 2)\nprint(args)";
    for args in [vec!["-", "a"], vec!["run", "-", "a"]] {
        let ran = run_with_stdin(source, &args);
        assert_eq!(ran.exit, Exit::Success, "{args:?}: {ran:?}");
        assert_eq!(ran.printed(), ["3", r#"[ "a" ]"#], "{args:?}");
    }
}

#[test]
fn every_subcommand_reads_a_script_of_dash_from_standard_input() {
    let ran = run_with_stdin("nope", &["check", "--color", "never", "-"]);
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
    assert!(
        ran.stderr.contains("<stdin>"),
        "named `<stdin>`: {}",
        ran.stderr
    );

    let listed = run_with_stdin("1 + 2", &["list", "-O", "none", "-"]);
    assert_eq!(listed.exit, Exit::Success, "{listed:?}");
    assert!(listed.stdout.contains("Add"), "{}", listed.stdout);

    let tree = run_with_stdin("1 + 2", &["ast", "-"]);
    assert_eq!(tree.exit, Exit::Success, "{tree:?}");
    assert!(tree.stdout.contains("BinOp"), "{}", tree.stdout);

    let output = scratch_path("stdin-image");
    let compiled = run_with_stdin("print(7)", &["compile", "-", "-o", &output]);
    assert_eq!(compiled.exit, Exit::Success, "{compiled:?}");
    assert_eq!(run(&["run", &output]).printed(), ["7"]);
}

#[test]
fn an_image_on_standard_input_runs() {
    let image = image("stdin-runs-image", "print(8)", &[]);
    let bytes = fs::read(&image).unwrap();
    let (stdout, stderr) = (Buffer::default(), Buffer::default());
    let exit = Driver::new().run(
        ["frost", "-"],
        bytes.as_slice(),
        stdout.clone(),
        stderr.clone(),
    );
    assert_eq!(exit, Exit::Success, "{}", stderr.contents());
    assert_eq!(stdout.contents(), "8\n");
}

#[test]
fn failing_to_read_standard_input_is_a_usage_error() {
    struct Broken;
    impl io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("input is gone"))
        }
    }
    let (stdout, stderr) = (Buffer::default(), Buffer::default());
    let exit = Driver::new().run(["frost", "-"], Broken, stdout.clone(), stderr.clone());
    assert_eq!(exit, Exit::UsageError);
    assert_eq!(
        stderr.contents(),
        "error: cannot read <stdin>: input is gone\n"
    );
}

#[test]
fn standard_input_is_read_only_for_a_script_of_dash() {
    let path = script("ignores-stdin.frst", "print('from the file')");
    let ran = run_with_stdin("print('from stdin')", &[&path, "-"]);
    assert_eq!(ran.printed(), ["from the file"], "{ran:?}");
}

// --- Syntax trees ---

#[test]
fn ast_shows_a_scripts_syntax_tree_without_running_it() {
    let path = script("ast.frst", "print(1 + 2)");
    let ran = run(&["ast", &path]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert!(ran.stdout.contains("BinOp"), "{}", ran.stdout);
    assert!(!ran.stdout.contains("\n3\n"), "nothing ran: {}", ran.stdout);
    assert_eq!(ran.stderr, "");
}

#[test]
fn ast_does_not_compile_the_script() {
    // An unbound name parses, though it would not compile.
    let path = script("ast-unbound.frst", "nope");
    let ran = run(&["ast", &path]);
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
}

#[test]
fn ast_reports_a_script_that_does_not_parse() {
    let path = script("ast-broken.frst", "def");
    let ran = run(&["ast", "--color", "never", &path]);
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
    assert!(ran.stderr.contains("ast-broken.frst"), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
}

#[test]
fn ast_takes_scripts_not_images() {
    let image = image("ast-image", "1", &[]);
    let ran = run(&["ast", &image]);
    assert_eq!(ran.exit, Exit::UsageError, "{ran:?}");
    assert!(
        ran.stderr.contains("is a compiled image, not a script"),
        "{}",
        ran.stderr
    );
}

// --- The embedder's configuration ---

#[test]
fn prints_go_to_stdout_in_place_of_the_configured_sink() {
    let bypassed = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = {
        let bypassed = Arc::clone(&bypassed);
        move |text: &str| bypassed.lock().unwrap().push(text.to_string())
    };
    let ran = run_configured(
        Driver::new(),
        |configuration| configuration.print_sink = Arc::new(sink),
        &["-e", r#"print("to stdout")"#],
    );
    assert_eq!(ran.printed(), ["to stdout"]);
    assert!(bypassed.lock().unwrap().is_empty());
}

#[test]
fn scripts_import_from_the_drivers_importer() {
    let importer = ImporterBuilder::new()
        .with_extension(Extension::new("answer", Value::Int(42)).unwrap())
        .unwrap()
        .build();
    let ran = run_configured(
        Driver::new().with_importer(importer),
        |_| {},
        &["-e", "print(import('ext.answer'))"],
    );
    assert_eq!(ran.exit, Exit::Success, "{ran:?}");
    assert_eq!(ran.printed(), ["42"]);
    // The default driver can import nothing.
    let ran = run(&["-e", "import('ext.answer')"]);
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
}
