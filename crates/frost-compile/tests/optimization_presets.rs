//! The optimization presets on [`OptimizationOptions`].

use frost_compile::OptimizationOptions;

#[test]
fn none_turns_every_optimization_off() {
    // No `..`: a new option fails to compile here until its preset is checked.
    let OptimizationOptions {
        constant_fold,
        constant_propagate,
        branch_eliminate,
        capture_hoist,
        consume_locals,
        deduplicate_constants,
    } = OptimizationOptions::NONE;
    let options = [
        constant_fold,
        constant_propagate,
        branch_eliminate,
        capture_hoist,
        consume_locals,
        deduplicate_constants,
    ];
    assert!(options.iter().all(|on| !on), "{options:?}");
}

#[test]
fn all_turns_every_optimization_on() {
    // No `..`: a new option fails to compile here until its preset is checked.
    let OptimizationOptions {
        constant_fold,
        constant_propagate,
        branch_eliminate,
        capture_hoist,
        consume_locals,
        deduplicate_constants,
    } = OptimizationOptions::ALL;
    let options = [
        constant_fold,
        constant_propagate,
        branch_eliminate,
        capture_hoist,
        consume_locals,
        deduplicate_constants,
    ];
    assert!(options.iter().all(|on| *on), "{options:?}");
}
