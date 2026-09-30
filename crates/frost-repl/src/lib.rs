//! An interactive Frost session: a read-eval-print loop.
//!
//! A [`Repl`] runs each piece of input as its own small program. Every
//! top-level `def` an input makes stays bound for the inputs after it, and a
//! later input may bind the same name again.
//!
//! ```no_run
//! use std::io;
//!
//! use frost_repl::{LineInput, Repl};
//!
//! fn main() -> io::Result<()> {
//!     Repl::new().run(&mut LineInput::stdin(), &mut io::stdout(), &mut io::stderr())
//! }
//! ```

mod input;

pub use input::{LineInput, ReplInput};

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::Arc;

use frost_compile::{CompilerErrors, CompilerOptions, OptimizationOptions, compile_in_scope};
use frost_runtime::{FrostError, IdleVm, Importer, Value, Vm, VmRuntimeConfiguration};

/// The name diagnostics give an input.
const INPUT_NAME: &str = "<repl>";

/// A read-eval-print loop: [`evaluate`](Self::evaluate) inputs one at a time,
/// or [`run`](Self::run) a whole session.
pub struct Repl {
    configuration: VmRuntimeConfiguration,
    importer: Arc<Importer>,
    optimization: OptimizationOptions,
    bindings: BTreeMap<String, Value>,
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
    /// [`VmRuntimeConfiguration`], and every optimization on.
    pub fn new() -> Self {
        Self {
            configuration: VmRuntimeConfiguration::default(),
            importer: Arc::default(),
            optimization: OptimizationOptions::ALL,
            bindings: BTreeMap::new(),
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

    /// Run `source` as the next input, returning its value: the value of its
    /// last expression, or Null.
    ///
    /// The input sees every binding earlier inputs made. The bindings it makes
    /// itself are kept only if it compiles and runs without error; a failed
    /// input changes nothing.
    pub fn evaluate(&mut self, source: &str) -> Result<Value, ReplError> {
        let scope: Vec<&str> = self.bindings.keys().map(String::as_str).collect();
        let options = CompilerOptions {
            optimization_options: self.optimization,
            // Each top-level binding is exported, to be kept for later inputs.
            implicit_export: true,
        };
        let program = compile_in_scope(INPUT_NAME, source, options, &scope)
            .map_err(ReplError::Compile)?
            .code;
        let closure = program
            .close(self.bindings.clone())
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
                Ok(value)
            }
            Err(failure) => {
                let error = failure.error().clone();
                self.idle_vm = Some(failure.into_idle_vm());
                Err(ReplError::Run(error))
            }
        }
    }

    /// The names bound so far, with their values.
    pub fn bindings(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.bindings
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    /// [`evaluate`](Self::evaluate) each segment `input` reads, until it has no
    /// more.
    ///
    /// Each value other than Null is written to `output`, pretty-printed as by
    /// [`Value::to_pretty_string`]; a failed input's diagnostics or error go to
    /// `errors`, and the session carries on. What inputs `print` goes to the
    /// configuration's [`print_sink`](VmRuntimeConfiguration::print_sink).
    ///
    /// Returns an error only if reading input or writing output fails.
    pub fn run(
        &mut self,
        input: &mut dyn ReplInput,
        output: &mut dyn Write,
        errors: &mut dyn Write,
    ) -> io::Result<()> {
        while let Some(segment) = input.read_segment()? {
            match self.evaluate(&segment) {
                Ok(Value::Null) => {}
                Ok(value) => writeln!(output, "{}", value.to_pretty_string())?,
                Err(ReplError::Compile(diagnostics)) => {
                    write!(errors, "{}", diagnostics.render())?;
                }
                Err(ReplError::Run(error)) => {
                    writeln!(errors, "{error}")?;
                    for frame in error.backtrace() {
                        writeln!(errors, "  in {frame}")?;
                    }
                }
            }
            output.flush()?;
            errors.flush()?;
        }
        Ok(())
    }
}

/// Why an input failed. Either way, it left the REPL's bindings unchanged.
#[derive(Debug)]
pub enum ReplError {
    /// The input did not compile.
    Compile(CompilerErrors),
    /// The input raised an error while running.
    Run(FrostError),
}
