//! Compiling Frost source into a program the [`Vm`](crate::Vm) runs.
//!
//! [`compile_program`] compiles a script; [`compile_in_scope`] compiles one
//! against an enclosing scope. A failure is reported as [`Diagnostics`], which
//! render for people; see the crate's [`graphical-diagnostics`](crate#features)
//! feature.

#![allow(unused)] // Keeps firing annoyingly on code that's simply not done yet
// Silence, Clippy

mod error;
mod lower;
mod optimization;

use crate::TrustedProgram;
pub use error::{Diagnostic, Diagnostics};
pub use frostlang_parse::ParseLimits;
pub use lower::{compile_in_scope, compile_program};
pub use optimization::{InvalidOptimizationSetting, Optimization, OptimizationOptions};

/// Options for [`compile_program`] and [`compile_in_scope`].
///
/// The default, [`new`](Self::new), is right for most uses: it compiles a
/// script as written, with every optimization on, under the default [`ParseLimits`].
///
/// ```
/// use frostlang::compile::{CompilerOptions, OptimizationOptions};
///
/// const OPTIONS: CompilerOptions = CompilerOptions::new()
///     .with_optimization(OptimizationOptions::NONE)
///     .with_implicit_export(true);
/// assert!(OPTIONS.implicit_export);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CompilerOptions {
    /// Which optimizations to apply.
    pub optimization_options: OptimizationOptions,
    /// Export every top-level binding, as if each carried `export`.
    pub implicit_export: bool,
    /// Look for [lints](CompilerOutput::lints). Off, the compiler spends no
    /// time looking.
    pub lints: bool,
    /// How deeply a script may nest. One nested past them is a compile error.
    pub parse_limits: ParseLimits,
}

impl CompilerOptions {
    /// Every optimization on, no implicit export, lints on, and the default [`ParseLimits`].
    pub const fn new() -> Self {
        Self {
            optimization_options: OptimizationOptions::ALL,
            implicit_export: false,
            lints: true,
            parse_limits: ParseLimits::new(),
        }
    }

    /// These options with [`optimization_options`](Self::optimization_options)
    /// set to `optimization`.
    #[must_use]
    pub const fn with_optimization(mut self, optimization: OptimizationOptions) -> Self {
        self.optimization_options = optimization;
        self
    }

    /// These options with [`implicit_export`](Self::implicit_export) on or off.
    #[must_use]
    pub const fn with_implicit_export(mut self, on: bool) -> Self {
        self.implicit_export = on;
        self
    }

    /// These options with [`lints`](Self::lints) on or off.
    #[must_use]
    pub const fn with_lints(mut self, on: bool) -> Self {
        self.lints = on;
        self
    }

    /// These options with [`parse_limits`](Self::parse_limits) set to `limits`.
    #[must_use]
    pub const fn with_parse_limits(mut self, limits: ParseLimits) -> Self {
        self.parse_limits = limits;
        self
    }
}

/// [`CompilerOptions::new`].
impl Default for CompilerOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// The result of a successful compilation.
#[derive(Debug)]
#[non_exhaustive]
pub struct CompilerOutput {
    /// The compiled program; [`close`](TrustedProgram::close) it to run it.
    pub code: TrustedProgram,
    /// What the compiler noticed that did not stop it, in source order: code
    /// that is probably a mistake, as a warning, or could be clearer, as
    /// advice. Each renders with a code naming its lint. Empty if
    /// [`lints`](CompilerOptions::lints) are off.
    pub lints: Diagnostics,
}
