//! A complete Frost command-line interface, built around your configuration.
//!
//! A [`Driver`] holds what a Frost program runs with: the [`Importer`] that
//! decides what it may import, the [`VmRuntimeConfiguration`] that bounds it,
//! and the optimizations it compiles with by default. It also holds how
//! interactive sessions start, in [`ReplSettings`].
//! [`Driver::run`] then parses a command line and carries it out.
//!
//! ```no_run
//! use frost_driver::{Driver, Exit};
//!
//! fn main() -> Exit {
//!     Driver::new()
//!         .with_name("my-frost")
//!         .with_version(env!("CARGO_PKG_VERSION"))
//!         .run_from_env()
//! }
//! ```

mod cli;
mod image;
mod repl;

pub use repl::ReplSettings;

use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::{ExitCode, Termination};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use clap::FromArgMatches;
use frost_compile::{CompilerErrors, CompilerOptions, OptimizationOptions, compile_program};
use frost_repl::Repl;
use frost_runtime::{FrostError, Importer, RunError, TrustedProgram, Vm, VmRuntimeConfiguration};

use cli::{Action, Cli, Color};

/// A Frost command-line interface: configure it, then [`run`](Self::run) a
/// command line.
///
/// A script's printed output goes to the `stdout` given to [`run`](Self::run),
/// as do help and version text; errors and diagnostics go to its `stderr`.
/// An interactive session shows its results and failures on its
/// [`Frontend`](frost_repl::Frontend) instead (see [`ReplSettings`]).
#[derive(Debug, Clone)]
pub struct Driver {
    name: String,
    version: String,
    importer: Arc<Importer>,
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
            importer: Arc::default(),
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

    /// Set what scripts may import.
    pub fn with_importer(mut self, importer: Arc<Importer>) -> Self {
        self.importer = importer;
        self
    }

    /// Set the configuration every script runs under. Its
    /// [`print_sink`](VmRuntimeConfiguration::print_sink) is replaced by the
    /// `stdout` given to [`run`](Self::run).
    pub fn with_configuration(mut self, configuration: VmRuntimeConfiguration) -> Self {
        self.configuration = configuration;
        self
    }

    /// Set the optimizations scripts compile with when the command line does
    /// not choose a preset.
    pub fn with_optimization(mut self, optimization: OptimizationOptions) -> Self {
        self.optimization = optimization;
        self
    }

    /// Carry out the command line `args`, whose first item is the program name,
    /// as a process's arguments are.
    /// A script's printed output, and help and version text, go to `stdout`;
    /// errors and diagnostics go to `stderr`. Both are flushed before this returns.
    /// Neither is taken to be a terminal, so `--color auto` writes no color.
    pub fn run<I, T>(
        &self,
        args: I,
        stdout: impl Write + Send + 'static,
        stderr: impl Write + Send + 'static,
    ) -> Exit
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let terminals = Terminals {
            stdout: false,
            stderr: false,
        };
        self.run_on(args, stdout, stderr, terminals)
    }

    /// [`run`](Self::run) the process's own command line, with its standard
    /// output and error. `--color auto` colors whichever of them is a terminal.
    pub fn run_from_env(&self) -> Exit {
        let terminals = Terminals {
            stdout: io::stdout().is_terminal(),
            stderr: io::stderr().is_terminal(),
        };
        self.run_on(std::env::args_os(), io::stdout(), io::stderr(), terminals)
    }

    fn run_on<I, T>(
        &self,
        args: I,
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
        let exit = self.carry_out(args, &stdout, &mut stderr, terminals);
        let _ = lock(&stdout).flush();
        let _ = stderr.flush();
        exit
    }

    fn carry_out<I, T, W>(
        &self,
        args: I,
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

        let optimization = match cli.options.optimization(self.optimization) {
            Ok(optimization) => optimization,
            Err(message) => {
                write_out(stderr, &format!("error: {message}\n"));
                return Exit::UsageError;
            }
        };
        let printed = Arc::clone(stdout);
        let print_sink = move |text: &str| {
            let _ = writeln!(lock(&printed), "{text}");
        };
        let session = Session {
            importer: &self.importer,
            configuration: VmRuntimeConfiguration {
                print_sink: Arc::new(print_sink),
                ..self.configuration.clone()
            },
            options: CompilerOptions {
                optimization_options: optimization,
                implicit_export: false,
            },
            color: cli.options.color,
            terminals,
            stderr,
        };
        match cli.action() {
            Action::Run(path) => session.run_file(&path),
            Action::Check(path) => session.check_file(&path),
            Action::Compile { file, output } => session.compile_file(&file, &output),
            Action::Eval(code) => session.run_source("<eval>", &code),
            Action::Repl => session.run_repl(&self.repl),
            Action::List(path) => session.list_file(&path, &mut Shared(Arc::clone(stdout))),
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
    /// The command line was invalid; a file could not be read, written, or
    /// loaded as an image; or an interactive session's input or output failed.
    /// Exit status 2.
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

/// Which of a run's output streams are terminals, for `--color auto`.
#[derive(Debug, Clone, Copy)]
struct Terminals {
    stdout: bool,
    stderr: bool,
}

/// One command line being carried out.
struct Session<'a> {
    importer: &'a Arc<Importer>,
    configuration: VmRuntimeConfiguration,
    options: CompilerOptions,
    color: Color,
    terminals: Terminals,
    stderr: &'a mut dyn Write,
}

impl Session<'_> {
    fn run_file(mut self, path: &Path) -> Exit {
        let contents = match self.read(path) {
            Ok(contents) => contents,
            Err(exit) => return exit,
        };
        if image::is_image(&contents) {
            return match image::decode(&contents) {
                // The user chose to run this image; see `Command::Run`.
                Ok(function) => self.run_program(Arc::new(function).assert_trusted()),
                Err(reason) => {
                    self.usage_error(&format!("cannot run {}: {reason}", path.display()))
                }
            };
        }
        match self.source(path, contents) {
            Ok(source) => self.run_source(&path.to_string_lossy(), &source),
            Err(exit) => exit,
        }
    }

    fn check_file(mut self, path: &Path) -> Exit {
        let source = match self.read_source(path) {
            Ok(source) => source,
            Err(exit) => return exit,
        };
        match compile_program(&path.to_string_lossy(), &source, self.options) {
            Ok(_) => Exit::Success,
            Err(errors) => self.report_diagnostics(&errors),
        }
    }

    fn compile_file(mut self, path: &Path, output: &Path) -> Exit {
        let source = match self.read_source(path) {
            Ok(source) => source,
            Err(exit) => return exit,
        };
        let program = match compile_program(&path.to_string_lossy(), &source, self.options) {
            Ok(output) => output.code,
            Err(errors) => return self.report_diagnostics(&errors),
        };
        let closure = program
            .into_closure()
            .expect("a standalone script captures nothing");
        match fs::write(output, image::encode(closure.inner_fn())) {
            Ok(()) => Exit::Success,
            Err(error) => self.usage_error(&format!("cannot write {}: {error}", output.display())),
        }
    }

    fn list_file(mut self, path: &Path, output: &mut dyn Write) -> Exit {
        let contents = match self.read(path) {
            Ok(contents) => contents,
            Err(exit) => return exit,
        };
        let function = if image::is_image(&contents) {
            match image::decode(&contents) {
                Ok(function) => Arc::new(function),
                Err(reason) => {
                    return self.usage_error(&format!("cannot list {}: {reason}", path.display()));
                }
            }
        } else {
            let source = match self.source(path, contents) {
                Ok(source) => source,
                Err(exit) => return exit,
            };
            match compile_program(&path.to_string_lossy(), &source, self.options) {
                Ok(output) => output
                    .code
                    .into_closure()
                    .expect("a standalone script captures nothing")
                    .inner_fn_arc(),
                Err(errors) => return self.report_diagnostics(&errors),
            }
        };
        let color = match self.color {
            Color::Always => true,
            Color::Never => false,
            Color::Auto => self.terminals.stdout,
        };
        match write!(output, "{}", function.disassemble().with_color(color)) {
            Ok(()) => Exit::Success,
            Err(error) => self.usage_error(&format!("cannot write the listing: {error}")),
        }
    }

    fn run_repl(mut self, settings: &ReplSettings) -> Exit {
        let color = match self.color {
            Color::Always => true,
            Color::Never => false,
            // A terminal frontend paints both streams.
            Color::Auto => {
                let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
                self.terminals.stdout && self.terminals.stderr && !no_color
            }
        };
        let repl = Repl::new()
            .with_configuration(self.configuration.clone())
            .with_importer(Arc::clone(self.importer))
            .with_optimization(self.options.optimization_options);
        let (mut frontend, mut repl) = settings.start(repl, color);
        match repl.run(&mut *frontend) {
            Ok(()) => Exit::Success,
            Err(error) => self.usage_error(&format!("the session ended: {error}")),
        }
    }

    fn run_source(mut self, filename: &str, source: &str) -> Exit {
        match compile_program(filename, source, self.options) {
            Ok(output) => self.run_program(output.code),
            Err(errors) => self.report_diagnostics(&errors),
        }
    }

    fn run_program(mut self, program: TrustedProgram) -> Exit {
        let closure = program
            .into_closure()
            .expect("a standalone script captures nothing");
        let outcome = Vm::factory()
            .configuration(self.configuration.clone())
            .with_importer(Arc::clone(self.importer))
            .build(closure)
            .and_then(|vm| vm.run().map_err(RunError::into_error));
        match outcome {
            Ok(_) => Exit::Success,
            Err(error) => self.report_error(&error),
        }
    }

    /// The contents of the file at `path`, or the exit for failing to read it.
    fn read(&mut self, path: &Path) -> Result<Vec<u8>, Exit> {
        fs::read(path)
            .map_err(|error| self.usage_error(&format!("cannot read {}: {error}", path.display())))
    }

    /// The script at `path`, or the exit for failing to read it as one.
    fn read_source(&mut self, path: &Path) -> Result<String, Exit> {
        let contents = self.read(path)?;
        if image::is_image(&contents) {
            return Err(self.usage_error(&format!(
                "{} is a compiled image, not a script",
                path.display()
            )));
        }
        self.source(path, contents)
    }

    /// `contents`, read from `path`, as script source.
    fn source(&mut self, path: &Path, contents: Vec<u8>) -> Result<String, Exit> {
        String::from_utf8(contents).map_err(|_| {
            self.usage_error(&format!(
                "cannot read {}: it is not UTF-8 text",
                path.display()
            ))
        })
    }

    fn usage_error(&mut self, message: &str) -> Exit {
        write_out(self.stderr, &format!("error: {message}\n"));
        Exit::UsageError
    }

    fn report_diagnostics(&mut self, errors: &CompilerErrors) -> Exit {
        let rendered = match self.color {
            // `render` also defers to the environment, such as `NO_COLOR`.
            Color::Auto if self.terminals.stderr => errors.render(),
            Color::Auto | Color::Never => errors.render_plain(),
            Color::Always => errors.render_pretty(),
        };
        write_out(self.stderr, &rendered);
        Exit::ScriptFailed
    }

    fn report_error(&mut self, error: &FrostError) -> Exit {
        let backtrace: String = error
            .backtrace()
            .iter()
            .map(|frame| format!("  in {frame}\n"))
            .collect();
        write_out(self.stderr, &format!("{error}\n{backtrace}"));
        Exit::ScriptFailed
    }
}

/// Write `text` to `out`. A failure to write is ignored: there is nowhere
/// left to report it.
fn write_out(out: &mut dyn Write, text: &str) {
    let _ = out.write_all(text.as_bytes());
}
