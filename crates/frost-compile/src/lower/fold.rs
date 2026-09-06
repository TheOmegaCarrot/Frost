//! Constant folding: replace a compile-time-evaluable expression fragment with
//! a load of its value.
//!
//! A fragment marked `foldable` computes its value from no runtime inputs using
//! only pure operations, so it can be evaluated now, on the real VM, and its
//! code replaced by a push of the result. Evaluating on the VM (rather than a
//! separate interpreter) keeps folding bit-for-bit consistent with runtime.
//!
//! If evaluation errors (e.g. division by zero), the fold is abandoned and the
//! original bytecode kept, so the error surfaces at runtime, and only if that
//! code actually executes. A VM *panic*, by contrast, is an internal compiler
//! bug and is left to propagate.

#[cfg(test)]
mod tests;

use frost_runtime::{Arity, Bytecode, Value, Vm};

use crate::lower::assemble::assemble_code;
use crate::lower::{ExprFragment, FunctionBuilder, Ir};

/// Emit the optimal IR for a known [`Value`]: an inline push for a scalar, a
/// constant-pool load for a structured value.
///
/// Panics on a function or opaque value: those cannot be constants, so reaching
/// here with one means the fold's eligibility check admitted a non-representable
/// value. That is an internal compiler bug, and failing loudly beats emitting
/// bad bytecode.
pub(super) fn value_to_ir(value: Value) -> Ir {
    match value {
        Value::Null => Ir::Ready(Bytecode::PushNull),
        Value::Bool(true) => Ir::Ready(Bytecode::PushTrue),
        Value::Bool(false) => Ir::Ready(Bytecode::PushFalse),
        Value::Int(int) => Ir::Ready(Bytecode::PushInt(int)),
        Value::Float(float) => Ir::Ready(Bytecode::PushFloat(float)),
        // Structured, but the pool can hold these: not inlinable in an opcode.
        value @ (Value::String(_) | Value::Bytes(_) | Value::Array(_) | Value::Map(_)) => {
            Ir::Const(value)
        }
        Value::NativeFunction(_) | Value::Closure(_) | Value::Opaque(_) => {
            panic!("ICE: constant-folded to a function or opaque, which cannot be a constant")
        }
    }
}

impl FunctionBuilder<'_> {
    /// Constant-fold `fragment` if eligible, replacing its code with a load of
    /// the computed value; otherwise return it unchanged.
    pub(super) fn fold(&self, fragment: ExprFragment) -> ExprFragment {
        // Skip when disabled, ineligible, or already a single op: a one-op
        // fragment is already minimal, so folding it would only spend a VM run.
        if !self.options.optimization_options.constant_fold
            || !fragment.foldable
            || fragment.code.len() <= 1
        {
            return fragment;
        }
        match self.evaluate(fragment.code.clone()) {
            // A basic-foldable fragment always yields a non-function value, so it
            // is always constant-representable. (Folding through lambdas will add
            // a transitive "not a function" check on the value here.)
            Some(value) => ExprFragment {
                code: vec![value_to_ir(value)],
                foldable: true,
            },
            // Provably-erroring user code: keep the bytecode to error at runtime.
            None => fragment,
        }
    }

    /// Evaluate a self-contained `( -- v )` fragment on the VM, returning its
    /// value, or `None` if the program errors.
    fn evaluate(&self, code: Vec<Ir>) -> Option<Value> {
        // Wrap as a zero-arg top-level function: the runner pushes the closure,
        // which the leading Pop discards; the fragment then leaves the value.
        let mut wrapped = vec![Ir::Ready(Bytecode::Pop)];
        wrapped.extend(code);
        let function = assemble_code(
            wrapped,
            self.next_label.0,
            "<fold>".to_string(),
            Arity::Exact(0),
            Vec::new(),
            0,
        );
        let closure = function.assert_trusted().into_closure().ok()?;
        let result = Vm::factory().build(closure).ok()?.run().ok()?;
        Some(result.tail().clone())
    }
}
