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
    let path = scratch_path(name);
    fs::write(&path, source).unwrap();
    path
}

/// A path named `name` in the tests' scratch directory, with nothing written
/// there yet.
fn scratch_path(name: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("frost-driver-tests");
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

// --- Compiled images ---

#[test]
fn an_image_runs_as_its_script_does() {
    let source = r#"defn fib(n) -> if n < 2: n else: fib(n - 1) + fib(n - 2)
        print(fib(15))
        print([0.0, -0.0, {a: [1, "two"]}, x'00ff'])"#;
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
fn every_optimization_has_a_switch() {
    // No `..`: a new option fails to compile here until its switch is listed.
    let OptimizationOptions {
        constant_fold: _,
        constant_propagate: _,
        branch_eliminate: _,
        capture_hoist: _,
        consume_locals: _,
        deduplicate_constants: _,
    } = OptimizationOptions::ALL;
    for name in [
        "constant-fold",
        "constant-propagate",
        "branch-eliminate",
        "capture-hoist",
        "consume-locals",
        "deduplicate-constants",
    ] {
        for switch in ["--enable", "--disable"] {
            let ran = run(&[switch, name, "-e", "1"]);
            assert_eq!(ran.exit, Exit::Success, "{switch} {name}: {ran:?}");
        }
    }
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
