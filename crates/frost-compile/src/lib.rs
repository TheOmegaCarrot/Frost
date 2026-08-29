//! The Frost bytecode compiler.

#![allow(unused)] // Keeps firing annoyingly on code that's simply not done yet
// Silence, Clippy

mod error;
mod lower;

pub use error::{CompilerError, CompilerErrors};
use frost_runtime::TrustedProgram;
pub use lower::compile_program;

// TODO: make some associated functions that just return some "reasonable presets"
// once I accumulate enough optimization options
#[derive(Debug)]
pub struct OptimizationOptions {
    pub constant_fold: bool,
}

#[derive(Debug)]
pub struct CompilerOptions {
    pub optimization_options: OptimizationOptions,
}

#[derive(Debug)]
pub struct CompilerOutput {
    pub code: TrustedProgram,
}
