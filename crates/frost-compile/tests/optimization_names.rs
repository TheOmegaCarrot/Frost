//! [`Optimization`]: each optimization by name, switched on
//! [`OptimizationOptions`] one at a time.

use std::collections::HashSet;

use frost_compile::{Optimization, OptimizationOptions};

#[test]
fn each_optimization_switches_its_own_field() {
    // No `..`: a new option fails to compile here until it has an
    // `Optimization`.
    let names = |options: OptimizationOptions| {
        let OptimizationOptions {
            constant_fold,
            constant_propagate,
            branch_eliminate,
            capture_hoist,
            consume_locals,
            deduplicate_constants,
        } = options;
        [
            ("constant_fold", constant_fold),
            ("constant_propagate", constant_propagate),
            ("branch_eliminate", branch_eliminate),
            ("capture_hoist", capture_hoist),
            ("consume_locals", consume_locals),
            ("deduplicate_constants", deduplicate_constants),
        ]
        .into_iter()
        .filter(|(_, on)| *on)
        .map(|(field, _)| field)
        .collect::<Vec<_>>()
    };
    for optimization in Optimization::ALL {
        let mut options = OptimizationOptions::NONE;
        options.set(optimization, true);
        let field = optimization.name().replace('-', "_");
        assert_eq!(names(options), [field.as_str()], "{optimization:?}");
    }
}

#[test]
fn every_optimization_on_is_all_and_every_one_off_is_none() {
    let mut options = OptimizationOptions::NONE;
    for optimization in Optimization::ALL {
        options.set(optimization, true);
    }
    assert_eq!(options, OptimizationOptions::ALL);
    for optimization in Optimization::ALL {
        options.set(optimization, false);
    }
    assert_eq!(options, OptimizationOptions::NONE);
}

#[test]
fn get_reads_what_set_wrote() {
    for optimization in Optimization::ALL {
        let mut options = OptimizationOptions::ALL;
        assert!(options.get(optimization), "{optimization:?}");
        options.set(optimization, false);
        assert!(!options.get(optimization), "{optimization:?}");
        // The others are untouched.
        let others_on = Optimization::ALL
            .into_iter()
            .filter(|&other| other != optimization)
            .all(|other| options.get(other));
        assert!(others_on, "{optimization:?}");
    }
}

#[test]
fn names_are_kebab_case_and_distinct() {
    let names: Vec<&str> = Optimization::ALL.map(Optimization::name).to_vec();
    assert_eq!(
        names,
        [
            "constant-fold",
            "constant-propagate",
            "branch-eliminate",
            "capture-hoist",
            "consume-locals",
            "deduplicate-constants",
        ]
    );
    assert_eq!(names.iter().collect::<HashSet<_>>().len(), names.len());
}

#[test]
fn from_name_finds_each_optimization_by_its_name() {
    for optimization in Optimization::ALL {
        assert_eq!(
            Optimization::from_name(optimization.name()),
            Some(optimization)
        );
    }
}

#[test]
fn from_name_finds_nothing_for_any_other_name() {
    for name in [
        "",
        "constant_fold",
        "ConstantFold",
        "constant-fold ",
        " constant-fold",
        "constant",
        "all",
        "none",
    ] {
        assert_eq!(Optimization::from_name(name), None, "{name:?}");
    }
}
