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

use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, IdleVm, Value, Vm, VmFactory, VmRuntimeConfiguration,
};

use crate::lower::assemble::assemble_code;
use crate::lower::{ExprFragment, FunctionBuilder, Ir};

/// A fuel-capped VM reused across a compilation to evaluate constant folds.
pub(super) struct FoldVm {
    factory: VmFactory,
    parked: RefCell<Option<IdleVm>>,
}

impl std::fmt::Debug for FoldVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FoldVm").finish_non_exhaustive()
    }
}

impl FoldVm {
    pub(super) fn new() -> Self {
        // Bounds a runaway fold. Reset per fold; generous because Frost iterates
        // by function call.
        let config = VmRuntimeConfiguration {
            fuel: NonZeroUsize::new(100_000),
            ..Default::default()
        };
        Self {
            factory: Vm::factory().configuration(config),
            parked: RefCell::new(None),
        }
    }

    /// Run a self-contained fold program, returning its value, or `None` if it
    /// errors or exhausts fuel.
    fn evaluate(&self, function: Arc<CompiledFunction>) -> Option<Value> {
        let closure = function.assert_trusted().into_closure().ok()?;
        let vm = match self.parked.borrow_mut().take() {
            Some(core) => core.build(closure),
            None => self.factory.build(closure).ok()?,
        };
        let (value, idle) = match vm.run() {
            Ok(result) => (Some(result.tail().clone()), result.into_idle_vm()),
            Err(error) => (None, error.into_idle_vm()),
        };
        *self.parked.borrow_mut() = Some(idle);
        value
    }
}

/// The optimal IR for a known [`Value`]: an inline push for a scalar, a
/// constant-pool load for a structured value.
///
/// `None` if the value is not const-representable: the pool cannot hold a
/// function or opaque, or a structure that transitively contains one.
pub(super) fn value_to_ir(value: Value) -> Option<Ir> {
    let ir = match value {
        Value::Null => Ir::Ready(Bytecode::PushNull),
        Value::Bool(true) => Ir::Ready(Bytecode::PushTrue),
        Value::Bool(false) => Ir::Ready(Bytecode::PushFalse),
        Value::Int(int) => Ir::Ready(Bytecode::PushInt(int)),
        Value::Float(float) => Ir::Ready(Bytecode::PushFloat(float)),
        // Structured (String, Bytes, Array, Map): not inlinable,
        // so put it in the constant pool (if eligible)
        other if is_const_representable(&other) => Ir::Const(other),
        _ => return None,
    };
    Some(ir)
}

/// Whether `value` can be a constant: neither a function nor opaque, and, for a
/// structure, holding no function or opaque at any depth.
fn is_const_representable(value: &Value) -> bool {
    match value {
        Value::Array(array) => array.iter().all(is_const_representable),
        Value::Map(map) => map.values().all(is_const_representable),
        other => !(other.is_function() || other.is_opaque()),
    }
}

impl FunctionBuilder<'_> {
    /// Constant-fold `fragment` if eligible, replacing its code with a load of
    /// the computed value; otherwise return it unchanged.
    pub(super) fn fold_if_eligible(&self, fragment: ExprFragment) -> ExprFragment {
        // Skip when disabled, ineligible, or already a single op: a one-op
        // fragment is already minimal, so folding it would only spend a VM run.
        if !self.options.optimization_options.constant_fold
            || !fragment.foldable
            || fragment.code.len() <= 1
        {
            return fragment;
        }
        match self.evaluate(fragment.code.clone()).and_then(value_to_ir) {
            Some(ir) => ExprFragment {
                code: vec![ir],
                foldable: true,
            },
            // The fragment errored, or its value cannot be a constant (holds a
            // function): keep the bytecode.
            None => fragment,
        }
    }

    /// Assemble a self-contained `( -- v )` fragment and evaluate it on the
    /// shared fold VM, returning its value, or `None` if there is no fold VM or
    /// the program errors.
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
        self.fold_vm?.evaluate(function)
    }
}
