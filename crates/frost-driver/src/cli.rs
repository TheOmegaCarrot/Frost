//! The command line a [`Driver`](crate::Driver) accepts.

use std::path::PathBuf;

use clap::{Args, Command as ClapCommand, CommandFactory, Parser, Subcommand, ValueEnum};
use frost_compile::OptimizationOptions;

/// The command line for a driver called `name` at `version`, whose scripts
/// compile with `default` optimizations unless a preset is chosen.
pub(crate) fn command(name: &str, version: &str, default: OptimizationOptions) -> ClapCommand {
    Cli::command()
        .name(name.to_string())
        .bin_name(name.to_string())
        .version(version.to_string())
        .mut_arg("preset", |arg| {
            let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
            arg.help(format!("{help} [default: {}]", describe(default)))
        })
}

/// `options` as the command line would name them: a preset if one matches,
/// otherwise the optimizations that are on.
fn describe(options: OptimizationOptions) -> String {
    if options == OptimizationOptions::ALL {
        return "all".to_string();
    }
    if options == OptimizationOptions::NONE {
        return "none".to_string();
    }
    Optimization::value_variants()
        .iter()
        .filter(|optimization| optimization.is_on(options))
        .map(|optimization| optimization.name())
        .collect::<Vec<_>>()
        .join(",")
}

#[derive(Debug, Parser)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Run this script or compiled image (shorthand for `run FILE`)
    file: Option<PathBuf>,

    /// Run CODE as a script
    #[arg(
        short = 'e',
        long = "eval",
        value_name = "CODE",
        conflicts_with = "file"
    )]
    eval: Option<String>,

    #[command(flatten)]
    pub(crate) options: Options,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a script, or an image written by `compile`
    ///
    /// An image runs as compiled, whatever optimizations the command line
    /// selects. Run only images you trust: a damaged one may crash.
    Run { file: PathBuf },
    /// Compile a script and report its diagnostics, without running it
    Check { file: PathBuf },
    /// Compile a script to an image, which `run` runs without compiling again
    ///
    /// Only the same version of Frost can run the image.
    Compile {
        file: PathBuf,
        /// Write the image here
        #[arg(short, long, value_name = "IMAGE")]
        output: PathBuf,
    },
    /// Start an interactive session (also what no arguments do)
    Repl,
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    /// Start from these optimizations instead of the defaults, then apply any --enable and --disable
    #[arg(short = 'O', long = "optimize", value_name = "PRESET", global = true)]
    preset: Option<Preset>,

    /// Turn these optimizations on
    #[arg(
        long,
        value_name = "OPTIMIZATION",
        value_delimiter = ',',
        global = true
    )]
    enable: Vec<Optimization>,

    /// Turn these optimizations off
    #[arg(
        long,
        value_name = "OPTIMIZATION",
        value_delimiter = ',',
        global = true
    )]
    disable: Vec<Optimization>,

    /// When to color diagnostics
    #[arg(long, value_name = "WHEN", default_value = "auto", global = true)]
    pub(crate) color: Color,
}

/// What the command line asks for.
pub(crate) enum Action {
    Run(PathBuf),
    Check(PathBuf),
    Compile { file: PathBuf, output: PathBuf },
    Eval(String),
    Repl,
}

impl Cli {
    pub(crate) fn action(self) -> Action {
        match (self.command, self.file, self.eval) {
            (Some(Command::Run { file }), ..) | (None, Some(file), _) => Action::Run(file),
            (Some(Command::Check { file }), ..) => Action::Check(file),
            (Some(Command::Compile { file, output }), ..) => Action::Compile { file, output },
            (None, None, Some(code)) => Action::Eval(code),
            (Some(Command::Repl), ..) | (None, None, None) => Action::Repl,
        }
    }
}

impl Options {
    /// The optimizations these options select, starting from `default` when no
    /// preset is given. Errors if an optimization is both enabled and disabled.
    pub(crate) fn optimization(
        &self,
        default: OptimizationOptions,
    ) -> Result<OptimizationOptions, String> {
        if let Some(both) = self.enable.iter().find(|on| self.disable.contains(on)) {
            return Err(format!("`{}` is both enabled and disabled", both.name()));
        }
        let mut options = match self.preset {
            None => default,
            Some(Preset::None) => OptimizationOptions::NONE,
            Some(Preset::All) => OptimizationOptions::ALL,
        };
        for &optimization in &self.enable {
            *optimization.flag(&mut options) = true;
        }
        for &optimization in &self.disable {
            *optimization.flag(&mut options) = false;
        }
        Ok(options)
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Preset {
    /// Every optimization off
    None,
    /// Every optimization on
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Optimization {
    ConstantFold,
    ConstantPropagate,
    BranchEliminate,
    CaptureHoist,
    ConsumeLocals,
    DeduplicateConstants,
}

impl Optimization {
    /// The name the command line uses for it.
    fn name(self) -> String {
        self.to_possible_value()
            .expect("no optimization is skipped")
            .get_name()
            .to_string()
    }

    fn is_on(self, mut options: OptimizationOptions) -> bool {
        *self.flag(&mut options)
    }

    fn flag(self, options: &mut OptimizationOptions) -> &mut bool {
        match self {
            Optimization::ConstantFold => &mut options.constant_fold,
            Optimization::ConstantPropagate => &mut options.constant_propagate,
            Optimization::BranchEliminate => &mut options.branch_eliminate,
            Optimization::CaptureHoist => &mut options.capture_hoist,
            Optimization::ConsumeLocals => &mut options.consume_locals,
            Optimization::DeduplicateConstants => &mut options.deduplicate_constants,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Color {
    /// Color when writing to a terminal
    Auto,
    Always,
    Never,
}
