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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerOptions {
    /// Which optimizations to apply.
    pub optimization_options: OptimizationOptions,
    /// Export every top-level binding, as if each carried `export`.
    pub implicit_export: bool,
}

/// The result of a successful compilation.
#[derive(Debug)]
pub struct CompilerOutput {
    /// The compiled program; [`close`](TrustedProgram::close) it to run it.
    pub code: TrustedProgram,
}
