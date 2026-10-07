//! IR passes: rewrites of a whole lowered function, run after lowering and
//! before assembly.
//!
//! Lowering emits correct IR on its own; every pass here is an optional
//! optimization, gated by its own [`OptimizationOptions`] flags, that returns
//! IR computing the same result.

mod consume_locals;
mod dead_code;

use crate::compile::OptimizationOptions;
use crate::compile::lower::LoweredFunction;

/// Run every pass `options` enables over `function`, in order.
pub(super) fn run_passes(
    function: LoweredFunction,
    options: &OptimizationOptions,
) -> LoweredFunction {
    let passes: [(bool, &dyn Fn(LoweredFunction) -> LoweredFunction); 2] = [
        // Before consuming locals: removing a read can change which read is last.
        (
            options.dead_store_eliminate || options.discard_eliminate,
            &|function| dead_code::eliminate_dead_code(function, options),
        ),
        (options.consume_locals, &consume_locals::consume_locals),
    ];
    passes
        .into_iter()
        .filter(|&(enabled, _)| enabled)
        .fold(function, |function, (_, pass)| pass(function))
}
