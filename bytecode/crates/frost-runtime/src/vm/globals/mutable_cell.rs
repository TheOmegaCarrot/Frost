//! Mutable reference cells -- Frost's only built-in mutable state.

use crate::Value;

pub(super) fn mutable_cell_global() -> Value {
    super::stub("mutable_cell")
}
