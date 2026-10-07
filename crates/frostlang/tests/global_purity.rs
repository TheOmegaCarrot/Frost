//! The `GLOBAL_PURITY` table: its parallel-array invariant with `GLOBAL_NAMES`,
//! and the classification the compiler relies on to decide fold eligibility.
//!
//! Purity is a correctness contract: marking an impure global `Pure` would let
//! the compiler evaluate a side-effecting or nondeterministic call at compile
//! time. These tests pin the classification so a change to it is deliberate.

use frostlang::bytecode::{GLOBAL_NAMES, GLOBAL_PURITY, Purity};

/// The declared purity of a global by name.
fn purity(name: &str) -> Purity {
    let slot = GLOBAL_NAMES
        .iter()
        .position(|&n| n == name)
        .unwrap_or_else(|| panic!("`{name}` is not a global"));
    GLOBAL_PURITY[slot]
}

#[test]
fn names_and_purity_stay_parallel() {
    // Same slot order, so a global's name and purity share an index.
    assert_eq!(
        GLOBAL_NAMES.len(),
        GLOBAL_PURITY.len(),
        "every global has exactly one purity entry"
    );
}

#[test]
fn representative_pure_globals_are_pure() {
    // Deterministic, side-effect-free: type checks, conversions, string ops,
    // operators, and the collection transforms are all foldable.
    for name in [
        "is_int",
        "type",
        "to_string",
        "to_upper",
        "plus",
        "len",
        "range",
        "transform",
        "select",
        "fold",
        "sorted",
        "compose",
        "call",
        // Higher-order natives that only invoke a callback: pure when the
        // callback is, and an impure callback stops its own fold, not this flag.
        "tap",
        "each",
        // Raising is not an external effect: a fold reaching it just abandons.
        "error",
        "assert",
        // A debug-oriented formatter; returns a string, prints nothing.
        "debug_dump",
    ] {
        assert_eq!(purity(name), Purity::Pure, "`{name}` should be Pure");
    }
}

#[test]
fn side_effecting_globals_are_impure() {
    // The only globals with an effect of their own: I/O, mutable state, imports,
    // and `imported`, which reads live Vm state and so is never a constant.
    for name in ["print", "mprint", "mutable_cell", "import", "imported"] {
        assert_eq!(purity(name), Purity::Impure, "`{name}` should be Impure");
    }
}
