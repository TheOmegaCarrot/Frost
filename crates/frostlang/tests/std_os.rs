//! `std.os`, from Frost source: environment variables, the process ID, the
//! platform, sleeping, and running programs.
//!
//! Each case runs with only `std.os` installed, bound as `os`. The cases that
//! run programs use POSIX tools, so they run only on Unix.

use crate::script;

use std::time::{Duration, Instant};

use frostlang::stdlib::StdlibConfig;
use frostlang::{FrostMap, ImporterBuilder, MapKey, Stdlib, Value, stdlib};
use script::assertions::{Library, library_assertions};
use script::{Script, UNOPTIMIZED};

library_assertions!(Library::module(stdlib::os, "os"));

// --- The module ---

#[test]
fn the_module_holds_its_functions_and_values() {
    assert_values(&[(
        "sorted(keys(os))",
        "['arch', 'env', 'getenv', 'pid', 'platform', 'run', 'sleep', 'stderr', 'stdin', 'stdout']",
    )]);
}

// --- Standard streams ---
//
// The streams are the test process's own, so these cases read nothing and write
// nothing visible. What streams do is tested through `std.string`'s buffers and
// `std.fs`'s files, which share their implementation.

#[test]
fn the_standard_streams_flow_without_positions_or_closing() {
    assert_values(&[
        (
            "sorted(keys(os.stdin))",
            "['eof', 'read_bytes', 'read_line', 'read_one', 'read_rest', 'read_rest_bytes']",
        ),
        ("sorted(keys(os.stdout))", "['flush', 'write', 'writeln']"),
        ("sorted(keys(os.stderr))", "['flush', 'write', 'writeln']"),
    ]);
}

#[test]
fn the_standard_output_streams_take_writes() {
    for stream in ["stdout", "stderr"] {
        assert_values(&[
            (&format!("os.{stream}.write('')"), "null"),
            (&format!("os.{stream}.write(x'')"), "null"),
            (&format!("os.{stream}.flush()"), "null"),
        ]);
        assert_raises(&[(
            &format!("os.{stream}.write(1)"),
            "Function writer.write requires String or Bytes as argument 1, got Int",
        )]);
    }
}

#[test]
fn the_module_is_not_contained() {
    let stdlib = Stdlib::contained(StdlibConfig::default());
    let contained = ImporterBuilder::new().with_stdlib(stdlib).build();
    let raised = Script::new("import('std.os')").importer(contained).raises();
    assert_eq!(raised, "Could not resolve import 'std.os'");
}

// --- getenv ---

#[test]
fn getenv_reads_a_variable() {
    let expected = std::env::var("CARGO_PKG_NAME").expect("cargo sets CARGO_PKG_NAME");
    assert_eq!(
        script("os.getenv('CARGO_PKG_NAME')").run(),
        Value::from(expected)
    );
}

#[test]
fn getenv_returns_null_for_a_variable_that_is_not_set() {
    assert_eq!(
        script("os.getenv('FROST_TEST_SURELY_UNSET_9C1E')").run(),
        Value::Null
    );
    // `getenv` reads no variable by these names.
    for name in ["''", "'A=B'", r"'A\u{0}B'"] {
        let expression = format!("os.getenv({name})");
        assert_eq!(script(&expression).run(), Value::Null, "{expression}");
    }
}

#[test]
fn getenv_checks_its_argument() {
    assert_raises(&[(
        "os.getenv(1)",
        "Function std.os.getenv requires String as argument 1 (name), got Int",
    )]);
    assert_arity("getenv", "1", &[0, 2]);
}

// --- env ---

/// Each variable `getenv` can read, by name:
/// what `os.env()` gives for an environment with no duplicate names.
fn readable_environment() -> FrostMap {
    std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
        .filter(|(name, _)| !name.is_empty() && !name.contains(['=', '\0']))
        .map(|(name, value)| (MapKey::from(name), Value::from(value)))
        .collect()
}

#[test]
fn env_reads_every_variable() {
    assert_eq!(
        script("os.env()").run(),
        Value::from(readable_environment()),
        "os.env() differs from the readable environment"
    );
    // A known variable, so an environment empty on both sides cannot pass.
    let expected = std::env::var("CARGO_PKG_NAME").expect("cargo sets CARGO_PKG_NAME");
    assert_eq!(
        script("os.env().CARGO_PKG_NAME").run(),
        Value::from(expected),
        "os.env() misreads CARGO_PKG_NAME"
    );
}

#[test]
fn env_agrees_with_getenv() {
    assert_values(&[(
        r"
        def vars = os.env()
        vars @ keys() @ filter(fn name -> os.getenv(name) != vars[name])
        ",
        "[]",
    )]);
}

#[test]
fn env_takes_no_arguments() {
    assert_arity("env", "0", &[1]);
}

/// `env` leaves out each variable `getenv` reads as Null:
/// one whose name or value is not UTF-8, or whose name `getenv` refuses.
/// Setting such variables in this process is unsafe,
/// so the test runs itself again in a child process,
/// with them set in an otherwise empty environment.
#[cfg(unix)]
#[test]
fn env_and_getenv_agree_on_unreadable_variables() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::process::Command;

    const TEST: &str = "std_os::env_and_getenv_agree_on_unreadable_variables";
    const CHILD: &str = "FROST_TEST_ENV_CHILD";
    let invalid_utf8 = OsStr::from_bytes(b"\xff");
    let value_not_utf8 = "FROST_TEST_VALUE_NOT_UTF8";
    let name_not_utf8 = OsStr::from_bytes(b"FROST_TEST_NAME_\xff");
    let name_with_equals = "=FROST_TEST_NAME_WITH_EQUALS";

    if std::env::var_os(CHILD).is_some() {
        // Each unreadable variable reached the child,
        // so its absence from `os.env()` is `os.env()`'s doing.
        let seen: Vec<_> = std::env::vars_os().map(|(name, _)| name).collect();
        for name in [
            OsStr::new(value_not_utf8),
            name_not_utf8,
            OsStr::new(name_with_equals),
        ] {
            assert!(
                seen.iter().any(|seen| seen == name),
                "the child lacks {name:?}"
            );
        }
        assert_values(&[
            (
                "os.env()",
                "{FROST_TEST_ENV_CHILD: '1', FROST_TEST_READABLE: 'yes'}",
            ),
            ("os.getenv('FROST_TEST_READABLE')", "'yes'"),
            (&format!("os.getenv('{value_not_utf8}')"), "null"),
            (&format!("os.getenv('{name_with_equals}')"), "null"),
        ]);
        return;
    }

    let child = Command::new(std::env::current_exe().expect("the test binary has a path"))
        .args([TEST, "--exact", "--test-threads=1"])
        .env_clear()
        .env(CHILD, "1")
        .env("FROST_TEST_READABLE", "yes")
        .env(value_not_utf8, invalid_utf8)
        .env(name_not_utf8, "x")
        .env(name_with_equals, "x")
        .output()
        .expect("the test binary runs");
    let stdout = String::from_utf8_lossy(&child.stdout);
    let stderr = String::from_utf8_lossy(&child.stderr);
    assert!(
        child.status.success() && stdout.contains("1 passed"),
        "the child process failed or ran no test\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

// --- pid ---

#[test]
fn pid_is_this_process() {
    assert_eq!(
        script("os.pid()").run(),
        Value::Int(i64::from(std::process::id()))
    );
    assert_arity("pid", "0", &[1]);
}

// --- platform and arch ---

#[test]
fn platform_and_arch_name_the_build_target() {
    // Null where the target names no operating system.
    let platform = match std::env::consts::OS {
        "" => Value::Null,
        os => Value::from(os),
    };
    assert_eq!(script("os.platform").run(), platform, "os.platform");
    assert_eq!(
        script("os.arch").run(),
        Value::from(std::env::consts::ARCH),
        "os.arch"
    );
}

// --- sleep ---

#[test]
fn sleep_pauses_then_returns_null() {
    assert_eq!(script("os.sleep(0)").run(), Value::Null);
    // One run, so the elapsed time bounds a single sleep.
    let started = Instant::now();
    assert_eq!(script("os.sleep(50)").run_under(UNOPTIMIZED), Value::Null);
    assert!(
        started.elapsed() >= Duration::from_millis(50),
        "os.sleep(50) took {:?}",
        started.elapsed()
    );
}

#[test]
fn sleep_checks_its_argument() {
    assert_raises(&[
        (
            "os.sleep(-1)",
            "Function std.os.sleep requires argument 1 (ms) to be at least 0, got -1",
        ),
        (
            "os.sleep(1.5)",
            "Function std.os.sleep requires Int as argument 1 (ms), got Float",
        ),
    ]);
    assert_arity("sleep", "1", &[0, 2]);
}

// --- run ---

#[cfg(unix)]
mod run {
    use super::*;

    #[test]
    fn run_returns_output_and_exit_status() {
        assert_values(&[
            (
                "os.run('echo', ['hello', 'world'])",
                r"{stdout: 'hello world\n', stderr: '', exit_code: 0, signal: null}",
            ),
            ("os.run('sh', ['-c', 'echo oops >&2']).stderr", r"'oops\n'"),
            ("os.run('sh', ['-c', 'exit 3']).exit_code", "3"),
        ]);
    }

    #[test]
    fn run_passes_each_argument_whole() {
        assert_values(&[(
            r#"os.run('sh', ['-c', 'printf "%s|" "$@"', 'sh', 'a b', 'c']).stdout"#,
            "'a b|c|'",
        )]);
    }

    #[test]
    fn run_reports_a_signal_instead_of_an_exit_code() {
        let source = r"
            def r = os.run('sh', ['-c', 'kill -9 $$'])
            [r.exit_code, r.signal]
        ";
        assert_values(&[(source, "[null, 9]")]);
    }

    #[test]
    fn run_feeds_stdin() {
        assert_values(&[
            ("os.run('cat', [], {stdin: 'hello'}).stdout", "'hello'"),
            ("os.run('cat', [], {stdin: x'68690a'}).stdout", r"'hi\n'"),
            // Without `stdin`, the program reads nothing.
            ("os.run('cat', []).stdout", "''"),
            // More than a pipe holds: written while the output is read.
            (
                "len(os.run('cat', [], {stdin: tile('x', 1000000)}).stdout)",
                "1000000",
            ),
        ]);
    }

    #[test]
    fn run_sets_the_working_directory() {
        assert_values(&[("os.run('pwd', [], {cwd: '/'}).stdout", r"'/\n'")]);
    }

    #[test]
    fn run_names_a_working_directory_it_cannot_use() {
        // Not blamed on the program, which exists.
        let raised = script("os.run('true', [], {cwd: '/frost-surely-no-such-dir'})").raises();
        assert!(
            raised.starts_with(
                "Function std.os.run cannot use `/frost-surely-no-such-dir` as its working directory: "
            ),
            "{raised}"
        );
        assert_raises(&[(
            "os.run('true', [], {cwd: '/dev/null'})",
            "Function std.os.run requires its `cwd` option to be a directory, but `/dev/null` is not",
        )]);
    }

    #[test]
    fn run_adds_to_or_replaces_the_environment() {
        assert_values(&[
            (
                r#"os.run('sh', ['-c', 'echo "$FROST_VAR"'], {env: {FROST_VAR: 'value'}}).stdout"#,
                r"'value\n'",
            ),
            (
                "os.run('/usr/bin/env', [], {replace_env: {ONLY: 'this'}}).stdout",
                r"'ONLY=this\n'",
            ),
        ]);
        // `env` adds to the inherited environment.
        let inherited =
            script(r#"os.run('sh', ['-c', 'echo "$CARGO_PKG_NAME"'], {env: {X: 'y'}}).stdout"#)
                .run();
        let name = std::env::var("CARGO_PKG_NAME").expect("cargo sets CARGO_PKG_NAME");
        assert_eq!(inherited, Value::from(format!("{name}\n")));
    }

    #[test]
    fn run_env_overrides_an_inherited_variable() {
        // Cargo sets CARGO_PKG_NAME in the test process, so it is inherited.
        assert_values(&[(
            r#"os.run('sh', ['-c', 'echo "$CARGO_PKG_NAME"'], {env: {CARGO_PKG_NAME: 'other'}}).stdout"#,
            r"'other\n'",
        )]);
    }

    #[test]
    fn run_ignores_a_child_that_exits_without_reading_its_stdin() {
        // Far more than a pipe holds, so the write fails when the child is gone.
        assert_values(&[(
            "os.run('true', [], {stdin: tile('x', 1000000)})",
            "{stdout: '', stderr: '', exit_code: 0, signal: null}",
        )]);
    }

    #[test]
    fn run_requires_utf8_output_unless_binary() {
        assert_raises(&[(
            // The shell runs `printf '\377'`, writing the single byte 0xff.
            r#"os.run('sh', ['-c', "printf '\\377'"])"#,
            "Function std.os.run got stdout from `sh` that is not UTF-8; \
             pass `binary: true` to get Bytes",
        )]);
        assert_values(&[(
            r#"os.run('sh', ['-c', "printf '\\377'"], {binary: true})"#,
            "{stdout: x'ff', stderr: x'', exit_code: 0, signal: null}",
        )]);
        assert_raises(&[(
            r#"os.run('sh', ['-c', "printf '\\377' >&2"])"#,
            "Function std.os.run got stderr from `sh` that is not UTF-8; \
             pass `binary: true` to get Bytes",
        )]);
    }

    #[test]
    fn run_raises_when_the_program_cannot_start() {
        let raised = script("os.run('frost-surely-no-such-program', [])").raises();
        assert!(
            raised
                .starts_with("Function std.os.run could not run `frost-surely-no-such-program`: "),
            "{raised}"
        );
    }

    #[test]
    fn run_checks_its_options() {
        assert_raises(&[
            (
                "os.run('true', [], {env: {}, replace_env: {}})",
                "Function std.os.run takes option `env` or `replace_env`, not both",
            ),
            (
                "os.run('true', [], {stdin: 1})",
                "Function std.os.run requires String or Bytes for option `stdin`, got Int",
            ),
            (
                "os.run('true', [], {stdin: null})",
                "Function std.os.run requires String or Bytes for option `stdin`, got Null",
            ),
        ]);
    }

    #[test]
    fn run_rejects_options_of_the_wrong_name_or_type() {
        for (options, problem) in [
            (
                "{stdni: 'x'}",
                "unknown field `stdni`, expected one of `stdin`, `cwd`, `env`, \
                 `replace_env`, `binary` (at a key)",
            ),
            ("{[1]: 2}", "expected field identifier, got Int (at a key)"),
            ("{cwd: x'2f'}", "expected String, got Bytes (at `cwd`)"),
            // An option given as Null is given, not left out.
            ("{cwd: null}", "expected String, got Null (at `cwd`)"),
            ("{env: null}", "expected Map, got Null (at `env`)"),
            ("{binary: 'yes'}", "expected Bool, got String (at `binary`)"),
            ("{env: ['A=b']}", "expected Map, got Array (at `env`)"),
            (
                "{replace_env: {A: 1}}",
                "expected String, got Int (at `replace_env.A`)",
            ),
            (
                "{env: {[1]: 'a'}}",
                "expected String, got Int (at a key of `env`)",
            ),
        ] {
            assert_raises(&[(
                &format!("os.run('true', [], {options})"),
                &format!(
                    "Function std.os.run requires argument 3 (options) to be valid: {problem}"
                ),
            )]);
        }
    }
}

#[test]
fn run_checks_its_arguments() {
    assert_raises(&[
        (
            "os.run(1, [])",
            "Function std.os.run requires String as argument 1 (command), got Int",
        ),
        (
            "os.run('echo', 'hi')",
            "Function std.os.run requires Array as argument 2 (args), got String",
        ),
        (
            "os.run('echo', [1])",
            "Function std.os.run requires String as element 0 of argument 2 (args), got Int",
        ),
        (
            "os.run('echo', ['a', 'b', null])",
            "Function std.os.run requires String as element 2 of argument 2 (args), got Null",
        ),
        (
            "os.run('echo', [], 1)",
            "Function std.os.run requires Map as argument 3 (options), got Int",
        ),
    ]);
    assert_arity("run", "between 2 and 3", &[0, 1, 4]);
}
