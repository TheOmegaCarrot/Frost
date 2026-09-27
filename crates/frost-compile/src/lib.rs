//! The Frost bytecode compiler.

#![allow(unused)] // Keeps firing annoyingly on code that's simply not done yet
// Silence, Clippy

mod error;
mod lower;

pub use error::{CompilerError, CompilerErrors};
use frost_runtime::TrustedProgram;
pub use lower::{compile_in_scope, compile_program};

/// Which optimizations the compiler applies. None changes what a program
/// computes, only how it computes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptimizationOptions {
    /// Evaluate an expression built only from compile-time-known values and pure
    /// operations at compile time, emitting just its value.
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

impl OptimizationOptions {
    /// Every optimization off: the program compiles as written.
    pub const NONE: Self = Self {
        constant_fold: false,
        constant_propagate: false,
        branch_eliminate: false,
        capture_hoist: false,
    };

    /// Every optimization on.
    pub const ALL: Self = Self {
        constant_fold: true,
        constant_propagate: true,
        branch_eliminate: true,
        capture_hoist: true,
    };
}

/// Options for [`compile_program`] and [`compile_in_scope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerOptions {
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
