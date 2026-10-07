//! The Frost bytecode compiler.
//!
//! # Features
//!
//! - `graphical-diagnostics` (off by default): render a [`Diagnostic`] as a
//!   drawing of the source snippet with its labels, and in the fixed styles of
//!   [`Diagnostic`]'s `render_*` methods. Without it, diagnostics render as
//!   plain narrated text. If you show Frost compiler errors to people, you
//!   probably want it. It turns on the same feature of `frostlang-parse`.

#![allow(unused)] // Keeps firing annoyingly on code that's simply not done yet
// Silence, Clippy

mod error;
mod lower;
mod optimization;

pub use error::{Diagnostic, Diagnostics};
use frostlang_runtime::TrustedProgram;
pub use lower::{compile_in_scope, compile_program};
pub use optimization::{InvalidOptimizationSetting, Optimization, OptimizationOptions};

/// Options for [`compile_program`] and [`compile_in_scope`].
///
/// The default, [`new`](Self::new), is right for most uses: it compiles a
/// script as written, with every optimization on.
///
/// ```
/// use frostlang_compile::{CompilerOptions, OptimizationOptions};
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
