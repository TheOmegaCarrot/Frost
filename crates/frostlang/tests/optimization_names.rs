//! [`Optimization`]: each optimization by name, switched on
//! [`OptimizationOptions`] one at a time or through settings as tools write
//! them.

use std::collections::HashSet;

use frostlang::compile::{InvalidOptimizationSetting, Optimization, OptimizationOptions};

#[test]
fn every_optimization_on_is_all_and_every_one_off_is_none() {
    let mut options = OptimizationOptions::NONE;
    for &optimization in Optimization::ALL {
        options.set(optimization, true);
    }
    assert_eq!(options, OptimizationOptions::ALL);
    for &optimization in Optimization::ALL {
        options.set(optimization, false);
    }
    assert_eq!(options, OptimizationOptions::NONE);
}

#[test]
fn get_reads_what_set_wrote() {
    for &optimization in Optimization::ALL {
        let mut options = OptimizationOptions::ALL;
        assert!(options.get(optimization), "{optimization:?}");
        options.set(optimization, false);
        assert!(!options.get(optimization), "{optimization:?}");
        // The others are untouched.
        let others_on = Optimization::ALL
            .iter()
            .filter(|&&other| other != optimization)
            .all(|&other| options.get(other));
        assert!(others_on, "{optimization:?}");
    }
}

#[test]
fn with_is_set_on_a_copy() {
    for &optimization in Optimization::ALL {
        for on in [true, false] {
            for base in [OptimizationOptions::NONE, OptimizationOptions::ALL] {
                let mut expected = base;
                expected.set(optimization, on);
                assert_eq!(
                    base.with(optimization, on),
                    expected,
                    "{optimization:?} set to {on} from {base:?}"
                );
            }
        }
    }
}

#[test]
fn with_chains() {
    let options = OptimizationOptions::NONE
        .with(Optimization::ConstantFold, true)
        .with(Optimization::ConsumeLocals, true)
        .with(Optimization::ConstantFold, false);
    let on: Vec<Optimization> = Optimization::ALL
        .iter()
        .copied()
        .filter(|&optimization| options.get(optimization))
        .collect();
    assert_eq!(on, [Optimization::ConsumeLocals], "a later call wins");
}

#[test]
fn names_are_kebab_case_and_distinct() {
    let names: Vec<&str> = Optimization::ALL.iter().map(|o| o.name()).collect();
    assert_eq!(
        names,
        [
            "constant-fold",
            "constant-propagate",
            "branch-eliminate",
            "capture-hoist",
            "dead-store-eliminate",
            "discard-eliminate",
            "consume-locals",
            "deduplicate-constants",
        ]
    );
    assert_eq!(names.iter().collect::<HashSet<_>>().len(), names.len());
}

#[test]
fn from_name_finds_each_optimization_by_its_name() {
    for &optimization in Optimization::ALL {
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

// --- Settings ---

/// `options` with `settings` applied, which must all be valid.
fn after(options: OptimizationOptions, settings: &str) -> OptimizationOptions {
    options
        .with_settings(settings)
        .unwrap_or_else(|invalid| panic!("{settings:?}: {invalid}"))
}

/// `OptimizationOptions::NONE` with only `on` turned on.
fn only(on: &[Optimization]) -> OptimizationOptions {
    let mut options = OptimizationOptions::NONE;
    for &optimization in on {
        options.set(optimization, true);
    }
    options
}

#[test]
fn a_setting_turns_one_optimization_on_or_off() {
    for &optimization in Optimization::ALL {
        let name = optimization.name();
        assert_eq!(
            after(OptimizationOptions::NONE, &format!("{name}=true")),
            only(&[optimization]),
            "{name}"
        );
        let mut expected = OptimizationOptions::ALL;
        expected.set(optimization, false);
        assert_eq!(
            after(OptimizationOptions::ALL, &format!("{name}=false")),
            expected,
            "{name}"
        );
    }
}

#[test]
fn a_preset_replaces_every_optimization() {
    let some = only(&[Optimization::CaptureHoist]);
    for (settings, expected) in [
        ("all", OptimizationOptions::ALL),
        ("none", OptimizationOptions::NONE),
        ("preset=all", OptimizationOptions::ALL),
        ("preset=none", OptimizationOptions::NONE),
    ] {
        assert_eq!(after(some, settings), expected, "{settings:?}");
    }
}

#[test]
fn settings_apply_left_to_right() {
    assert_eq!(
        after(
            OptimizationOptions::ALL,
            "none,constant-fold=true,branch-eliminate=true"
        ),
        only(&[Optimization::ConstantFold, Optimization::BranchEliminate])
    );
    assert_eq!(
        after(OptimizationOptions::NONE, "constant-fold=true,none"),
        OptimizationOptions::NONE,
        "a later preset overrides an earlier setting"
    );
    assert_eq!(
        after(
            OptimizationOptions::NONE,
            "constant-fold=true,constant-fold=false"
        ),
        OptimizationOptions::NONE,
        "a later setting overrides an earlier one"
    );
}

#[test]
fn spaces_around_each_part_are_ignored() {
    assert_eq!(
        after(
            OptimizationOptions::ALL,
            "  none ,  constant-fold =  true , preset= none ,all "
        ),
        OptimizationOptions::ALL
    );
    assert_eq!(
        after(OptimizationOptions::ALL, " none , consume-locals = true "),
        only(&[Optimization::ConsumeLocals])
    );
}

#[test]
fn an_invalid_setting_is_refused_with_its_reason() {
    use InvalidOptimizationSetting::*;
    for (settings, expected) in [
        ("constant-fold", Malformed("constant-fold".to_string())),
        ("", Malformed(String::new())),
        ("none,", Malformed(String::new())),
        ("some", Malformed("some".to_string())),
        (
            "constant_fold=true",
            UnknownOptimization("constant_fold".to_string()),
        ),
        ("=true", UnknownOptimization(String::new())),
        (
            "constant-fold=yes",
            InvalidValue {
                optimization: Optimization::ConstantFold,
                value: "yes".to_string(),
            },
        ),
        (
            "consume-locals=",
            InvalidValue {
                optimization: Optimization::ConsumeLocals,
                value: String::new(),
            },
        ),
        ("preset=some", UnknownPreset("some".to_string())),
        ("preset=true", UnknownPreset("true".to_string())),
    ] {
        assert_eq!(
            OptimizationOptions::ALL.with_settings(settings),
            Err(expected),
            "{settings:?}"
        );
    }
}

#[test]
fn a_refusal_explains_itself() {
    use InvalidOptimizationSetting::*;
    let cases = [
        (
            Malformed("x".to_string()),
            "`x` should be `<optimization>=true|false`, `preset=all|none`, `all`, or `none`",
        ),
        (
            UnknownOptimization("x".to_string()),
            "there is no optimization `x`; there are constant-fold, constant-propagate, \
             branch-eliminate, capture-hoist, dead-store-eliminate, discard-eliminate, \
             consume-locals, deduplicate-constants",
        ),
        (
            InvalidValue {
                optimization: Optimization::CaptureHoist,
                value: "x".to_string(),
            },
            "`capture-hoist` is `true` or `false`, not `x`",
        ),
        (
            UnknownPreset("x".to_string()),
            "`preset` is `all` or `none`, not `x`",
        ),
    ];
    for (invalid, expected) in cases {
        assert_eq!(invalid.to_string(), expected);
    }
}
