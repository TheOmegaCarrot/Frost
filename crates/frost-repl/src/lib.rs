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
//! use std::io;
//!
//! use frost_repl::Repl;
//!
//! fn main() -> io::Result<()> {
//!     let mut frontend = frost_repl::default_frontend(true);
//!     Repl::new().run(&mut *frontend)
//! }
//! ```
//!
//! # Features
//!
//! - `line-editor` (default): [`TerminalFrontend`], with line editing,
//!   history, and syntax highlighting.

mod frontend;
#[cfg(feature = "line-editor")]
mod highlight;
mod scripted;
mod segment;
#[cfg(feature = "line-editor")]
mod terminal;

pub use frontend::{Frontend, LineFrontend};
pub use scripted::{ScriptedFrontend, Transcript};
pub use segment::complete_segment;
#[cfg(feature = "line-editor")]
pub use terminal::TerminalFrontend;

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::io;
use std::sync::Arc;

use frost_compile::{CompilerErrors, CompilerOptions, OptimizationOptions, compile_in_scope};
use frost_parse::{Token, tokens};
use frost_runtime::{FrostError, IdleVm, Importer, Value, Vm, VmRuntimeConfiguration};

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
/// `results` is an ordinary name. Once an input or a seeded binding binds it,
/// the REPL leaves it alone and keeps no more results.
pub struct Repl {
    configuration: VmRuntimeConfiguration,
    importer: Arc<Importer>,
    optimization: OptimizationOptions,
    bindings: BTreeMap<String, Value>,
    results_kept: usize,
    // Oldest first.
    results: VecDeque<Value>,
    // The Vm the last input ran on, kept warm for the next.
    idle_vm: Option<IdleVm>,
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

impl Repl {
    /// A REPL with nothing bound, nothing importable, the default
    /// [`VmRuntimeConfiguration`], every optimization on, and five recent
    /// results kept.
    pub fn new() -> Self {
        Self {
            configuration: VmRuntimeConfiguration::default(),
            importer: Arc::default(),
            optimization: OptimizationOptions::ALL,
            bindings: BTreeMap::new(),
            results_kept: 5,
            results: VecDeque::new(),
            idle_vm: None,
        }
    }

    /// Set the configuration every input runs under.
    pub fn with_configuration(mut self, configuration: VmRuntimeConfiguration) -> Self {
        self.configuration = configuration;
        self
    }

    /// Set what inputs may import.
    pub fn with_importer(mut self, importer: Arc<Importer>) -> Self {
        self.importer = importer;
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
        let excess = self.results.len().saturating_sub(count);
        self.results.drain(..excess);
        self
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
    pub fn evaluate(&mut self, source: &str) -> Result<Value, ReplError> {
        let mut bindings = self.bindings.clone();
        if self.keeps_results() {
            bindings.insert(RESULTS.to_string(), self.results.iter().cloned().collect());
        }
        let scope: Vec<&str> = bindings.keys().map(String::as_str).collect();
        let options = CompilerOptions {
            optimization_options: self.optimization,
            // Each top-level binding is exported, to be kept for later inputs.
            implicit_export: true,
        };
        let program = compile_in_scope(INPUT_NAME, source, options, &scope)
            .map_err(ReplError::Compile)?
            .code;
        let closure = program
            .close(bindings)
            .expect("every name in scope is bound");
        let vm = match self.idle_vm.take() {
            Some(idle_vm) => idle_vm.build(closure),
            None => Vm::factory()
                .configuration(self.configuration.clone())
                .with_importer(Arc::clone(&self.importer))
                .build(closure)
                .map_err(ReplError::Run)?,
        };
        match vm.run() {
            Ok(result) => {
                let value = result.tail().clone();
                self.bindings.extend(
                    result
                        .exports()
                        .map(|(name, value)| (name.to_string(), value.clone())),
                );
                self.idle_vm = Some(result.into_idle_vm());
                if self.keeps_results() && value != Value::Null {
                    if self.results.len() == self.results_kept {
                        self.results.pop_front();
                    }
                    self.results.push_back(value.clone());
                }
                Ok(value)
            }
            Err(failure) => {
                let error = failure.error().clone();
                self.idle_vm = Some(failure.into_idle_vm());
                Err(ReplError::Run(error))
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

    /// Whether the REPL binds `results`: it keeps some, and no input or seed
    /// has taken the name.
    fn keeps_results(&self) -> bool {
        self.results_kept > 0 && !self.bindings.contains_key(RESULTS)
    }

    /// [`evaluate`](Self::evaluate) each segment `frontend` reads, and have it
    /// [`render`](Frontend::render) the outcome, until it has no more. A failed
    /// input does not end the session.
    ///
    /// What inputs `print` goes to the configuration's
    /// [`print_sink`](VmRuntimeConfiguration::print_sink), not the frontend.
    ///
    /// Returns an error only if `frontend` fails.
    pub fn run(&mut self, frontend: &mut dyn Frontend) -> io::Result<()> {
        while let Some(segment) = frontend.read_segment()? {
            frontend.render(self.evaluate(&segment).as_ref())?;
        }
        Ok(())
    }
}

/// Why an input failed. Either way, it left the REPL's bindings unchanged.
///
/// [`Display`](fmt::Display) shows it as plain text: the diagnostics, or the
/// error and its backtrace.
#[derive(Debug, Clone)]
pub enum ReplError {
    /// The input did not compile.
    Compile(CompilerErrors),
    /// The input raised an error while running.
    Run(FrostError),
}

impl fmt::Display for ReplError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(diagnostics) => f.write_str(diagnostics.render_plain().trim_end()),
            Self::Run(error) => {
                write!(f, "{error}")?;
                error
                    .backtrace()
                    .iter()
                    .try_for_each(|frame| write!(f, "\n  in {frame}"))
            }
        }
    }
}

impl std::error::Error for ReplError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Compile(diagnostics) => Some(diagnostics),
            Self::Run(error) => Some(error),
        }
    }
}

/// Check that Frost source can refer to `name`, so that binding it is of use:
/// it must be an identifier, and not a keyword such as `if` or `and`.
pub fn check_name(name: &str) -> Result<(), InvalidName> {
    let mut lexed = tokens(name);
    match (lexed.next(), lexed.next()) {
        (Some((Ok(Token::Identifier(_)), span)), None) if span == (0..name.len()) => Ok(()),
        _ => Err(InvalidName {
            name: name.to_string(),
        }),
    }
}

/// A name Frost source cannot refer to; see [`check_name`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidName {
    name: String,
}

impl InvalidName {
    /// The name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for InvalidName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not a name Frost source can refer to", self.name)
    }
}

impl std::error::Error for InvalidName {}
