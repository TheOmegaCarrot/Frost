//! Consume locals: turn each local's last read into a move out of its slot, so
//! the value it holds is no longer shared by the slot, and a structure held
//! only there can be updated in place.
//!
//! "Last" is in code order, which is sound because every jump goes forward: no
//! code earlier in the function can run after a later instruction, so once the
//! last read runs, nothing reads the slot again. Where branches each read a
//! local, only the read latest in code order is consumed; the others still copy.
//!
//! An exported local is never consumed: its slot is read once the function
//! finishes.

use std::collections::HashSet;

use crate::lower::{Ir, LoweredFunction};

pub(super) fn consume_locals(mut function: LoweredFunction) -> LoweredFunction {
    let mut read_later = HashSet::new();
    for ir in function.code.iter_mut().rev() {
        if let Ir::LoadLocal(id) = *ir
            && !function.locals.is_exported(id)
            && read_later.insert(id)
        {
            *ir = Ir::ConsumeLocal(id);
        }
    }
    function
}
