//! Assembly: lower a function's fused `Vec<Ir>` into a runnable
//! [`CompiledFunction`].
//!
//! This is the single pass that turns symbolic IR into final bytecode:
//! - `Label` markers are zero-width; each `Jump` resolves to a forward relative
//!   offset (the VM only ever jumps forward).
//! - Inline payloads (`Const`, `ConstKey`, `Closure`) are drained into their
//!   pools, and the op is rewritten to reference the assigned pool slot. With
//!   [`deduplicate_constants`](crate::compile::OptimizationOptions::deduplicate_constants),
//!   constants that are the same share a slot (see [`pool`]).
//! - `LoadLocal`, `ConsumeLocal`, and `DefLocal` ids resolve to their frame
//!   slots per the [`SlotPlan`], and a `ClearLocals` run to its consecutive
//!   slots; one whose locals occupy none emits nothing.
//! - Each instruction's span goes to the function's debug info, beside the
//!   stack height on entry to it (see [`heights`]), if the compilation keeps
//!   debug info.
//!
//! Assembly is infallible. The IR is compiler-produced and already well-formed,
//! so an undefined label or a backward jump is a compiler bug, not a user error:
//! it panics rather than returning a diagnostic.

mod heights;
mod pool;

use std::sync::Arc;

use crate::Arity;
use crate::bytecode::{
    Bytecode, CompiledFunction, DebugInfo, FormatVersion, OpDebugInfo, SourceLines,
};

use crate::compile::Diagnostic;
use crate::compile::lower::assemble::pool::Pool;
use crate::compile::lower::locals::SlotPlan;
use crate::compile::lower::passes::run_passes;
use crate::compile::lower::{ConstKeyOp, FunctionBuilder, Ir, JumpType, LoweredFunction, Op};
use crate::vm::to_operand;

impl FunctionBuilder<'_> {
    /// End lowering: package this function's fused IR with its metadata, and
    /// run the IR passes over it. The lints found in it, nested functions'
    /// included, come back beside it, for the enclosing function to take on.
    pub(super) fn finish(mut self, code: Vec<Ir>) -> (LoweredFunction, Vec<Diagnostic>) {
        if self.lints_on() {
            self.lint_unused_bindings();
        }
        let function = LoweredFunction {
            name: self.name,
            origin: self.origin,
            arity: self.arity,
            code,
            locals: self.locals,
            num_labels: self.next_label.0,
            effectful: self.effectful,
            deduplicate_constants: self.options.optimization_options.deduplicate_constants,
            debug_lines: self.debug_lines,
        };
        let function = run_passes(function, &self.options.optimization_options);
        (function, self.lints)
    }

    /// [`finish`](Self::finish) then [`LoweredFunction::assemble`], for a test
    /// that needs neither the lowered form nor the lints.
    #[cfg(test)]
    pub(super) fn assemble(self, code: Vec<Ir>) -> Arc<CompiledFunction> {
        self.finish(code).0.assemble()
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
            self.origin.clone(),
            self.arity,
            plan,
            AssemblyOptions {
                deduplicate_constants: self.deduplicate_constants,
                debug_lines: self.debug_lines.as_ref(),
            },
        )
    }
}

/// How to assemble a function, beyond its code and metadata.
pub(super) struct AssemblyOptions<'a> {
    /// See [`OptimizationOptions::deduplicate_constants`](crate::compile::OptimizationOptions::deduplicate_constants).
    pub(super) deduplicate_constants: bool,
    /// The compilation's line table, to keep debug info against; `None` keeps none.
    pub(super) debug_lines: Option<&'a Arc<SourceLines>>,
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
    origin: Option<Arc<str>>,
    arity: Arity,
    plan: SlotPlan,
    options: AssemblyOptions,
) -> Arc<CompiledFunction> {
    // Left out before labels resolve, so jump offsets count only what is
    // emitted.
    let code: Vec<&Ir> = code
        .iter()
        .filter(|ir| !matches!(&ir.op, Op::ClearLocals(run) if plan.slots_of(run).is_none()))
        .collect();
    let label_positions = resolve_labels(&code, num_labels);

    let mut out = Vec::new();
    let mut spans = Vec::new();
    let mut constants = Pool::new(options.deduplicate_constants);
    let mut key_constants = Pool::new(options.deduplicate_constants);
    let mut child_fns = Vec::new();

    for ir in code {
        let bytecode = match &ir.op {
            Op::Ready(bytecode) => {
                debug_assert!(
                    !is_symbolic_opcode(bytecode),
                    "Op::Ready holds {bytecode:?}, an opcode another Op variant owns"
                );
                *bytecode
            }
            // Zero-width: a label contributes no instruction.
            Op::Label(_) => continue,
            Op::Const(value) => Bytecode::LoadConst(to_operand(constants.add(value))),
            Op::ConstKey { op, key } => {
                let index = to_operand(key_constants.add(key));
                match op {
                    ConstKeyOp::HardIndex => Bytecode::HardIndexMap(index),
                    ConstKeyOp::Test => Bytecode::TestConstKey(index),
                    ConstKeyOp::Extract => Bytecode::ExtractConstKey(index),
                }
            }
            Op::LoadLocal(id) => Bytecode::LoadLocal(to_operand(plan.slot_of(*id))),
            Op::ConsumeLocal(id) => Bytecode::ConsumeLocal(to_operand(plan.slot_of(*id))),
            Op::DefLocal(id) => Bytecode::DefLocal(to_operand(plan.slot_of(*id))),
            Op::ClearLocals(run) => {
                let slots = plan
                    .slots_of(run)
                    .expect("a clear of no slots was left out");
                Bytecode::ClearLocals {
                    first: to_u32_operand(slots.start),
                    count: to_u32_operand(slots.len()),
                }
            }
            Op::Closure { compiled, .. } => {
                let child = to_u32_operand(child_fns.len());
                child_fns.push(Arc::clone(compiled));
                Bytecode::CreateClosure {
                    child,
                    captures: to_u32_operand(compiled.num_captures),
                }
            }
            Op::Jump { kind, label } => {
                let target =
                    label_positions[label.0].expect("jump to a label that was never emitted");
                // `Jump(n)` skips n instructions, so from site p it lands at
                // p + 1 + n. The site is the next slot to be filled.
                let offset = target
                    .checked_sub(out.len() + 1)
                    .expect("backward or self jump: the VM only jumps forward");
                jump_bytecode(*kind, offset)
            }
        };
        out.push(bytecode);
        spans.push(ir.span);
    }

    let debug_info = options.debug_lines.map(|lines| DebugInfo {
        ops: spans
            .into_iter()
            .zip(heights::entry_heights(&out, arity))
            .map(|(span, height)| OpDebugInfo {
                span,
                stack_height: height.and_then(|height| u32::try_from(height).ok()),
            })
            .collect(),
        lines: Arc::clone(lines),
    });

    Arc::new(CompiledFunction {
        version: FormatVersion,
        name,
        origin,
        code: out,
        child_fns,
        constants: constants.into_entries(),
        key_constants: key_constants.into_entries(),
        num_captures: plan.num_captures(),
        name_table: plan.into_name_table(),
        arity,
        debug_info,
    })
}

/// `n`, a count or index of child functions, captures, or slots, as a `u32`
/// operand. Each of these takes at least one byte of source, so exceeding `u32`
/// needs a source over 4 GiB.
fn to_u32_operand(n: usize) -> u32 {
    u32::try_from(n).expect("operand exceeds u32")
}

/// Whether an opcode carries an index or offset that some other `Op` variant
/// assigns at assembly. `Op::Ready` is for already-final opcodes, so it must
/// never hold one of these.
fn is_symbolic_opcode(bytecode: &Bytecode) -> bool {
    matches!(
        bytecode,
        Bytecode::LoadLocal(_)
            | Bytecode::ConsumeLocal(_)
            | Bytecode::DefLocal(_)
            | Bytecode::ClearLocals { .. }
            | Bytecode::LoadConst(_)
            | Bytecode::HardIndexMap(_)
            | Bytecode::TestConstKey(_)
            | Bytecode::ExtractConstKey(_)
            | Bytecode::CreateClosure { .. }
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
fn resolve_labels(code: &[&Ir], num_labels: usize) -> Vec<Option<usize>> {
    let mut positions = vec![None; num_labels];
    let mut index = 0;
    for ir in code {
        match ir.op {
            Op::Label(label) => {
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
    let offset = to_operand(offset);
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
