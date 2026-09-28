//! Assembly: lower a function's fused `Vec<Ir>` into a runnable
//! [`CompiledFunction`].
//!
//! This is the single pass that turns symbolic IR into final bytecode:
//! - `Label` markers are zero-width; each `Jump` resolves to a forward relative
//!   offset (the VM only ever jumps forward).
//! - Inline payloads (`Const`, `KeyIndex`, `Closure`) are drained into their
//!   pools, and the op is rewritten to reference the assigned pool slot.
//! - `LoadLocal`, `ConsumeLocal`, and `DefLocal` ids resolve to their frame
//!   slots per the [`SlotPlan`].
//!
//! Assembly is infallible. The IR is compiler-produced and already well-formed,
//! so an undefined label or a backward jump is a compiler bug, not a user error:
//! it panics rather than returning a diagnostic.

use std::sync::Arc;

use frost_runtime::{Arity, Bytecode, CompiledFunction, FormatVersion};

use crate::lower::locals::SlotPlan;
use crate::lower::passes::run_passes;
use crate::lower::{FunctionBuilder, Ir, JumpType, LoweredFunction};

impl FunctionBuilder<'_> {
    /// End lowering: package this function's fused IR with its metadata, and
    /// run the IR passes over it.
    pub(super) fn finish(self, code: Vec<Ir>) -> LoweredFunction {
        let function = LoweredFunction {
            name: self.name,
            arity: self.arity,
            code,
            locals: self.locals,
            num_labels: self.next_label.0,
            effectful: self.effectful,
        };
        run_passes(function, &self.options.optimization_options)
    }

    /// [`finish`](Self::finish) then [`LoweredFunction::assemble`], for a
    /// function whose lowered form is not otherwise needed.
    pub(super) fn assemble(self, code: Vec<Ir>) -> Arc<CompiledFunction> {
        self.finish(code).assemble()
    }
}

impl LoweredFunction {
    /// Assemble this function into its runnable [`CompiledFunction`].
    pub(super) fn assemble(&self) -> Arc<CompiledFunction> {
        let plan = self.locals.plan_slots(&self.code);
        assemble_code(
            &self.code,
            self.num_labels,
            self.name.clone(),
            self.arity,
            plan,
        )
    }
}

/// Lower fused IR into a [`CompiledFunction`] with the given metadata.
///
/// `num_labels` bounds the label ids the code may reference (ids `< num_labels`);
/// it may over-count. This is the reusable core of [`LoweredFunction::assemble`],
/// also used to assemble a self-contained fragment for constant-folding.
pub(super) fn assemble_code(
    code: &[Ir],
    num_labels: usize,
    name: String,
    arity: Arity,
    plan: SlotPlan,
) -> Arc<CompiledFunction> {
    let label_positions = resolve_labels(code, num_labels);

    let mut out = Vec::new();
    let mut constants = Vec::new();
    let mut key_constants = Vec::new();
    let mut child_fns = Vec::new();

    for ir in code {
        match ir {
            Ir::Ready(bytecode) => {
                debug_assert!(
                    !is_symbolic_opcode(bytecode),
                    "Ir::Ready holds {bytecode:?}, an opcode another Ir variant owns"
                );
                out.push(*bytecode);
            }
            // Zero-width: a label contributes no instruction.
            Ir::Label(_) => {}
            Ir::Const(value) => {
                out.push(Bytecode::LoadConst(constants.len()));
                constants.push(value.clone());
            }
            Ir::KeyIndex(key) => {
                out.push(Bytecode::HardIndexMap(key_constants.len()));
                key_constants.push(key.clone());
            }
            Ir::LoadLocal(id) => out.push(Bytecode::LoadLocal(plan.slot_of(*id))),
            Ir::ConsumeLocal(id) => out.push(Bytecode::ConsumeLocal(plan.slot_of(*id))),
            Ir::DefLocal(id) => out.push(Bytecode::DefLocal(plan.slot_of(*id))),
            Ir::Closure { compiled, .. } => {
                out.push(Bytecode::CreateClosure(child_fns.len()));
                child_fns.push(Arc::clone(compiled));
            }
            Ir::Jump { kind, label } => {
                let target =
                    label_positions[label.0].expect("jump to a label that was never emitted");
                // `Jump(n)` skips n instructions, so from site p it lands at
                // p + 1 + n. The site is the next slot to be filled.
                let offset = target
                    .checked_sub(out.len() + 1)
                    .expect("backward or self jump: the VM only jumps forward");
                out.push(jump_bytecode(*kind, offset));
            }
        }
    }

    Arc::new(CompiledFunction {
        version: FormatVersion,
        name,
        code: out,
        child_fns,
        constants,
        key_constants,
        num_captures: plan.num_captures(),
        name_table: plan.into_name_table(),
        arity,
    })
}

/// Whether an opcode carries an index or offset that some other `Ir` variant
/// assigns at assembly. `Ir::Ready` is for already-final opcodes, so it must
/// never hold one of these.
fn is_symbolic_opcode(bytecode: &Bytecode) -> bool {
    matches!(
        bytecode,
        Bytecode::LoadLocal(_)
            | Bytecode::ConsumeLocal(_)
            | Bytecode::DefLocal(_)
            | Bytecode::LoadConst(_)
            | Bytecode::HardIndexMap(_)
            | Bytecode::CreateClosure(_)
            | Bytecode::Jump(_)
            | Bytecode::JumpIfTrue(_)
            | Bytecode::JumpIfFalse(_)
            | Bytecode::PeekJumpIfTrue(_)
            | Bytecode::PeekJumpIfFalse(_)
    )
}

/// Map each label to the final index of the instruction it precedes.
///
/// A label is zero-width, so its position is the number of real instructions
/// emitted ahead of it. `num_labels` sizes the table; every minted label id is a
/// valid index into it.
fn resolve_labels(code: &[Ir], num_labels: usize) -> Vec<Option<usize>> {
    let mut positions = vec![None; num_labels];
    let mut index = 0;
    for ir in code {
        match ir {
            Ir::Label(label) => {
                debug_assert!(
                    positions[label.0].is_none(),
                    "label {} emitted more than once",
                    label.0
                );
                positions[label.0] = Some(index);
            }
            _ => index += 1,
        }
    }
    positions
}

/// The concrete jump opcode for a [`JumpType`] and its resolved forward offset.
fn jump_bytecode(kind: JumpType, offset: usize) -> Bytecode {
    match kind {
        JumpType::Unconditional => Bytecode::Jump(offset),
        JumpType::IfTrue => Bytecode::JumpIfTrue(offset),
        JumpType::IfFalse => Bytecode::JumpIfFalse(offset),
        JumpType::PeekIfTrue => Bytecode::PeekJumpIfTrue(offset),
        JumpType::PeekIfFalse => Bytecode::PeekJumpIfFalse(offset),
    }
}

#[cfg(test)]
mod tests;
