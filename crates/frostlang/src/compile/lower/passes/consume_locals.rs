//! Consume locals: turn each local's last read into a move out of its slot, so
//! the value it holds is no longer shared by the slot, and a structure held
//! only there can be updated in place.
//!
//! A read is last when no path from it reaches another read of the same local.
//! Reads in separate branches are each last on their own path, so each is
//! consumed.
//!
//! One backward pass finds them, because every jump goes forward: walking
//! backward, a label is always passed before any jump to it, so what is read
//! after the label is known by the time each jump is met.
//!
//! An exported local is never consumed: its slot is read once the function
//! finishes.

use std::collections::HashSet;

use crate::compile::lower::{Ir, JumpType, LoweredFunction};

pub(super) fn consume_locals(mut function: LoweredFunction) -> LoweredFunction {
    // The locals some path from the current point reads again, and the same at
    // each label already passed.
    let mut read_later = HashSet::new();
    let mut read_after_label = vec![HashSet::new(); function.num_labels];
    for ir in function.code.iter_mut().rev() {
        match ir {
            Ir::Label(label) => read_after_label[label.0].clone_from(&read_later),
            Ir::Jump { kind, label } => {
                let at_target = &read_after_label[label.0];
                match kind {
                    // The next instruction in code order never runs after this one.
                    JumpType::Unconditional => read_later.clone_from(at_target),
                    JumpType::IfTrue
                    | JumpType::IfFalse
                    | JumpType::PeekIfTrue
                    | JumpType::PeekIfFalse => read_later.extend(at_target),
                }
            }
            Ir::LoadLocal(id) => {
                let id = *id;
                if read_later.insert(id) && !function.locals.is_exported(id) {
                    *ir = Ir::ConsumeLocal(id);
                }
            }
            // A definition replaces the slot's value: no read before it reaches a
            // read after it.
            Ir::DefLocal(id) => {
                read_later.remove(id);
            }
            _ => {}
        }
    }
    function
}
