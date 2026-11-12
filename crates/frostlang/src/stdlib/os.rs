//! `std.os`: the process's environment variables, ID, platform, and standard
//! streams, the clock, pausing, and running other programs.

use std::collections::BTreeMap;
use std::io::{self, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};

use itertools::Itertools;
use serde::Deserialize;

use crate::native::{De, Flat, Optional};
use crate::stdlib::stream::{self, Kind};
use crate::stdlib::time::unix_micros;
use crate::{
    FrostError, FrostMap, FrostString, MapKey, NativeCtx, Nullable, StdlibModule, Value, frost_fn,
};

/// The `std.os` module: reading environment variables, the process ID,
/// the operating system and architecture it runs on,
/// the process's standard input, output, and error streams, reading the clock,
/// sleeping, and running programs to completion.
///
/// It reaches outside the script: a script with it can read the environment
/// and the clock, use the process's standard streams, and run any program the
/// host process could.
pub fn os() -> StdlibModule {
    StdlibModule::builder("os")
        .function(getenv_frost())
        .function(env_frost())
        .function(pid_frost())
        .function(now_frost())
        .function(monotonic_frost())
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

/// Whether a variable named `name` can be read.
/// std reads none whose name is empty or holds `=` or a NUL,
/// and the platform may reject being asked.
fn is_readable_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['=', '\0'])
}

#[frost_fn]
fn getenv(name: FrostString) -> Value {
    if !is_readable_name(&name) {
        return Value::Null;
    }
    // An unset variable and one whose value is not UTF-8 are both Null.
    std::env::var(name.as_str()).map_or(Value::Null, Value::from)
}

#[frost_fn]
fn env() -> FrostMap {
    // This target has no environment,
    // but std's `vars_os` panics on it rather than listing none.
    if cfg!(all(target_family = "wasm", target_os = "unknown")) {
        return FrostMap::empty();
    }
    // Each entry is what `getenv` reads for its name:
    // a variable it reads as Null is left out,
    // and a name the environment holds twice keeps its first value, the one `getenv` finds.
    std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
        .filter(|(name, _)| is_readable_name(name))
        .unique_by(|(name, _)| name.clone())
        .map(|(name, value)| (MapKey::from(name), Value::from(value)))
        .collect()
}

#[frost_fn]
fn pid() -> i64 {
    i64::from(std::process::id())
}

/// Whether this target has no clock: std's clocks panic on it rather than
/// reporting none.
const NO_CLOCK: bool = cfg!(all(target_family = "wasm", target_os = "unknown"));

#[frost_fn]
fn now() -> Nullable<i64> {
    if NO_CLOCK {
        return Nullable(None);
    }
    Nullable(unix_micros(SystemTime::now()))
}

#[frost_fn]
fn monotonic() -> Nullable<i64> {
    if NO_CLOCK {
        return Nullable(None);
    }
    // The fixed point the readings count from: the first reading.
    static START: OnceLock<Instant> = OnceLock::new();
    let start = *START.get_or_init(Instant::now);
    Nullable(i64::try_from(start.elapsed().as_micros()).ok())
}

#[frost_fn]
fn sleep(us: u64) {
    std::thread::sleep(Duration::from_micros(us));
}

// --- run ---

/// The options `os.run` takes in its optional third argument.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RunOptions {
    stdin: Option<Flat>,
    cwd: Option<String>,
    env: Option<BTreeMap<String, String>>,
    replace_env: Option<BTreeMap<String, String>>,
    binary: bool,
}

impl RunOptions {
    /// Checks what deserializing cannot: options that exclude each other.
    /// `function` names the function in errors.
    fn validate(self, function: &str) -> Result<Self, FrostError> {
        let options = self;
        if options.env.is_some() && options.replace_env.is_some() {
            return Err(FrostError::from_string(format!(
                "Function {function} takes option `env` or `replace_env`, not both"
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
    let input = options.stdin.as_ref().map(Flat::as_bytes);

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
