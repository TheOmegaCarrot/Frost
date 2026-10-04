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
    /// Move a local's value out on its last use, rather than copying it, so a
    /// structure held only by that local can be updated in place.
    pub consume_locals: bool,
    /// Store identical constants once per function, however many places use
    /// them, making the compiled program smaller.
    pub deduplicate_constants: bool,
}

impl OptimizationOptions {
    /// Every optimization off: the program compiles as written.
    pub const NONE: Self = Self {
        constant_fold: false,
        constant_propagate: false,
        branch_eliminate: false,
        capture_hoist: false,
        consume_locals: false,
        deduplicate_constants: false,
    };

    /// Every optimization on.
    pub const ALL: Self = Self {
        constant_fold: true,
        constant_propagate: true,
        branch_eliminate: true,
        capture_hoist: true,
        consume_locals: true,
        deduplicate_constants: true,
    };

    /// Whether `optimization` is on.
    pub fn get(self, optimization: Optimization) -> bool {
        let mut options = self;
        *options.flag(optimization)
    }

    /// Turn `optimization` on or off.
    pub fn set(&mut self, optimization: Optimization, on: bool) {
        *self.flag(optimization) = on;
    }

    fn flag(&mut self, optimization: Optimization) -> &mut bool {
        match optimization {
            Optimization::ConstantFold => &mut self.constant_fold,
            Optimization::ConstantPropagate => &mut self.constant_propagate,
            Optimization::BranchEliminate => &mut self.branch_eliminate,
            Optimization::CaptureHoist => &mut self.capture_hoist,
            Optimization::ConsumeLocals => &mut self.consume_locals,
            Optimization::DeduplicateConstants => &mut self.deduplicate_constants,
        }
    }
}

/// One of the optimizations [`OptimizationOptions`] turns on or off, each
/// described on its field there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Optimization {
    /// [`OptimizationOptions::constant_fold`].
    ConstantFold,
    /// [`OptimizationOptions::constant_propagate`].
    ConstantPropagate,
    /// [`OptimizationOptions::branch_eliminate`].
    BranchEliminate,
    /// [`OptimizationOptions::capture_hoist`].
    CaptureHoist,
    /// [`OptimizationOptions::consume_locals`].
    ConsumeLocals,
    /// [`OptimizationOptions::deduplicate_constants`].
    DeduplicateConstants,
}

impl Optimization {
    /// Every optimization, in the order [`OptimizationOptions`] declares them.
    pub const ALL: [Self; 6] = [
        Self::ConstantFold,
        Self::ConstantPropagate,
        Self::BranchEliminate,
        Self::CaptureHoist,
        Self::ConsumeLocals,
        Self::DeduplicateConstants,
    ];

    /// The name tools give it: its field's name in kebab-case, as
    /// `constant-fold`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ConstantFold => "constant-fold",
            Self::ConstantPropagate => "constant-propagate",
            Self::BranchEliminate => "branch-eliminate",
            Self::CaptureHoist => "capture-hoist",
            Self::ConsumeLocals => "consume-locals",
            Self::DeduplicateConstants => "deduplicate-constants",
        }
    }

    /// The optimization with the [`name`](Self::name) `name`, if any.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|optimization| optimization.name() == name)
    }
}

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
