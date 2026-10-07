//! `std.os`, from Frost source: environment variables, the process ID,
//! sleeping, and running programs.
//!
//! Each case runs with only `std.os` installed, bound as `os`. The cases that
//! run programs use POSIX tools, so they run only on Unix.

mod script;

use std::time::{Duration, Instant};

use frostlang::stdlib::StdlibConfig;
use frostlang::{ImporterBuilder, Stdlib, Value, stdlib};
use script::assertions::{Library, library_assertions};
use script::{Script, UNOPTIMIZED};

library_assertions!(Library::module(stdlib::os, "os"));

// --- The module ---

#[test]
fn the_module_holds_its_functions_and_streams() {
    assert_values(&[(
        "sorted(keys(os))",
        "['getenv', 'pid', 'run', 'sleep', 'stderr', 'stdin', 'stdout']",
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
    // No variable can have these names.
    for name in ["''", "'A=B'", r"'A\u{0}B'"] {
        let expression = format!("os.getenv({name})");
        assert_eq!(script(&expression).run(), Value::Null, "{expression}");
    }
}

#[test]
fn getenv_checks_its_argument() {
    assert_raises(&[(
        "os.getenv(1)",
        "Function os.getenv requires String as argument 1 (name), got Int",
    )]);
    assert_arity("getenv", "1", &[0, 2]);
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
            "Function os.sleep requires argument 1 (ms) to be at least 0, got -1",
        ),
        (
            "os.sleep(1.5)",
            "Function os.sleep requires Int as argument 1 (ms), got Float",
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
                "Function os.run cannot use `/frost-surely-no-such-dir` as its working directory: "
            ),
            "{raised}"
        );
        assert_raises(&[(
            "os.run('true', [], {cwd: '/dev/null'})",
            "Function os.run requires its `cwd` option to be a directory, but `/dev/null` is not",
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
            "Function os.run got stdout from `sh` that is not UTF-8; \
             pass `binary: true` to get Bytes",
        )]);
        assert_values(&[(
            r#"os.run('sh', ['-c', "printf '\\377'"], {binary: true})"#,
            "{stdout: x'ff', stderr: x'', exit_code: 0, signal: null}",
        )]);
        assert_raises(&[(
            r#"os.run('sh', ['-c', "printf '\\377' >&2"])"#,
            "Function os.run got stderr from `sh` that is not UTF-8; \
             pass `binary: true` to get Bytes",
        )]);
    }

    #[test]
    fn run_raises_when_the_program_cannot_start() {
        let raised = script("os.run('frost-surely-no-such-program', [])").raises();
        assert!(
            raised.starts_with("Function os.run could not run `frost-surely-no-such-program`: "),
            "{raised}"
        );
    }

    #[test]
    fn run_checks_its_options() {
        assert_raises(&[
            (
                "os.run('true', [], {env: {}, replace_env: {}})",
                "Function os.run takes option `env` or `replace_env`, not both",
            ),
            (
                "os.run('true', [], {stdin: 1})",
                "Function os.run requires String or Bytes for option `stdin`, got Int",
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
                &format!("Function os.run requires valid options: {problem}"),
            )]);
        }
    }
}

#[test]
fn run_checks_its_arguments() {
    assert_raises(&[
        (
            "os.run(1, [])",
            "Function os.run requires String as argument 1 (command), got Int",
        ),
        (
            "os.run('echo', 'hi')",
            "Function os.run requires Array as argument 2 (args), got String",
        ),
        (
            "os.run('echo', [1])",
            "Function os.run requires an Array of Strings as argument 2 (args), \
             but element 0 is Int",
        ),
        (
            "os.run('echo', [], 1)",
            "Function os.run requires Map as argument 3 (options), got Int",
        ),
    ]);
    assert_arity("run", "between 2 and 3", &[0, 1, 4]);
}
