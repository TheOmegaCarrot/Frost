//! Checks that every field of [`OptimizationOptions`] has its [`Optimization`].
//!
//! Outside this crate the options are non-exhaustive, so only here can a
//! pattern name every field: a new field fails to compile here until it is
//! listed, and the test then requires an `Optimization` that switches it.

use crate::{Optimization, OptimizationOptions};

/// The names of the fields that are on in `options`.
fn fields_on(options: OptimizationOptions) -> Vec<&'static str> {
    // No `..`: a new field fails to compile here until it is listed.
    let OptimizationOptions {
        constant_fold,
        constant_propagate,
        branch_eliminate,
        capture_hoist,
        dead_store_eliminate,
        discard_eliminate,
        consume_locals,
        deduplicate_constants,
    } = options;
    [
        ("constant_fold", constant_fold),
        ("constant_propagate", constant_propagate),
        ("branch_eliminate", branch_eliminate),
        ("capture_hoist", capture_hoist),
        ("dead_store_eliminate", dead_store_eliminate),
        ("discard_eliminate", discard_eliminate),
        ("consume_locals", consume_locals),
        ("deduplicate_constants", deduplicate_constants),
    ]
    .into_iter()
    .filter(|(_, on)| *on)
    .map(|(field, _)| field)
    .collect()
}

#[test]
fn each_optimization_switches_its_own_field() {
    for &optimization in Optimization::ALL {
        let field = optimization.name().replace('-', "_");
        assert_eq!(
            fields_on(OptimizationOptions::NONE.with(optimization, true)),
            [field.as_str()],
            "{optimization:?} turns on only the field it names"
        );
    }
}

#[test]
fn every_field_has_an_optimization() {
    assert_eq!(
        fields_on(OptimizationOptions::ALL).len(),
        Optimization::ALL.len(),
        "each field is switched by exactly one optimization"
    );
}
