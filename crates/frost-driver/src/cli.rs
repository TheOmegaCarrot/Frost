//! The command line a [`Driver`](crate::Driver) accepts.

use std::path::PathBuf;

use clap::{Args, Command as ClapCommand, CommandFactory, Parser, Subcommand, ValueEnum};
use frost_compile::{InvalidOptimizationSetting, Optimization, OptimizationOptions};

/// The command line for a driver called `name` at `version`, whose scripts
/// compile with `default` optimizations unless `-O` changes them.
pub(crate) fn command(name: &str, version: &str, default: OptimizationOptions) -> ClapCommand {
    // Usage is written out because clap's would not show that everything
    // after a script is its arguments.
    Cli::command()
        .name(name.to_string())
        .bin_name(name.to_string())
        .version(version.to_string())
        .override_usage(format!(
            "{name} [OPTIONS] [SCRIPT [ARGS]...]\n       \
             {name} [OPTIONS] -e CODE [ARGS]...\n       \
             {name} [OPTIONS] <COMMAND>"
        ))
        .mut_subcommand("run", |run| {
            run.override_usage(format!("{name} run [OPTIONS] SCRIPT [ARGS]..."))
        })
        .mut_arg("optimize", |arg| {
            let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
            arg.help(format!("{help} [default: {}]", describe(default)))
        })
}

/// `options` as `-O` settings: a preset if one matches, otherwise `none` and
/// the optimizations that are on.
fn describe(options: OptimizationOptions) -> String {
    if options == OptimizationOptions::ALL {
        return "all".to_string();
    }
    let on = Optimization::ALL
        .into_iter()
        .filter(|&optimization| options.get(optimization))
        .map(|optimization| format!(",{}=true", optimization.name()));
    std::iter::once("none".to_string()).chain(on).collect()
}

/// The heading over options most people never need, which `--help` shows but
/// `-h` does not.
const ADVANCED: &str = "Advanced options";

#[derive(Debug, Parser)]
#[command(
    after_help = "Options go before SCRIPT: everything after it is the script's. \
                        A SCRIPT of `-` is read from standard input."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// A script or compiled image to run (shorthand for `run`), then the
    /// arguments it gets as `args`
    #[arg(
        value_name = "SCRIPT",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    script_and_args: Vec<String>,

    /// Run CODE as a script, with any ARGS as its `args`
    #[arg(short = 'e', long = "eval", value_name = "CODE")]
    eval: Option<String>,

    #[command(flatten)]
    pub(crate) options: Options,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a script, or an image written by `compile`, with ARGS as its `args`
    ///
    /// An image runs as compiled, whatever optimizations the command line
    /// selects. Run only images you trust: a damaged one may crash.
    Run {
        /// The script or image, then the arguments it gets as `args`
        #[arg(
            value_name = "SCRIPT",
            required = true,
            trailing_var_arg = true,
            allow_hyphen_values = true
        )]
        script_and_args: Vec<String>,
    },
    /// Compile a script and report its diagnostics, without running it
    Check { script: String },
    /// Compile a script to an image, which `run` runs without compiling again
    ///
    /// Only the same version of Frost can run the image.
    Compile {
        script: String,
        /// Write the image here
        #[arg(short, long, value_name = "IMAGE")]
        output: PathBuf,
    },
    /// Start an interactive session (also what no arguments do at a terminal)
    Repl,
    /// List the bytecode a script compiles to, or an image holds
    List { script: String },
    /// Show the syntax tree a script parses to
    Ast { script: String },
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    /// When to color diagnostics, listings, and interactive sessions
    #[arg(long, value_name = "WHEN", default_value = "auto", global = true)]
    pub(crate) color: Color,

    /// Change the optimizations: comma-separated settings, applied in order,
    /// each `<optimization>=true|false`, `all`, or `none`. May be given more
    /// than once.
    #[arg(
        short = 'O',
        long = "optimize",
        id = "optimize",
        value_name = "SETTINGS",
        global = true,
        hide_short_help = true,
        help_heading = ADVANCED
    )]
    optimize: Vec<String>,

    /// Compile without `args`, leaving the name free; no ARGS may be given.
    /// An image runs as compiled, whatever this says.
    #[arg(long, global = true, hide_short_help = true, help_heading = ADVANCED)]
    pub(crate) no_args: bool,
}

/// A script named on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Script {
    /// The file at this path.
    File(PathBuf),
    /// Standard input, named by `-`.
    Stdin,
}

impl From<String> for Script {
    fn from(name: String) -> Self {
        if name == "-" {
            Self::Stdin
        } else {
            Self::File(PathBuf::from(name))
        }
    }
}

/// What the command line asks for.
pub(crate) enum Action {
    Run {
        script: Script,
        args: Vec<String>,
    },
    Eval {
        code: String,
        args: Vec<String>,
    },
    Check(Script),
    Compile {
        script: Script,
        output: PathBuf,
    },
    Repl,
    List(Script),
    Ast(Script),
    /// No arguments at all: a session at a terminal, or else the script on
    /// standard input.
    Nothing,
}

impl Cli {
    /// What the command line asks for, or why it asks for nothing sensible.
    pub(crate) fn action(self) -> Result<Action, String> {
        let Self {
            command,
            script_and_args,
            eval,
            ..
        } = self;
        match (command, eval) {
            (Some(_), Some(_)) => Err("`-e` cannot be given with a subcommand".to_string()),
            (Some(command), None) => Ok(match command {
                Command::Run { script_and_args } => {
                    run(script_and_args).expect("clap requires `run`'s script")
                }
                Command::Check { script } => Action::Check(script.into()),
                Command::Compile { script, output } => Action::Compile {
                    script: script.into(),
                    output,
                },
                Command::Repl => Action::Repl,
                Command::List { script } => Action::List(script.into()),
                Command::Ast { script } => Action::Ast(script.into()),
            }),
            (None, Some(code)) => Ok(Action::Eval {
                code,
                args: script_and_args,
            }),
            (None, None) => Ok(run(script_and_args).unwrap_or(Action::Nothing)),
        }
    }
}

/// Running the first of `script_and_args` with the rest as its arguments, if
/// there is a first.
fn run(script_and_args: Vec<String>) -> Option<Action> {
    let mut words = script_and_args.into_iter();
    let script = words.next()?.into();
    Some(Action::Run {
        script,
        args: words.collect(),
    })
}

impl Options {
    /// The optimizations these options select, starting from `default`.
    pub(crate) fn optimization(
        &self,
        default: OptimizationOptions,
    ) -> Result<OptimizationOptions, InvalidOptimizationSetting> {
        self.optimize
            .iter()
            .try_fold(default, |options, settings| options.with_settings(settings))
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Color {
    /// Color when writing to a terminal, unless `NO_COLOR` is set
    Auto,
    /// Always color
    Always,
    /// Never color
    Never,
}
