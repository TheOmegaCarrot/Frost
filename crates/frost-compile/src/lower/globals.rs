//! The compiler's view of the runtime globals: a name maps to its `LoadGlobal`
//! slot. A name's position in [`GLOBAL_NAMES`] is its slot, so this is just an
//! index over that list, built once to avoid a linear scan per lookup.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use frost_runtime::GLOBAL_NAMES;

// A `BTreeMap` beats hashing at this size (a few dozen fixed names).
static SLOTS: LazyLock<BTreeMap<&'static str, usize>> =
    LazyLock::new(|| GLOBAL_NAMES.iter().copied().enumerate().map(|(i, n)| (n, i)).collect());

/// The `LoadGlobal` slot for a global name, or `None` if `name` is not a global.
pub(super) fn global_slot(name: &str) -> Option<usize> {
    SLOTS.get(name).copied()
}
