//! Constant folding: replace a compile-time-evaluable expression fragment with
//! a load of its value.
//!
//! A fragment marked `foldable` computes its value from no runtime inputs using
//! only pure operations, so it can be evaluated now, on the real VM, and its
//! code replaced by a push of the result. It may read a local whose value is
//! compile-time known but cannot be a constant, such as a function: the
//! evaluation seats that value as a capture. Evaluating on the VM (rather than a
//! separate interpreter) keeps folding bit-for-bit consistent with runtime.
//!
//! Each maximal foldable subtree is folded exactly once, at its root. A node
//! whose children are all foldable is itself foldable, so it defers: an ancestor
//! folds it whole. A node with any non-foldable child cannot fold, so each of
//! its foldable children is a maximal subtree and folds there. A fold point
//! (e.g. a statement or a `def` rhs) folds whatever reaches it still foldable.
//!
//! If evaluation errors (e.g. division by zero), the fold is abandoned and the
//! original bytecode kept, so the error surfaces at runtime, and only if that
//! code actually executes. A VM *panic*, by contrast, is an internal compiler
//! bug and is left to propagate.

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use crate::bytecode::{Bytecode, CompiledFunction};
use crate::compile::lower::assemble::{AssemblyOptions, assemble_code};
use crate::compile::lower::{ExprFragment, FunctionBuilder, Ir, LocalId, Op, UNEMITTED, code_span};
use crate::core::find;
use crate::vm::to_operand;
use crate::{Arity, IdleVm, MapKey, Value, ValueMap, Vm, VmFactory, VmRuntimeConfiguration};

// TODO: A function is effectful once it loads an impure global, even if branch
// elimination later discards that load. A lambda whose only `print` sits in an
// eliminated branch is never foldable.

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
        let config = VmRuntimeConfiguration::default()
            .with_fuel(NonZeroU64::new(100_000))
            // `print` is impure, so no fold ever reaches it.
            .with_print_sink(Arc::new(|_: &str| unreachable!("a constant fold printed")));
        Self {
            factory: Vm::factory().configuration(config),
            parked: RefCell::new(None),
        }
    }

    /// Run a fold program with `captures` seated, returning its value, or `None`
    /// if it errors or exhausts fuel.
    fn evaluate(
        &self,
        function: Arc<CompiledFunction>,
        captures: BTreeMap<String, Value>,
    ) -> Option<Value> {
        let closure = function.assert_well_formed().close(captures).ok()?;
        let vm = match self.parked.borrow_mut().take() {
            Some(core) => core.build(closure),
            None => self.factory.build(closure),
        };
        let (value, idle) = match vm.run() {
            Ok(result) => {
                let parts = result.into_parts();
                (Some(parts.tail), parts.idle_vm)
            }
            Err(error) => (None, error.into_idle_vm()),
        };
        *self.parked.borrow_mut() = Some(idle);
        value
    }
}

/// The optimal op for a known [`Value`]: an inline push for a scalar, a
/// constant-pool load for a structured value.
///
/// `None` if the value cannot be a constant (see [`is_const_representable`]).
pub(super) fn value_to_op(value: Value) -> Option<Op> {
    let op = match value {
        Value::Null => Op::Ready(Bytecode::PushNull),
        Value::Bool(true) => Op::Ready(Bytecode::PushTrue),
        Value::Bool(false) => Op::Ready(Bytecode::PushFalse),
        Value::Int(int) => Op::Ready(Bytecode::PushInt(int)),
        Value::Float(float) => Op::Ready(Bytecode::PushFloat(float)),
        // Structured (String, Bytes, Array, Map): not inlinable,
        // so put it in the constant pool (if eligible)
        other if is_const_representable(&other) => Op::Const(other),
        _ => return None,
    };
    Some(op)
}

/// The compile-time constant a fragment loads, if its code is a single
/// value-producing op (a literal, a fold result, or a propagated constant).
pub(super) fn constant_of(code: &[Ir]) -> Option<Value> {
    let [ir] = code else {
        return None;
    };
    match &ir.op {
        Op::Ready(Bytecode::PushNull) => Some(Value::Null),
        Op::Ready(Bytecode::PushTrue) => Some(Value::Bool(true)),
        Op::Ready(Bytecode::PushFalse) => Some(Value::Bool(false)),
        Op::Ready(Bytecode::PushInt(int)) => Some(Value::Int(*int)),
        Op::Ready(Bytecode::PushFloat(float)) => Some(Value::Float(*float)),
        // An empty literal builds its structure from nothing.
        Op::Ready(Bytecode::MakeArray(0)) => Some(Value::from(Vec::<Value>::new())),
        Op::Ready(Bytecode::MakeMap(0)) => Some(Value::from(ValueMap::new())),
        Op::Const(value) => Some(value.clone()),
        _ => None,
    }
}

/// The single op loading `value` in place of `code`, which computes it,
/// derived from all of `code`'s source. `None` if `value` cannot be a constant.
fn load_of_folded(value: Value, code: &[Ir]) -> Option<Ir> {
    let span = code_span(code).expect("folded code is not empty");
    Some(value_to_op(value)?.at(span))
}

/// The compile-time key a fragment loads: its [`constant_of`], if that is a valid
/// Map key.
pub(super) fn constant_key_of(code: &[Ir]) -> Option<MapKey> {
    MapKey::try_from(constant_of(code)?).ok()
}

/// Whether `value` can be a constant: neither a Function nor an Opaque, and, for a
/// structure, holding no Function or Opaque at any depth. The pool holds neither.
fn is_const_representable(value: &Value) -> bool {
    find::find(value, |held| held.is_function() || held.is_opaque()).is_none()
}

impl FunctionBuilder<'_> {
    /// Constant-fold `fragment` if eligible, replacing its code with a load of
    /// the computed value; otherwise return it unchanged.
    pub(super) fn fold_if_eligible(&self, fragment: ExprFragment) -> ExprFragment {
        // Skip when disabled, ineligible, or already a single op: a one-op
        // fragment is already minimal, so folding it would only spend a VM run.
        // Skip too a fragment whose last op, past any clear of locals, creates
        // a closure: every path makes that closure its value, a Function, which
        // can never be a constant.
        let last_computing = fragment
            .code
            .iter()
            .rfind(|ir| !matches!(ir.op, Op::ClearLocals(_)));
        let ends_in_closure = matches!(
            last_computing,
            Some(Ir {
                op: Op::Closure { .. },
                ..
            })
        );
        if !self.options.optimization_options.constant_fold
            || !fragment.foldable
            || fragment.code.len() <= 1
            || ends_in_closure
        {
            return fragment;
        }
        let folded = self
            .evaluate(fragment.code.clone())
            .and_then(|value| load_of_folded(value, &fragment.code));
        match folded {
            Some(ir) => ExprFragment {
                code: vec![ir],
                foldable: true,
            },
            // The fragment errored, or its value cannot be a constant (holds a
            // function): keep the bytecode.
            None => fragment,
        }
    }

    /// Fold a binding's value as [`fold_if_eligible`](Self::fold_if_eligible)
    /// does, also returning the value if it is compile-time known, for the
    /// binding to record. A value that cannot be a constant, such as a function,
    /// is still returned: a later fold can use it through the binding.
    pub(super) fn fold_binding_value(
        &self,
        fragment: ExprFragment,
    ) -> (ExprFragment, Option<Value>) {
        if let Some(value) = constant_of(&fragment.code) {
            return (fragment, Some(value));
        }
        if !self.options.optimization_options.constant_fold || !fragment.foldable {
            return (fragment, None);
        }
        let Some(value) = self.evaluate(fragment.code.clone()) else {
            return (fragment, None);
        };
        let fragment = match load_of_folded(value.clone(), &fragment.code) {
            Some(ir) => ExprFragment {
                code: vec![ir],
                foldable: true,
            },
            None => fragment,
        };
        (fragment, Some(value))
    }

    /// Evaluate `code`, a foldable `( -- )` fragment that binds `bound`, and
    /// return each one's value, in order; `None` if the code errors.
    pub(super) fn fold_bindings(&self, code: Vec<Ir>, bound: &[LocalId]) -> Option<Vec<Value>> {
        let collect = bound
            .iter()
            .map(|id| Op::LoadLocal(*id).at(UNEMITTED))
            .chain([Op::Ready(Bytecode::MakeArray(to_operand(bound.len()))).at(UNEMITTED)]);
        let values = self.evaluate(code.into_iter().chain(collect).collect())?;
        let values = values
            .as_array()
            .expect("the fragment's value is the Array of its bindings");
        Some(values.as_slice().to_vec())
    }

    /// Apply the folding rule (see the module doc) to one node's children: if any
    /// is not foldable, fold each that is. Also returns whether all are foldable.
    pub(super) fn fold_siblings<const N: usize>(
        &self,
        siblings: [ExprFragment; N],
    ) -> ([ExprFragment; N], bool) {
        let all_foldable = siblings.iter().all(|sibling| sibling.foldable);
        if all_foldable {
            return (siblings, true);
        }
        (
            siblings.map(|sibling| self.fold_if_eligible(sibling)),
            false,
        )
    }

    /// [`fold_siblings`](Self::fold_siblings) for a variable number of children.
    pub(super) fn fold_sibling_list(
        &self,
        siblings: Vec<ExprFragment>,
    ) -> (Vec<ExprFragment>, bool) {
        let all_foldable = siblings.iter().all(|sibling| sibling.foldable);
        if all_foldable {
            return (siblings, true);
        }
        let folded = siblings
            .into_iter()
            .map(|sibling| self.fold_if_eligible(sibling))
            .collect();
        (folded, false)
    }

    /// Assemble a self-contained `( -- v )` fragment and evaluate it on the
    /// shared fold VM, returning its value, or `None` if there is no fold VM,
    /// the fragment [`reads_pending`](crate::compile::lower::locals::Locals::reads_pending), or the program
    /// errors.
    fn evaluate(&self, code: Vec<Ir>) -> Option<Value> {
        if self.locals.reads_pending(&code) {
            return None;
        }
        // Wrap as a zero-arg top-level function: the runner pushes the closure,
        // which the leading Pop discards; the fragment then leaves the value.
        let mut wrapped = vec![Op::Ready(Bytecode::Pop).at(UNEMITTED)];
        wrapped.extend(code);
        let (plan, captures) = self.locals.plan_fragment_slots(&wrapped);
        let function = assemble_code(
            &wrapped,
            self.next_label.0,
            "<fold>".to_string(),
            self.origin.clone(),
            Arity::Exact(0),
            plan,
            AssemblyOptions {
                // Run once and discarded: a smaller pool would save nothing,
                // and nothing reads its debug info.
                deduplicate_constants: false,
                debug_lines: None,
            },
        );
        self.fold_vm?.evaluate(function, captures)
    }
}
