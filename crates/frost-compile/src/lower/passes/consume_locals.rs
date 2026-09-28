//! Consume locals: turn each local's last read into a move out of its slot, so
//! the value it holds is no longer shared by the slot, and a structure held
//! only there can be updated in place.

use crate::lower::LoweredFunction;

// TODO: rewrite each last-use `LoadLocal` into a consuming load.
pub(super) fn consume_locals(function: LoweredFunction) -> LoweredFunction {
    function
}
