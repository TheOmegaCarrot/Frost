//! The Frost bytecode compiler.

#![allow(unused)] // Keeps firing annoyingly on code that's simply not done yet
// Silence, Clippy

mod error;
mod lower;

pub use error::{CompilerError, CompilerErrors};
use frost_runtime::TrustedProgram;
pub use lower::{compile_in_scope, compile_program};

// TODO: make some associated functions that just return some "reasonable presets"
// once I accumulate enough optimization options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptimizationOptions {
    pub constant_fold: bool,
    /// Propagate a binding whose value is compile-time known: a lookup of it loads
    /// the value directly, so it is itself fold-eligible.
    pub constant_propagate: bool,
    /// Resolve a branch whose condition is compile-time known, emitting only the
    /// path taken.
    pub branch_eliminate: bool,
    /// Build a captured value that is compile-time known into the capturing
    /// function itself, rather than passing it in each time a closure is
    /// created. A captured binding's value is known only through
    /// [`constant_propagate`](Self::constant_propagate), so this has effect only
    /// alongside it.
    pub capture_hoist: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerOptions {
    pub optimization_options: OptimizationOptions,
    /// Export every top-level binding, as if each carried `export`.
    pub implicit_export: bool,
}

#[derive(Debug)]
pub struct CompilerOutput {
    pub code: TrustedProgram,
}
