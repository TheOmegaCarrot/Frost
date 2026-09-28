//! IR passes: rewrites of a whole lowered function, run after lowering and
//! before assembly.
//!
//! Lowering emits correct IR on its own; every pass here is an optional
//! optimization, gated by its own [`OptimizationOptions`] flag, that returns
//! IR computing the same result.

mod consume_locals;

use crate::OptimizationOptions;
use crate::lower::LoweredFunction;

/// A pass: takes a function and returns it rewritten.
type Pass = fn(LoweredFunction) -> LoweredFunction;

/// Run every pass `options` enables over `function`, in order.
pub(super) fn run_passes(
    function: LoweredFunction,
    options: &OptimizationOptions,
) -> LoweredFunction {
    let passes: [(bool, Pass); 1] = [(options.consume_locals, consume_locals::consume_locals)];
    passes
        .into_iter()
        .filter(|&(enabled, _)| enabled)
        .fold(function, |function, (_, pass)| pass(function))
}
