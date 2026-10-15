//! An interactive Frost session: a read-eval-print loop.
//!
//! A [`Repl`] runs each piece of input as its own small program. Every
//! top-level `def` an input makes stays bound for the inputs after it, and a
//! later input may bind the same name again.
//!
//! A session reads its inputs from, and shows their outcomes on, a
//! [`Frontend`].
//!
//! ```no_run
//! use frostlang_repl::{Repl, SessionError};
//!
//! fn main() -> Result<(), SessionError> {
//!     let mut frontend = frostlang_repl::default_frontend(true);
//!     Repl::new().run(&mut *frontend)
//! }
//! ```
//!
//! # Features
//!
//! - `line-editor` (default): [`TerminalFrontend`], with line editing,
//!   history, and syntax highlighting.
//! - `graphical-diagnostics`: draw compile errors as source snippets with their
//!   labels, in color at a color terminal, rather than narrating them as plain
//!   text. If you show the REPL to people, you probably want it.

mod builtins;
mod cancel;
mod config;
mod config_runner;
#[cfg(feature = "line-editor")]
mod edit_dir;
mod frontend;
#[cfg(feature = "line-editor")]
mod highlight;
mod metacommand;
mod scripted;
mod segment;
#[cfg(feature = "line-editor")]
mod terminal;

pub use cancel::CancelHandle;
pub use config::{ConfigError, SessionProblem};
pub use frontend::{Frontend, LineFrontend, render_value};
pub use metacommand::{
    InvalidMetacommand, Invocation, MetacommandError, MetacommandHandler, MetacommandProblem,
    MetacommandSpec, MetacommandTable,
};
pub use scripted::{ScriptedFrontend, Transcript};
pub use segment::complete_segment;
#[cfg(feature = "line-editor")]
pub use terminal::TerminalFrontend;

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::Arc;

use frostlang::compile::{CompilerOptions, Diagnostics, OptimizationOptions, compile_in_scope};
use frostlang::{
    AbortReason, CancelToken, Closure, FrostError, IdleVm, Importer, ProgramParts, RunErrorKind,
    Value, Vm, VmRuntimeConfiguration,
};
use frostlang_parse::{Token, tokens};

use builtins::Reply;
use config::Reading;
use metacommand::{Handler, Registry};

/// The name diagnostics give an input.
const INPUT_NAME: &str = "<repl>";

/// The frontend a command-line REPL most likely wants. With the `line-editor`
/// feature, and a terminal on standard input and output, that is a
/// [`TerminalFrontend::new`], colored if `color` is true. Otherwise it is
/// [`LineFrontend::stdin`], and `color` is unused.
pub fn default_frontend(color: bool) -> Box<dyn Frontend> {
    #[cfg(feature = "line-editor")]
    {
        use std::io::IsTerminal;

        if io::stdin().is_terminal() && io::stdout().is_terminal() {
            return Box::new(TerminalFrontend::new().with_color(color));
        }
    }
    #[cfg(not(feature = "line-editor"))]
    let _ = color;
    Box::new(LineFrontend::stdin())
}

/// The name inputs refer to recent results by.
const RESULTS: &str = "results";

/// A read-eval-print loop: [`evaluate`](Self::evaluate) inputs one at a time,
/// or [`run`](Self::run) a whole session.
///
/// # Recent results
///
/// Inputs may refer to `results`: an Array of the most recent inputs' values,
/// oldest first, so that `results[-1]` is the last. It holds the values of
/// inputs that succeeded, other than Null, up to five by default (see
/// [`with_results_kept`](Self::with_results_kept)).
///
/// `results` is an ordinary name: an input or a seeded binding may bind it,
/// and inputs then see that binding instead. The REPL keeps recording results
/// meanwhile, so once that binding is removed, `results` holds the most recent
/// ones.
///
/// # Metacommands
///
/// In a session [`run`](Self::run), a segment starting with `:` and then a
/// name, as `:help` does, is a metacommand: an instruction to the REPL or its
/// frontend, not Frost source. Whatever follows the name is its
/// [argument](Invocation::argument).
///
/// The REPL's own are `:help`, `:quit`, `:bindings`, `:undef`,
/// `:disassemble`, `:ast`, `:optimize`, and `:config`. `:help` describes them, with any its frontend
/// adds (see [`Frontend::metacommands`]). A failed metacommand is shown as a
/// failed input is, and the session carries on.
///
/// # Config file
///
/// A session [`run`](Self::run) may start by reading a config file (see
/// [`with_config_file`](Self::with_config_file) and
/// [`with_default_config_file`](Self::with_default_config_file)), and reads it
/// again at `:config reload`. It is a Frost script, run apart from the
/// session, so its definitions are not bound in it. It runs under small
/// limits, and may import only `std.string`, `std.math`, and `repl`. `repl`'s
/// `colors` and `style` hold functions that wrap a String in ANSI codes, or,
/// if the frontend takes no [ANSI styling](Frontend::ansi_styling), return it
/// unchanged. `repl.colors` holds the terminal's 16: `black`, `red`, `green`,
/// `yellow`, `blue`, `magenta`, `cyan`, and `white`, and each of them with
/// `bright_` before it. `repl.style` holds `bold`, `dim`, `italic`, and
/// `underline`.
///
/// Its exports are sections, each a Map of the settings for one part of the
/// session. The REPL takes `session`, and the frontend takes the one its
/// [`config_key`](Frontend::config_key) names (see [`Frontend::configure`]).
/// Any other export is left alone, as another frontend's.
///
/// `session` holds:
///
/// - `results_kept`: how many [recent results](#recent-results) to keep, in
///   place of [`with_results_kept`](Self::with_results_kept)'s count.
/// - `preload`: a Map from names to module paths. As the session starts, each
///   name is bound to its module, as though the first input were
///   `def name = import(path)`.
///
/// ```frost
/// export def session = {results_kept: 10, preload: {string: 'std.string'}}
/// ```
///
/// With no file there, nothing is set. A problem with the file is shown on
/// the frontend as a [`ReplError::Config`]. A file that cannot be read, does
/// not compile, or fails sets nothing, as does a `session` holding an unknown
/// setting or one of the wrong type. A setting left out, or Null, keeps its
/// default.
///
/// `:config reload` applies `results_kept` and the frontend's section again,
/// but `preload` only applies as a session starts. If the file sets nothing,
/// for one of those problems, the settings stay as they were.
pub struct Repl {
    configuration: VmRuntimeConfiguration,
    importer: Arc<Importer>,
    optimization: OptimizationOptions,
    bindings: BTreeMap<String, Value>,
    // `None` for no config file.
    config_file: Option<PathBuf>,
    results_kept: usize,
    // In place of `results_kept`, if the config file sets it.
    config_results_kept: Option<usize>,
    // Oldest first.
    results: VecDeque<Value>,
    // The Vm the last input ran on, kept warm for the next.
    idle_vm: Option<IdleVm>,
    cancel: CancelHandle,
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

/// The [`max_call_depth`](VmRuntimeConfiguration::max_call_depth) a [`Repl`]
/// runs inputs under unless given a configuration: deep enough for any sound
/// recursion, and shallow enough that a runaway one fails before it exhausts
/// memory.
pub const DEFAULT_MAX_CALL_DEPTH: NonZeroU64 = NonZeroU64::new(1_000_000).unwrap();

impl Repl {
    /// A REPL with nothing bound, nothing importable, the default
    /// [`VmRuntimeConfiguration`] but with a call depth of
    /// [`DEFAULT_MAX_CALL_DEPTH`], every optimization on, five recent results
    /// kept, and no [config file](Self#config-file).
    pub fn new() -> Self {
        Self {
            configuration: VmRuntimeConfiguration::default()
                .with_max_call_depth(Some(DEFAULT_MAX_CALL_DEPTH)),
            importer: Arc::default(),
            optimization: OptimizationOptions::ALL,
            bindings: BTreeMap::new(),
            config_file: None,
            results_kept: 5,
            config_results_kept: None,
            results: VecDeque::new(),
            idle_vm: None,
            cancel: CancelHandle::default(),
        }
    }

    /// Set the configuration every input runs under, its
    /// [`max_call_depth`](VmRuntimeConfiguration::max_call_depth) included.
    pub fn with_configuration(mut self, configuration: VmRuntimeConfiguration) -> Self {
        self.configuration = configuration;
        self
    }

    /// Set what inputs may import.
    pub fn with_importer(mut self, importer: Arc<Importer>) -> Self {
        self.importer = importer;
        self
    }

    /// Read the [config file](Self#config-file) at `path` as each session
    /// starts.
    pub fn with_config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_file = Some(path.into());
        self
    }

    /// Read the [config file](Self#config-file) `repl.frst`, in a `frost`
    /// directory in the platform's usual place for configuration, as each
    /// session starts:
    ///
    /// | Platform | Config file |
    /// | -------- | ----------- |
    /// | Linux | `$XDG_CONFIG_HOME/frost/repl.frst`, or `~/.config/frost/repl.frst` |
    /// | macOS | `~/Library/Application Support/frost/repl.frst` |
    /// | Windows | `%APPDATA%\frost\repl.frst` |
    ///
    /// If the platform has no such place, no file is read.
    pub fn with_default_config_file(mut self) -> Self {
        self.config_file = config::default_path();
        self
    }

    /// Set the optimizations inputs compile with.
    pub fn with_optimization(mut self, optimization: OptimizationOptions) -> Self {
        self.optimization = optimization;
        self
    }

    /// Set how many [recent results](Self#recent-results) `results` holds. With
    /// 0, the REPL binds no `results` at all.
    pub fn with_results_kept(mut self, count: usize) -> Self {
        self.results_kept = count;
        self.forget_excess_results();
        self
    }

    /// How many recent results to keep.
    fn results_limit(&self) -> usize {
        self.config_results_kept.unwrap_or(self.results_kept)
    }

    /// Forget the oldest results, past those to keep.
    fn forget_excess_results(&mut self) {
        let excess = self.results.len().saturating_sub(self.results_limit());
        self.results.drain(..excess);
    }

    /// Bind `name` to `value`, as though an earlier input had defined it.
    /// Fails if `name` is not one Frost source can refer to (see
    /// [`check_name`]).
    pub fn with_binding(
        mut self,
        name: impl Into<String>,
        value: Value,
    ) -> Result<Self, InvalidName> {
        let name = name.into();
        check_name(&name)?;
        self.bindings.insert(name, value);
        Ok(self)
    }

    /// [`with_binding`](Self::with_binding) for each of `bindings` in turn: a
    /// name given twice takes its last value.
    pub fn with_bindings<N: Into<String>>(
        self,
        bindings: impl IntoIterator<Item = (N, Value)>,
    ) -> Result<Self, InvalidName> {
        bindings
            .into_iter()
            .try_fold(self, |repl, (name, value)| repl.with_binding(name, value))
    }

    /// Run `source` as the next input, returning its value: the value of its
    /// last expression, or Null.
    ///
    /// The input sees every binding earlier inputs made. The bindings it makes
    /// itself are kept only if it compiles and runs without error; a failed
    /// input changes nothing.
    ///
    /// A [`CancelHandle`] from [`cancel_handle`](Self::cancel_handle) can stop
    /// the input meanwhile.
    pub fn evaluate(&mut self, source: &str) -> Result<Value, ReplError> {
        let token = self.cancel.begin();
        let outcome = self.evaluate_until_cancelled(source, token);
        self.cancel.end();
        outcome
    }

    /// The means to cancel whichever input this REPL is evaluating, from
    /// another thread or a [`Frontend`].
    pub fn cancel_handle(&self) -> CancelHandle {
        self.cancel.clone()
    }

    /// [`evaluate`](Self::evaluate), stopping when `token` is cancelled.
    fn evaluate_until_cancelled(
        &mut self,
        source: &str,
        token: CancelToken,
    ) -> Result<Value, ReplError> {
        let closure = self.compile(source)?;
        let (tail, exports) = self.run_closure(closure, token)?;
        self.bindings.extend(exports);
        if self.results_limit() > 0 && tail != Value::Null {
            if self.results.len() == self.results_limit() {
                self.results.pop_front();
            }
            self.results.push_back(tail.clone());
        }
        Ok(tail)
    }

    /// The tail and exports of `closure`, run as an input is, stopping when
    /// `token` is cancelled.
    fn run_closure(
        &mut self,
        closure: Arc<Closure>,
        token: CancelToken,
    ) -> Result<(Value, Vec<(String, Value)>), ReplError> {
        let vm = match self.idle_vm.take() {
            Some(idle_vm) => idle_vm.build(closure),
            None => Vm::factory()
                .configuration(self.configuration.clone())
                .with_importer(Arc::clone(&self.importer))
                .build(closure),
        };
        match vm.with_cancel_token(token).run() {
            Ok(result) => {
                let ProgramParts {
                    tail,
                    exports,
                    idle_vm,
                } = result.into_parts();
                self.idle_vm = Some(idle_vm);
                Ok((tail, exports))
            }
            Err(failure) => {
                let cancelled = failure.kind() == RunErrorKind::Aborted(AbortReason::Cancelled);
                let (error, idle_vm) = failure.into_parts();
                self.idle_vm = Some(idle_vm);
                Err(if cancelled {
                    ReplError::Cancelled(error)
                } else {
                    ReplError::Run(error)
                })
            }
        }
    }

    /// The names inputs and seeded bindings have bound so far, with their
    /// values. The REPL's own [`results`](Self#recent-results) is not among
    /// them.
    pub fn bindings(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.bindings
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    /// [`evaluate`](Self::evaluate) each segment `frontend` reads, and have it
    /// [`render`](Frontend::render) the outcome, until it has no more or
    /// `:quit` ends the session. A failed input does not end the session.
    ///
    /// A segment that is a [metacommand](Self#metacommands) is run as one, not
    /// evaluated. What inputs `print` goes to the configuration's
    /// [`print_sink`](VmRuntimeConfiguration::print_sink), not the frontend.
    ///
    /// Fails if `frontend` fails, or if one of its
    /// [metacommands](Frontend::metacommands) is refused; then it reads
    /// nothing.
    ///
    /// The session starts by giving `frontend` this REPL's [`CancelHandle`]
    /// (see [`Frontend::set_cancel_handle`]), and then reading the
    /// [config file](Self#config-file), if there is one.
    pub fn run(&mut self, frontend: &mut dyn Frontend) -> Result<(), SessionError> {
        let registry = Registry::new(frontend.metacommands()).map_err(SessionError::Metacommand)?;
        frontend.set_cancel_handle(self.cancel_handle());
        if let Some(path) = self.config_file.clone() {
            self.read_config(&path, frontend, Reading::Start)?;
        }
        while let Some(segment) = frontend.read_segment()? {
            let Some(invocation) = Invocation::parse(&segment) else {
                frontend.render(self.evaluate(&segment).as_ref())?;
                continue;
            };
            let reply = match registry.find(invocation.name()) {
                Some(Handler::Builtin(builtin)) => {
                    self.run_builtin(builtin, &invocation, &registry, frontend.ansi_styling())
                }
                Some(Handler::Frontend) => {
                    frontend.metacommand(&invocation)?;
                    continue;
                }
                None => Err(ReplError::Metacommand(MetacommandError::Unknown(
                    invocation.name().to_string(),
                ))),
            };
            match reply {
                Ok(Reply::Text(text)) => frontend.render_text(&text)?,
                Ok(Reply::Nothing) => {}
                Ok(Reply::Quit) => break,
                Ok(Reply::ReadConfig(path)) => {
                    self.read_config(&path, frontend, Reading::Reload)?;
                }
                Err(error) => frontend.render(Err(&error))?,
            }
        }
        Ok(())
    }

    /// The input `source` compiles to, ready to run with every binding in
    /// scope.
    fn compile(&self, source: &str) -> Result<Arc<Closure>, ReplError> {
        let mut bindings = self.bindings.clone();
        if self.keeps_results() {
            bindings.insert(RESULTS.to_string(), self.results.iter().cloned().collect());
        }
        let scope: Vec<&str> = bindings.keys().map(String::as_str).collect();
        let options = CompilerOptions::new()
            .with_optimization(self.optimization)
            // Each top-level binding is exported, to be kept for later inputs.
            .with_implicit_export(true)
            // A session shows no lints, so finds none.
            .with_lints(false);
        let program = compile_in_scope(INPUT_NAME, source, options, &scope)
            .map_err(ReplError::Compile)?
            .code;
        Ok(program
            .close(bindings)
            .expect("every name in scope is bound"))
    }

    /// Whether the REPL binds `results`: it keeps some, and no input or seed
    /// has taken the name.
    fn keeps_results(&self) -> bool {
        self.results_limit() > 0 && !self.bindings.contains_key(RESULTS)
    }
}

/// Why [`Repl::run`] ended a session early.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The frontend failed.
    #[error("{0}")]
    Frontend(#[from] io::Error),
    /// The REPL refused one of the frontend's metacommands, before reading
    /// anything.
    #[error("{0}")]
    Metacommand(#[source] InvalidMetacommand),
}

/// Why an input failed, or what is wrong with the
/// [config file](Repl#config-file). A failed input left the REPL's bindings
/// unchanged.
///
/// [`Display`](std::fmt::Display) shows it as plain text: the diagnostics, the
/// error and its backtrace, the metacommand's failure, or the config file's
/// problem.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ReplError {
    /// The input did not compile.
    #[cfg_attr(
        feature = "graphical-diagnostics",
        error("{}", .0.render_plain().trim_end())
    )]
    #[cfg_attr(
        not(feature = "graphical-diagnostics"),
        error("{}", .0.render_narrated().trim_end())
    )]
    Compile(#[source] Diagnostics),
    /// The input raised an error while running.
    #[error("Error: {}", .0.with_backtrace())]
    Run(#[source] FrostError),
    /// The input was cancelled through a [`CancelHandle`] before it finished.
    /// The error says where it was running when it stopped.
    #[error("Error: {}", .0.with_backtrace())]
    Cancelled(#[source] FrostError),
    /// The input was a [metacommand](Repl#metacommands), which failed.
    #[error("Error: {0}")]
    Metacommand(#[source] MetacommandError),
    /// The config file has a problem, which no input caused.
    #[error("{0}")]
    Config(#[source] ConfigError),
}

/// Check that Frost source can refer to `name`, so that binding it is of use:
/// it must be an identifier, and not a keyword such as `if` or `and`.
pub fn check_name(name: &str) -> Result<(), InvalidName> {
    let mut lexed = tokens(name);
    match (lexed.next(), lexed.next()) {
        (Some((Ok(Token::Identifier(_)), span)), None) if span == (0..name.len()).into() => Ok(()),
        _ => Err(InvalidName {
            name: name.to_string(),
        }),
    }
}

/// A name Frost source cannot refer to; see [`check_name`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{name}` is not a name Frost source can refer to")]
pub struct InvalidName {
    name: String,
}

impl InvalidName {
    /// The name.
    pub fn name(&self) -> &str {
        &self.name
    }
}
