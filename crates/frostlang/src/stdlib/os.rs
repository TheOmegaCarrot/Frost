//! `std.os`: the process's environment variables, ID, and standard streams,
//! pausing, and running other programs.

use std::collections::BTreeMap;
use std::io::{self, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Deserialize;

use crate::native::{Args, De, FrostArg, Optional};
use crate::stdlib::stream::{self, Kind};
use crate::{Arity, FrostError, FrostType, Param, Params, StdlibModule, Value};

/// The `std.os` module: reading environment variables, the process ID,
/// the process's standard input, output, and error streams, sleeping, and
/// running programs to completion.
///
/// It reaches outside the script: a script with it can read the environment,
/// use the process's standard streams, and run any program the host process
/// could.
pub fn os() -> StdlibModule {
    StdlibModule::new(
        "os",
        Value::map([
            ("getenv", getenv()),
            ("pid", pid()),
            ("sleep", sleep()),
            ("run", run()),
            (
                "stdin",
                stream::reader(BufReader::new(io::stdin()), Kind::Stream),
            ),
            ("stdout", stream::writer(io::stdout(), Kind::Stream)),
            ("stderr", stream::writer(io::stderr(), Kind::Stream)),
        ]),
    )
}

fn getenv() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::STRING).named("name")]);
    Value::checked_native("os.getenv", PARAMS, |_, args| {
        let name = args[0].as_str().expect("type-checked as a String");
        // No variable has such a name, and the platform may reject asking.
        if name.is_empty() || name.contains(['=', '\0']) {
            return Ok(Value::Null);
        }
        // An unset variable and one whose value is not UTF-8 are both Null.
        Ok(std::env::var(name).map_or(Value::Null, Value::from))
    })
}

fn pid() -> Value {
    Value::native("os.pid", Arity::Exact(0), |_, _| {
        Ok(Value::Int(i64::from(std::process::id())))
    })
}

fn sleep() -> Value {
    const PARAMS: Params = Params::new(&[<u64 as FrostArg>::PARAM.named("ms")]);
    Value::checked_native("os.sleep", PARAMS, |ctx, args| {
        let ms: u64 = Args::new(ctx.name(), PARAMS, args).take()?;
        std::thread::sleep(Duration::from_millis(ms));
        Ok(Value::Null)
    })
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
    /// the type of `stdin`.
    fn validate(self) -> Result<Self, FrostError> {
        let options = self;
        if options.env.is_some() && options.replace_env.is_some() {
            return Err(FrostError::from_static(
                "Function os.run takes option `env` or `replace_env`, not both",
            ));
        }
        if let Some(stdin) = &options.stdin
            && stdin.as_byte_slice().is_none()
        {
            return Err(FrostError::from_string(format!(
                "Function os.run requires String or Bytes for option `stdin`, got {}",
                stdin.type_name()
            )));
        }
        Ok(options)
    }
}

/// `output`, from the child's stream `name`, as a String, or as Bytes in
/// binary mode.
fn stream_value(
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
            "Function os.run got {name} from `{command}` that is not UTF-8; \
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

fn run() -> Value {
    const PARAMS: Params = Params::new(&[
        <String as FrostArg>::PARAM.named("command"),
        <Vec<String> as FrostArg>::PARAM.named("args"),
        // Taken as `Optional<De<RunOptions>>`, but only from a Map.
        Param::of(FrostType::MAP).named("options").optional(),
    ]);
    Value::checked_native("os.run", PARAMS, |ctx, args| {
        let mut args = Args::new(ctx.name(), PARAMS, args);
        let program: String = args.take()?;
        let arguments: Vec<String> = args.take()?;
        let Optional(options) = args.take::<Optional<De<RunOptions>>>()?;
        let options = options.map_or_else(RunOptions::default, |De(options)| options);
        let options = options.validate()?;
        let input = options.stdin.as_ref().and_then(Value::as_byte_slice);

        let mut command = Command::new(&program);
        command
            .args(arguments)
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
                        "Function os.run requires its `cwd` option to be a directory, \
                         but `{cwd}` is not"
                    )));
                }
                Err(err) => {
                    return Err(FrostError::from_string(format!(
                        "Function os.run cannot use `{cwd}` as its working directory: {err}"
                    )));
                }
            }
            command.current_dir(cwd);
        }
        if let Some(vars) = &options.replace_env {
            command.env_clear().envs(vars);
        }
        if let Some(vars) = &options.env {
            command.envs(vars);
        }

        let could_not_run = |err: std::io::Error| {
            FrostError::from_string(format!("Function os.run could not run `{program}`: {err}"))
        };
        let mut child = command.spawn().map_err(could_not_run)?;
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
                stream_value("stdout", output.stdout, options.binary, &program)?,
            ),
            (
                "stderr",
                stream_value("stderr", output.stderr, options.binary, &program)?,
            ),
            ("exit_code", exit_code),
            ("signal", signal),
        ]))
    })
}
