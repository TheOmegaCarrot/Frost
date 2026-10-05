//! The compiler's view of the runtime globals: a name maps to its `LoadGlobal`
//! slot. A name's position in [`GLOBAL_NAMES`] is its slot, so this is just an
//! index over that list, built once to avoid a linear scan per lookup.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use frostlang_runtime::{GLOBAL_NAMES, GLOBAL_PURITY, Purity};

static SLOTS: LazyLock<BTreeMap<&'static str, usize>> = LazyLock::new(|| {
    GLOBAL_NAMES
        .iter()
        .copied()
        .enumerate()
        .map(|(i, n)| (n, i))
        .collect()
});

/// The `LoadGlobal` slot for a global name, or `None` if `name` is not a global.
pub(super) fn global_slot(name: &str) -> Option<usize> {
    SLOTS.get(name).copied()
}

/// Whether the global at `slot` is pure, and so a call to it may be constant
/// folded. `slot` must be a valid global slot (from [`global_slot`]).
pub(super) fn global_pure(slot: usize) -> bool {
    GLOBAL_PURITY[slot] == Purity::Pure
}
