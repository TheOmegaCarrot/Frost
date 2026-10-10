//! A complete Frost command-line interface, built around your configuration.
//!
//! A [`Driver`] holds what a Frost program runs with: the [`Importer`] that
//! decides what it may import, the [`VmRuntimeConfiguration`] that bounds it,
//! and the optimizations it compiles with by default. It also holds how
//! interactive sessions start, in [`ReplSettings`].
//! [`Driver::run`] then parses a command line and carries it out.
//!
//! A script run from the command line has the arguments after it as `args`,
//! an Array of Strings.
//!
//! ```no_run
//! use frostlang_driver::{Driver, Exit};
//!
//! fn main() -> Exit {
//!     Driver::new()
//!         .with_name("my-frost")
//!         .with_version(env!("CARGO_PKG_VERSION"))
//!         .run_from_env()
//! }
//! ```
//!
//! # Features
//!
//! - `graphical-diagnostics` (off by default): draw compile errors as source
//!   snippets with their labels, in color at a terminal, rather than narrating
//!   them as plain text. A command-line interface for people almost certainly
//!   wants it.

mod cli;
mod repl;

pub use repl::ReplSettings;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;
use std::process::{ExitCode, Termination};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use clap::FromArgMatches;
use frostlang::compile::{
    CompilerOptions, Diagnostic, Diagnostics, OptimizationOptions, compile_in_scope,
};
use frostlang::image::{self, ImageError};
use frostlang::{
    FrostError, Importer, RunError, TrustedProgram, Value, Vm, VmRuntimeConfiguration,
};
use frostlang_parse::parse_program;
use frostlang_repl::{Frontend, LineFrontend, Repl};

use cli::{Action, Cli, Color, Script};

/// The name a script's arguments are bound to.
const ARGS: &str = "args";

/// A Frost command-line interface: configure it, then [`run`](Self::run) a
/// command line.
///
/// A script's printed output goes to the `stdout` given to [`run`](Self::run),
/// as do help and version text; errors and diagnostics go to its `stderr`.
/// An interactive session shows its results and failures on its
/// [`Frontend`] instead (see [`ReplSettings`]).
#[derive(Debug, Clone)]
pub struct Driver {
    name: String,
    version: String,
    importer: ImporterFor,
    configuration: VmRuntimeConfiguration,
    optimization: OptimizationOptions,
    repl: ReplSettings,
}

impl Default for Driver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver {
    /// A driver named `frost`, with nothing importable, the default
    /// [`VmRuntimeConfiguration`], every optimization on, and the default
    /// [`ReplSettings`].
    pub fn new() -> Self {
        Self {
            name: "frost".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            importer: ImporterFor::fixed(Arc::default()),
            configuration: VmRuntimeConfiguration::default(),
            optimization: OptimizationOptions::ALL,
            repl: ReplSettings::new(),
        }
    }

    /// Set how interactive sessions start.
    pub fn with_repl(mut self, settings: ReplSettings) -> Self {
        self.repl = settings;
        self
    }

    /// Set the name shown in help and usage messages.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the version `--version` reports.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set what scripts may import, the same for every script.
    pub fn with_importer(mut self, importer: Arc<Importer>) -> Self {
        self.importer = ImporterFor::fixed(importer);
        self
    }

    /// Set what scripts may import, chosen for each command line: `importer` is
    /// given the path of the script file to run, as the command line names it,
    /// or `None` when there is none, as for a script from standard input or
    /// `-e`, or an interactive session.
    /// Replaces an importer set with [`with_importer`](Self::with_importer).
    pub fn with_importer_for(
        mut self,
        importer: impl Fn(Option<&Path>) -> Arc<Importer> + Send + Sync + 'static,
    ) -> Self {
        self.importer = ImporterFor(Arc::new(importer));
        self
    }

    /// Set the configuration every script runs under. Its
    /// [`print_sink`](VmRuntimeConfiguration::print_sink) is replaced by the
    /// `stdout` given to [`run`](Self::run).
    pub fn with_configuration(mut self, configuration: VmRuntimeConfiguration) -> Self {
        self.configuration = configuration;
        self
    }

    /// Set the optimizations scripts compile with before the command line's
    /// `-O` changes them.
    pub fn with_optimization(mut self, optimization: OptimizationOptions) -> Self {
        self.optimization = optimization;
        self
    }

    /// Carry out the command line `args`, whose first item is the program name,
    /// as a process's arguments are.
    ///
    /// A script named `-` is read from `stdin`, as is the script when there are
    /// no arguments: `stdin` is taken not to be a terminal, so no arguments
    /// never start an interactive session. A script's printed output, and help
    /// and version text, go to `stdout`; errors and diagnostics go to `stderr`.
    /// Both are flushed before this returns. Neither is taken to be a
    /// terminal, so `--color auto` writes no color.
    pub fn run<I, T>(
        &self,
        args: I,
        stdin: impl Read,
        stdout: impl Write + Send + 'static,
        stderr: impl Write + Send + 'static,
    ) -> Exit
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let terminals = Terminals {
            stdin: false,
            stdout: false,
            stderr: false,
        };
        self.run_on(args, stdin, stdout, stderr, terminals)
    }

    /// [`run`](Self::run) the process's own command line, with its standard
    /// input, output, and error. No arguments start an interactive session if
    /// standard input is a terminal. `--color auto` colors whichever of
    /// standard output and error is a terminal.
    pub fn run_from_env(&self) -> Exit {
        let terminals = Terminals {
            stdin: io::stdin().is_terminal(),
            stdout: io::stdout().is_terminal(),
            stderr: io::stderr().is_terminal(),
        };
        self.run_on(
            std::env::args_os(),
            io::stdin(),
            io::stdout(),
            io::stderr(),
            terminals,
        )
    }

    fn run_on<I, T>(
        &self,
        args: I,
        mut stdin: impl Read,
        stdout: impl Write + Send + 'static,
        mut stderr: impl Write + Send + 'static,
        terminals: Terminals,
    ) -> Exit
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        // Shared by the print sink and the driver's own output.
        let stdout = Arc::new(Mutex::new(stdout));
        let exit = self.carry_out(args, &mut stdin, &stdout, &mut stderr, terminals);
        let _ = lock(&stdout).flush();
        let _ = stderr.flush();
        exit
    }

    fn carry_out<I, T, W>(
        &self,
        args: I,
        stdin: &mut dyn Read,
        stdout: &Arc<Mutex<W>>,
        stderr: &mut dyn Write,
        terminals: Terminals,
    ) -> Exit
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
        W: Write + Send + 'static,
    {
        let cli = match cli::command(&self.name, &self.version, self.optimization)
            .try_get_matches_from(args)
            .and_then(|matches| Cli::from_arg_matches(&matches))
        {
            Ok(cli) => cli,
            Err(error) => {
                let text = error.render().to_string();
                return if error.use_stderr() {
                    write_out(stderr, &text);
                    Exit::UsageError
                } else {
                    write_out(&mut *lock(stdout), &text);
                    Exit::Success
                };
            }
        };

        let usage_error = |stderr: &mut dyn Write, message: &str| {
            write_out(stderr, &format!("error: {message}\n"));
            Exit::UsageError
        };
        let optimization = match cli.options.optimization(self.optimization) {
            Ok(optimization) => optimization,
            Err(invalid) => return usage_error(stderr, &invalid.to_string()),
        };
        let (color, no_args) = (cli.options.color, cli.options.no_args);
        let action = match cli.action() {
            Ok(action) => action,
            Err(message) => return usage_error(stderr, &message),
        };
        let printed = Arc::clone(stdout);
        let print_sink = move |text: &str| {
            let _ = writeln!(lock(&printed), "{text}");
        };
        let script_file = match &action {
            Action::Run {
                script: Script::File(path),
                ..
            } => Some(path.as_path()),
            _ => None,
        };
        let importer = (self.importer.0)(script_file);
        let session = Session {
            importer: &importer,
            configuration: self
                .configuration
                .clone()
                .with_print_sink(Arc::new(print_sink)),
            options: CompilerOptions::new().with_optimization(optimization),
            no_args,
            color,
            terminals,
            stdin,
            stderr,
        };
        let mut stdout = Shared(Arc::clone(stdout));
        match action {
            Action::Run { script, args } => session.run_file(&script, args),
            Action::Eval { code, args } => session.run_source("<eval>", &code, args),
            Action::Check(script) => session.check_file(&script),
            Action::Compile { script, output } => session.compile_file(&script, &output),
            Action::Repl { basic } => session.run_repl(&self.repl, basic),
            Action::List(script) => session.list_file(&script, &mut stdout),
            Action::Ast(script) => session.show_ast(&script, &mut stdout),
            Action::Nothing if terminals.stdin => session.run_repl(&self.repl, false),
            Action::Nothing => session.run_file(&Script::Stdin, Vec::new()),
        }
    }
}

/// Lock `mutex`, even if a panic while it was held poisoned it: a writer left
/// mid-write is still fit to write to.
fn lock<W>(mutex: &Mutex<W>) -> MutexGuard<'_, W> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A writer shared with the print sink, locked for each write so the two
/// interleave in order.
struct Shared<W>(Arc<Mutex<W>>);

impl<W: Write> Write for Shared<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        lock(&self.0).write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        lock(&self.0).flush()
    }
}

/// How a [`Driver`] run ended, and so the process's exit status.
/// Return it from `main` to exit with that status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// Everything asked for was done. Exit status 0.
    Success,
    /// The script did not compile, or raised an error. Exit status 1.
    ScriptFailed,
    /// The command line was invalid; a file or standard input could not be
    /// read, a file could not be written or loaded as an image; or an
    /// interactive session's frontend failed. Exit status 2.
    UsageError,
}

impl Exit {
    /// The process exit status this stands for.
    pub fn code(self) -> u8 {
        match self {
            Exit::Success => 0,
            Exit::ScriptFailed => 1,
            Exit::UsageError => 2,
        }
    }
}

impl Termination for Exit {
    fn report(self) -> ExitCode {
        ExitCode::from(self.code())
    }
}

/// Which of a run's standard streams are terminals.
/// Chooses the importer for a command line from the path of its script file.
#[derive(Clone)]
struct ImporterFor(Arc<ChooseImporter>);

type ChooseImporter = dyn Fn(Option<&Path>) -> Arc<Importer> + Send + Sync;

impl ImporterFor {
    /// `importer`, whatever the script.
    fn fixed(importer: Arc<Importer>) -> Self {
        Self(Arc::new(move |_| Arc::clone(&importer)))
    }
}

impl std::fmt::Debug for ImporterFor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ImporterFor(..)")
    }
}

#[derive(Debug, Clone, Copy)]
struct Terminals {
    stdin: bool,
    stdout: bool,
    stderr: bool,
}

/// One command line being carried out.
struct Session<'a> {
    importer: &'a Arc<Importer>,
    configuration: VmRuntimeConfiguration,
    options: CompilerOptions,
    no_args: bool,
    color: Color,
    terminals: Terminals,
    stdin: &'a mut dyn Read,
    stderr: &'a mut dyn Write,
}

impl Session<'_> {
    fn run_file(mut self, script: &Script, args: Vec<String>) -> Exit {
        let contents = match self.read(script) {
            Ok(contents) => contents,
            Err(exit) => return exit,
        };
        if image::is_image(&contents) {
            return match image::decode(&contents) {
                // The user chose to run this image; see `Command::Run`.
                Ok(function) => self.run_program(Arc::new(function).assert_trusted(), args),
                Err(error) => {
                    let message = format!("cannot run {}: {}", name(script), unloadable(&error));
                    self.usage_error(&message)
                }
            };
        }
        match self.source(script, contents) {
            Ok(source) => self.run_source(&name(script), &source, args),
            Err(exit) => exit,
        }
    }

    fn check_file(mut self, script: &Script) -> Exit {
        let source = match self.read_source(script) {
            Ok(source) => source,
            Err(exit) => return exit,
        };
        match self.compile(&name(script), &source) {
            Ok(_) => Exit::Success,
            Err(exit) => exit,
        }
    }

    fn compile_file(mut self, script: &Script, output: &std::path::Path) -> Exit {
        let program = match self
            .read_source(script)
            .and_then(|source| self.compile(&name(script), &source))
        {
            Ok(program) => program,
            Err(exit) => return exit,
        };
        match fs::write(output, image::encode(&program)) {
            Ok(()) => Exit::Success,
            Err(error) => self.usage_error(&format!("cannot write {}: {error}", output.display())),
        }
    }

    fn list_file(mut self, script: &Script, output: &mut dyn Write) -> Exit {
        let contents = match self.read(script) {
            Ok(contents) => contents,
            Err(exit) => return exit,
        };
        let function = if image::is_image(&contents) {
            match image::decode(&contents) {
                Ok(function) => Arc::new(function),
                Err(error) => {
                    let message = format!("cannot list {}: {}", name(script), unloadable(&error));
                    return self.usage_error(&message);
                }
            }
        } else {
            let program = match self
                .source(script, contents)
                .and_then(|source| self.compile(&name(script), &source))
            {
                Ok(program) => program,
                Err(exit) => return exit,
            };
            close(program, Vec::new()).inner_fn_arc()
        };
        let color = self.colors(self.terminals.stdout);
        match write!(output, "{}", function.disassemble().with_color(color)) {
            Ok(()) => Exit::Success,
            Err(error) => self.usage_error(&format!("cannot write the listing: {error}")),
        }
    }

    fn show_ast(mut self, script: &Script, output: &mut dyn Write) -> Exit {
        let source = match self.read_source(script) {
            Ok(source) => source,
            Err(exit) => return exit,
        };
        let filename = name(script);
        let program = match parse_program(&filename, &source) {
            Ok(program) => program,
            Err(error) => {
                let diagnostic = Diagnostic::from_parse_error(&error, &filename, &source);
                return self.report_diagnostics(&Diagnostics::from(diagnostic));
            }
        };
        match writeln!(output, "{}", program.dump()) {
            Ok(()) => Exit::Success,
            Err(error) => self.usage_error(&format!("cannot write the tree: {error}")),
        }
    }

    /// Run a session on the frontend `settings` give, or, if they give none,
    /// on a [`LineFrontend`] if `basic` is true, else on the default.
    fn run_repl(mut self, settings: &ReplSettings, basic: bool) -> Exit {
        // A terminal frontend paints both streams.
        let color = self.colors(self.terminals.stdout && self.terminals.stderr);
        let repl = Repl::new()
            .with_configuration(self.configuration.clone())
            .with_importer(Arc::clone(self.importer))
            .with_optimization(self.options.optimization_options);
        let default_frontend = || -> Box<dyn Frontend> {
            if basic {
                Box::new(LineFrontend::stdin())
            } else {
                frostlang_repl::default_frontend(color)
            }
        };
        let (mut frontend, mut repl) = settings.start(repl, default_frontend);
        match repl.run(&mut *frontend) {
            Ok(()) => Exit::Success,
            Err(error) => self.usage_error(&format!("the session ended: {error}")),
        }
    }

    fn run_source(mut self, filename: &str, source: &str, args: Vec<String>) -> Exit {
        match self.compile(filename, source) {
            Ok(program) => self.run_program(program, args),
            Err(exit) => exit,
        }
    }

    fn run_program(mut self, program: TrustedProgram, args: Vec<String>) -> Exit {
        if self.no_args && !args.is_empty() {
            return self.usage_error("`--no-args` takes no arguments for the script");
        }
        let outcome = Vm::factory()
            .configuration(self.configuration.clone())
            .with_importer(Arc::clone(self.importer))
            .build(close(program, args))
            .run()
            .map_err(RunError::into_error);
        match outcome {
            Ok(_) => Exit::Success,
            Err(error) => self.report_error(&error),
        }
    }

    /// The program `source` compiles to, with `args` in scope unless
    /// `--no-args` says otherwise, or the exit for its diagnostics.
    fn compile(&mut self, filename: &str, source: &str) -> Result<TrustedProgram, Exit> {
        let scope: &[&str] = if self.no_args { &[] } else { &[ARGS] };
        compile_in_scope(filename, source, self.options, scope)
            .map(|output| output.code)
            .map_err(|errors| self.report_diagnostics(&errors))
    }

    /// Whether to color output to a stream, which is a terminal if `terminal`
    /// is true.
    fn colors(&self, terminal: bool) -> bool {
        match self.color {
            Color::Always => true,
            Color::Never => false,
            Color::Auto => {
                terminal && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
            }
        }
    }

    /// The contents of `script`, or the exit for failing to read it.
    fn read(&mut self, script: &Script) -> Result<Vec<u8>, Exit> {
        let contents = match script {
            Script::File(path) => fs::read(path),
            Script::Stdin => {
                let mut contents = Vec::new();
                self.stdin.read_to_end(&mut contents).map(|_| contents)
            }
        };
        contents
            .map_err(|error| self.usage_error(&format!("cannot read {}: {error}", name(script))))
    }

    /// The source of `script`, or the exit for failing to read it as source.
    fn read_source(&mut self, script: &Script) -> Result<String, Exit> {
        let contents = self.read(script)?;
        if image::is_image(&contents) {
            let message = format!("{} is a compiled image, not a script", name(script));
            return Err(self.usage_error(&message));
        }
        self.source(script, contents)
    }

    /// `contents`, read from `script`, as source.
    fn source(&mut self, script: &Script, contents: Vec<u8>) -> Result<String, Exit> {
        String::from_utf8(contents).map_err(|_| {
            self.usage_error(&format!(
                "cannot read {}: it is not UTF-8 text",
                name(script)
            ))
        })
    }

    fn usage_error(&mut self, message: &str) -> Exit {
        write_out(self.stderr, &format!("error: {message}\n"));
        Exit::UsageError
    }

    fn report_diagnostics(&mut self, errors: &Diagnostics) -> Exit {
        #[cfg(feature = "graphical-diagnostics")]
        let rendered = if self.colors(self.terminals.stderr) {
            errors.render_pretty()
        } else {
            errors.render_plain()
        };
        #[cfg(not(feature = "graphical-diagnostics"))]
        let rendered = errors.render_narrated();
        write_out(self.stderr, &rendered);
        Exit::ScriptFailed
    }

    fn report_error(&mut self, error: &FrostError) -> Exit {
        write_out(self.stderr, &format!("Error: {}\n", error.with_backtrace()));
        Exit::ScriptFailed
    }
}

/// Why an image cannot load, as the end of a sentence about it.
fn unloadable(error: &ImageError) -> String {
    match error {
        ImageError::VersionMismatch { found } => {
            format!("it was compiled by Frost {found}; compile it again with this Frost")
        }
        ImageError::Damaged => "it is damaged".to_string(),
        other => other.to_string(),
    }
}

/// How messages and diagnostics name `script`.
fn name(script: &Script) -> String {
    match script {
        Script::File(path) => path.to_string_lossy().into_owned(),
        Script::Stdin => "<stdin>".to_string(),
    }
}

/// `program`, ready to run with `args` as its arguments.
fn close(program: TrustedProgram, args: Vec<String>) -> Arc<frostlang::Closure> {
    let args: Value = args.into_iter().map(Value::from).collect();
    program
        .close(BTreeMap::from([(ARGS.to_string(), args)]))
        .expect("a script captures nothing but its arguments")
}

/// Write `text` to `out`. A failure to write is ignored: there is nowhere
/// left to report it.
fn write_out(out: &mut dyn Write, text: &str) {
    let _ = out.write_all(text.as_bytes());
}
