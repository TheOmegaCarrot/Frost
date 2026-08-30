//! Assembly: lower a function's fused `Vec<Ir>` into a runnable
//! [`CompiledFunction`].
//!
//! This is the single pass that turns symbolic IR into final bytecode:
//! - `Label` markers are zero-width; each `Jump` resolves to a forward relative
//!   offset (the VM only ever jumps forward).
//! - Inline payloads (`Const`, `KeyIndex`, `Closure`) are drained into their
//!   pools, and the op is rewritten to reference the assigned pool slot.
//!
//! Assembly is infallible. The IR is compiler-produced and already well-formed,
//! so an undefined label or a backward jump is a compiler bug, not a user error:
//! it panics rather than returning a diagnostic.

use std::sync::Arc;

use frost_runtime::{Bytecode, CompiledFunction, FormatVersion};

use super::{FunctionBuilder, Ir, JumpType};

impl FunctionBuilder<'_> {
    /// Lower this function's fused IR into its [`CompiledFunction`].
    /// Consumes the builder: its metadata moves into the result.
    pub(super) fn assemble(self, code: Vec<Ir>) -> Arc<CompiledFunction> {
        let label_positions = resolve_labels(&code, self.next_label.0);

        let mut out = Vec::new();
        let mut constants = Vec::new();
        let mut key_constants = Vec::new();
        let mut child_fns = Vec::new();

        for ir in code {
            match ir {
                Ir::Ready(bytecode) => out.push(bytecode),
                // Zero-width: a label contributes no instruction.
                Ir::Label(_) => {}
                Ir::Const(value) => {
                    out.push(Bytecode::LoadConst(constants.len()));
                    constants.push(value);
                }
                Ir::KeyIndex(key) => {
                    out.push(Bytecode::HardIndexMap(key_constants.len()));
                    key_constants.push(key);
                }
                Ir::Closure { function, .. } => {
                    out.push(Bytecode::CreateClosure(child_fns.len()));
                    child_fns.push(function);
                }
                Ir::Jump { kind, label } => {
                    let target =
                        label_positions[label.0].expect("jump to a label that was never emitted");
                    // `Jump(n)` skips n instructions, so from site p it lands at
                    // p + 1 + n. The site is the next slot to be filled.
                    let offset = target
                        .checked_sub(out.len() + 1)
                        .expect("backward or self jump: the VM only jumps forward");
                    out.push(jump_bytecode(kind, offset));
                }
            }
        }

        Arc::new(CompiledFunction {
            version: FormatVersion,
            name: self.name,
            code: out,
            child_fns,
            constants,
            key_constants,
            name_table: self.locals.into_name_table(),
            num_captures: self.num_captures,
            arity: self.arity,
        })
    }
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
