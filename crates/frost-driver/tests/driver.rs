//! The driver end to end: command lines in; exit statuses, a script's printed
//! output, and the driver's own output out.

use std::fs;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use frost_compile::OptimizationOptions;
use frost_driver::{Driver, Exit};
use frost_runtime::{Extension, ImporterBuilder, Value, VmRuntimeConfiguration};

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

/// Run `driver` on `args` (after the program name), capturing its output.
/// `configure` adjusts the configuration it runs scripts under.
fn run_configured(
    driver: Driver,
    configure: impl FnOnce(&mut VmRuntimeConfiguration),
    args: &[&str],
) -> Ran {
    let mut configuration = VmRuntimeConfiguration::default();
    configure(&mut configuration);
    let (stdout, stderr) = (Buffer::default(), Buffer::default());
    let exit = driver.with_configuration(configuration).run(
        ["frost"].iter().chain(args),
        stdout.clone(),
        stderr.clone(),
    );
    Ran {
        exit,
        stdout: stdout.contents(),
        stderr: stderr.contents(),
    }
}

fn run(args: &[&str]) -> Ran {
    run_configured(Driver::new(), |_| {}, args)
}

/// Write `source` to a script file named `name`, returning its path.
fn script(name: &str, source: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("frost-driver-tests");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, source).unwrap();
    path.to_string_lossy().into_owned()
}

// --- Running scripts ---

#[test]
fn run_runs_a_script_file() {
    let path = script("run.frst", r#"print("hello"); print(1 + 2)"#);
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
    let ran = run(&[
        "-e",
        r#"print("before"); defn f() -> 1 / 0; f(); print("after")"#,
    ]);
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
    for args in [
        vec!["--color", "never", path.as_str()],
        vec!["--color", "never", "-e", "print(1); nope"],
    ] {
        let ran = run(&args);
        assert_eq!(ran.exit, Exit::ScriptFailed, "{args:?}");
        assert!(ran.printed().is_empty(), "nothing runs: {ran:?}");
        assert!(!ran.stderr.is_empty(), "{args:?}");
    }
    let ran = run(&["--color", "never", "-e", "print(1); nope"]);
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
fn optimizations_follow_the_presets_and_switches() {
    assert!(folds(&[]), "every optimization is on by default");
    assert!(folds(&["-O", "all"]));
    assert!(!folds(&["-O", "none"]));
    assert!(!folds(&["--optimize", "none"]));
    assert!(folds(&["-O", "none", "--enable", "constant-fold"]));
    assert!(!folds(&["--disable", "constant-fold"]));
    assert!(!folds(&["--disable", "branch-eliminate,constant-fold"]));
    assert!(folds(&[
        "-O",
        "none",
        "--enable",
        "capture-hoist,constant-fold"
    ]));
    // The switches apply to a subcommand too.
    let path = script("foldable.frst", FOLDABLE);
    let ran = run_configured(
        Driver::new(),
        |configuration| configuration.fuel = NonZeroUsize::new(5),
        &["run", "-O", "none", &path],
    );
    assert_eq!(ran.exit, Exit::ScriptFailed, "{ran:?}");
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
fn an_optimization_both_enabled_and_disabled_is_a_usage_error() {
    let ran = run(&[
        "--enable",
        "capture-hoist",
        "--disable",
        "capture-hoist",
        "-e",
        "1",
    ]);
    assert_eq!(ran.exit, Exit::UsageError);
    assert!(
        ran.stderr
            .contains("`capture-hoist` is both enabled and disabled"),
        "{}",
        ran.stderr
    );
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
    let custom = OptimizationOptions {
        constant_fold: true,
        capture_hoist: true,
        ..OptimizationOptions::NONE
    };
    for (driver, default) in [
        (Driver::new(), "[default: all]"),
        (
            Driver::new().with_optimization(OptimizationOptions::NONE),
            "[default: none]",
        ),
        (
            Driver::new().with_optimization(custom),
            "[default: constant-fold,capture-hoist]",
        ),
    ] {
        for flag in ["-h", "--help"] {
            let help = run_configured(driver.clone(), |_| {}, &[flag]).stdout;
            let start = help.find("--optimize").expect("the help lists --optimize");
            let end = help.find("--enable <").expect("the help lists --enable");
            let entry = &help[start..end];
            assert!(entry.contains(default), "{flag}: {entry}");
        }
    }
}

#[test]
fn a_bad_command_line_is_a_usage_error() {
    for args in [
        vec![],
        vec!["--no-such-flag"],
        vec!["-O", "some"],
        vec!["--enable", "no-such-optimization", "-e", "1"],
        vec!["script.frst", "-e", "1"],
        vec!["run"],
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
