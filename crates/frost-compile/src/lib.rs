//! The Frost bytecode compiler.

mod error;

pub use error::{CompilerError, CompilerErrors};

use frost_runtime::TrustedProgram;
use frost_parse::ast::Program;

// TODO: make some associated functions that just return some "reasonable presets"
// once I accumulate enough optimization options
pub struct OptimizationOptions {
    pub constant_fold: bool,
}

pub struct CompilerOptions {
    pub optimization_options: OptimizationOptions,
}

pub struct CompilerOutput {
    pub code: TrustedProgram,
}

pub fn compile(filename: &str, script: Program, options: CompilerOptions) -> Result<CompilerOutput, CompilerErrors> {
    todo!();
}
