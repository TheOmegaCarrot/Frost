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
pub use lower::{compile_in_scope, compile_program};
pub use optimization::{InvalidOptimizationSetting, Optimization, OptimizationOptions};

/// Options for [`compile_program`] and [`compile_in_scope`].
///
/// The default, [`new`](Self::new), is right for most uses: it compiles a
/// script as written, with every optimization on.
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
}

impl CompilerOptions {
    /// Every optimization on, and no implicit export.
    pub const fn new() -> Self {
        Self {
            optimization_options: OptimizationOptions::ALL,
            implicit_export: false,
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
}
