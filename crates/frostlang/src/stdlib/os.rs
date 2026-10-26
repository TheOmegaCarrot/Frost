//! `std.os`: the process's environment variables, ID, platform, and standard
//! streams, pausing, and running other programs.

use std::collections::BTreeMap;
use std::io::{self, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Deserialize;

use crate::native::{De, Optional};
use crate::stdlib::stream::{self, Kind};
use crate::{FrostError, FrostString, NativeCtx, StdlibModule, Value, frost_fn};

/// The `std.os` module: reading environment variables, the process ID,
/// the operating system and architecture it runs on,
/// the process's standard input, output, and error streams, sleeping, and
/// running programs to completion.
///
/// It reaches outside the script: a script with it can read the environment,
/// use the process's standard streams, and run any program the host process
/// could.
pub fn os() -> StdlibModule {
    StdlibModule::builder("os")
        .function(getenv_frost())
        .function(pid_frost())
        .function(sleep_frost())
        .function(run_frost())
        .value(
            "stdin",
            stream::reader(BufReader::new(io::stdin()), Kind::Stream),
        )
        .value("stdout", stream::writer(io::stdout(), Kind::Stream))
        .value("stderr", stream::writer(io::stderr(), Kind::Stream))
        .value("platform", platform())
        .value("arch", Value::from(std::env::consts::ARCH))
        .build()
}

/// The operating system's name,
/// or Null on a target for which Rust names none, such as `wasm32-unknown-unknown`.
fn platform() -> Value {
    match std::env::consts::OS {
        "" => Value::Null,
        os => Value::from(os),
    }
}

#[frost_fn]
fn getenv(name: FrostString) -> Value {
    // No variable has such a name, and the platform may reject asking.
    if name.is_empty() || name.contains(['=', '\0']) {
        return Value::Null;
    }
    // An unset variable and one whose value is not UTF-8 are both Null.
    std::env::var(name.as_str()).map_or(Value::Null, Value::from)
}

#[frost_fn]
fn pid() -> i64 {
    i64::from(std::process::id())
}

#[frost_fn]
fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

// --- run ---

/// The options `os.run` takes in its optional third argument.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RunOptions {
    /// String or Bytes, checked after deserializing: a `Value` arrives as is.
    stdin: Option<Value>,
    cwd: Option<String>,
    env: Option<BTreeMap<String, String>>,
    replace_env: Option<BTreeMap<String, String>>,
    binary: bool,
}

impl RunOptions {
    /// Checks what deserializing cannot: options that exclude each other, and
    /// the type of `stdin`. `function` names the function in errors.
    fn validate(self, function: &str) -> Result<Self, FrostError> {
        let options = self;
        if options.env.is_some() && options.replace_env.is_some() {
            return Err(FrostError::from_string(format!(
                "Function {function} takes option `env` or `replace_env`, not both"
            )));
        }
        if let Some(stdin) = &options.stdin
            && stdin.as_byte_slice().is_none()
        {
            return Err(FrostError::from_string(format!(
                "Function {function} requires String or Bytes for option `stdin`, got {}",
                stdin.type_name()
            )));
        }
        Ok(options)
    }
}

/// `output`, from the child's stream `name`, as a String, or as Bytes in
/// binary mode. `function` names the function in errors.
fn stream_value(
    function: &str,
    name: &str,
    output: Vec<u8>,
    binary: bool,
    command: &str,
) -> Result<Value, FrostError> {
    if binary {
        return Ok(Value::from(output));
    }
    String::from_utf8(output).map(Value::from).map_err(|_| {
        FrostError::from_string(format!(
            "Function {function} got {name} from `{command}` that is not UTF-8; \
             pass `binary: true` to get Bytes"
        ))
    })
}

/// How the child ended, as the result's `exit_code` and `signal`: one of them
/// is Null.
fn exit_values(status: std::process::ExitStatus) -> (Value, Value) {
    if let Some(code) = status.code() {
        return (Value::Int(i64::from(code)), Value::Null);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return (Value::Null, Value::Int(i64::from(signal)));
        }
    }
    (Value::Null, Value::Null)
}

#[frost_fn]
fn run(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    command: String,
    args: Vec<String>,
    options: Optional<De<RunOptions>>,
) -> Result<Value, FrostError> {
    let function = ctx.name();
    let Optional(options) = options;
    let options = options.map_or_else(RunOptions::default, |De(options)| options);
    let options = options.validate(function)?;
    let input = options.stdin.as_ref().and_then(Value::as_byte_slice);

    let mut process = Command::new(&command);
    process
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = &options.cwd {
        // Spawning reports a bad working directory as if the program were
        // missing, so it is checked first, to name what is wrong.
        match std::fs::metadata(cwd) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                return Err(FrostError::from_string(format!(
                    "Function {function} requires its `cwd` option to be a directory, \
                     but `{cwd}` is not"
                )));
            }
            Err(err) => {
                return Err(FrostError::from_string(format!(
                    "Function {function} cannot use `{cwd}` as its working directory: {err}"
                )));
            }
        }
        process.current_dir(cwd);
    }
    if let Some(vars) = &options.replace_env {
        process.env_clear().envs(vars);
    }
    if let Some(vars) = &options.env {
        process.envs(vars);
    }

    let could_not_run = |err: std::io::Error| {
        FrostError::from_string(format!(
            "Function {function} could not run `{command}`: {err}"
        ))
    };
    let mut child = process.spawn().map_err(could_not_run)?;
    let stdin = child.stdin.take();
    // Input is written from its own thread while this one collects the
    // output: a child can block writing output until its input is read.
    let output = std::thread::scope(|scope| {
        if let (Some(mut pipe), Some(input)) = (stdin, input) {
            scope.spawn(move || {
                // A child may exit without reading all its input; that is
                // its business, so a failed write is not an error.
                let _ = pipe.write_all(input);
            });
        }
        child.wait_with_output()
    })
    .map_err(could_not_run)?;

    let (exit_code, signal) = exit_values(output.status);
    Ok(Value::map([
        (
            "stdout",
            stream_value(function, "stdout", output.stdout, options.binary, &command)?,
        ),
        (
            "stderr",
            stream_value(function, "stderr", output.stderr, options.binary, &command)?,
        ),
        ("exit_code", exit_code),
        ("signal", signal),
    ]))
}
