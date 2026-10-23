//! Unreachable patterns are compile errors.
//!
//! `unreachable match arm`: an arm after the first arm that matches every value.
//! An arm matches every value when it has no guard and its pattern cannot fail:
//! a name or `_` with no `is` constraint, or an alternative with such a branch.
//! Every other pattern can fail.
//!
//! `unreachable alternative branch`: a branch of an alternative, in any pattern,
//! after a branch that cannot fail.
//!
//! Only the first unreachable arm or branch is reported, after any error before
//! it in the source. The error is the same under every optimization
//! permutation, which `compile_errors` checks, and with lints off.

use crate::script;

use frostlang::Value;
use frostlang::compile::{CompilerOptions, Diagnostics, compile_program};
use script::{Script, compile_errors, label_line, underlined};

const ARM: &str = "unreachable match arm";
const BRANCH: &str = "unreachable alternative branch";

/// Assert each `source` is rejected for `code`, with exactly one error.
fn assert_rejected(code: &str, sources: &[&str]) {
    for source in sources {
        let errors = compile_errors(source);
        let narrated = errors.render_narrated();
        assert_eq!(errors.len(), 1, "{source:?} has one error:\n{narrated}");
        assert!(
            narrated.contains(code),
            "{source:?} is rejected for {code:?}:\n{narrated}"
        );
    }
}

/// Assert each `source` compiles under every optimization permutation.
fn assert_compiles(sources: &[&str]) {
    for source in sources {
        for optimization in script::optimization_permutations() {
            let options = CompilerOptions::new().with_optimization(optimization);
            if let Err(errors) = compile_program("test.frst", source, options) {
                panic!(
                    "{source:?} compiles under {optimization:?}:\n{}",
                    errors.render_plain()
                );
            }
        }
    }
}

/// `arms` in a match on `1`.
fn matching(arms: &str) -> String {
    format!("match 1 {{\n{arms}\n}}")
}

/// The narrated text of the one error of `source`.
fn narrated(source: &str) -> String {
    let errors: Diagnostics = compile_errors(source);
    let narrated = errors.render_narrated();
    assert_eq!(errors.len(), 1, "{source:?} has one error:\n{narrated}");
    narrated
}

// --- Match arms ---

#[test]
fn an_arm_after_one_that_matches_every_value_is_rejected() {
    let cases = [
        "x => 0,\n2 => 0,",
        "_ => 0,\n2 => 0,",
        "_ => 0,\n_ => 0,",
        "x => 0,\ny => 0,",
        // Unguarded after guarded.
        "x if: x > 0 => 0,\n_ => 0,\n2 => 0,",
        // An alternative with a branch that cannot fail, last so as not to
        // leave a branch unreachable.
        "1 | _ => 0,\n2 => 0,",
        "1 | 2 | _ => 0,\n3 => 0,",
        "[x] | x => 0,\n3 => 0,",
        "[x] | [x, _] | x => 0,\n3 => 0,",
    ]
    .map(matching);
    assert_rejected(ARM, &cases.each_ref().map(String::as_str));
}

#[test]
fn an_arm_after_arms_that_can_fail_compiles() {
    let mut cases = vec![
        // The catch-all last, or alone.
        "2 => 0,\n_ => 0,",
        "2 => 0,\n3 => 0,\nx => 0,",
        "_ => 0,",
        "x => 0,",
        // No catch-all.
        "1 => 0,\n2 => 0,\n3 => 0,",
        "n is Int => 0,\n[a, b] => 0,\n{k} => 0,",
        // Patterns that can fail.
        "'s' => 0,\n2 => 0,",
        "null => 0,\n2 => 0,",
        "true => 0,\n2 => 0,",
        "1.5 => 0,\n2 => 0,",
        "[a, b] => 0,\n2 => 0,",
        "[] => 0,\n2 => 0,",
        "[...rest] => 0,\n2 => 0,",
        "[a, ...rest] => 0,\n2 => 0,",
        "{k} => 0,\n2 => 0,",
        "{} => 0,\n2 => 0,",
        "{k: v} => 0,\n2 => 0,",
        "{k} as whole => 0,\n2 => 0,",
        "_ is Nonnull => 0,\n2 => 0,",
        // Guarded catch-alls.
        "x if: x > 0 => 0,\n2 => 0,",
        "_ if: false => 0,\n2 => 0,",
        "_ if: true => 0,\n2 => 0,",
        "1 | _ if: false => 0,\n2 => 0,",
        "[x] | x if: x > 0 => 0,\n2 => 0,",
        // Alternatives of branches that can fail.
        "1 | 2 => 0,\n3 => 0,",
        "x is Int | x is Float => 0,\n3 => 0,",
        "[a] | [a, _] => 0,\n3 => 0,",
        "1 | _ is Nonnull => 0,\n3 => 0,",
        "[x] | x is Int => 0,\n3 => 0,",
        // An alternative nested in a structure leaves the structure able to fail.
        "[1 | _] => 0,\n2 => 0,",
        "{k: 1 | _} => 0,\n2 => 0,",
    ]
    .into_iter()
    .map(matching)
    .collect::<Vec<_>>();
    cases.push(
        r"
        def y = 1
        match 1 {
            (y) => 0,
            2 => 0,
        }
        "
        .to_owned(),
    );
    for constraint in [
        "Null",
        "Int",
        "Float",
        "Bool",
        "String",
        "Bytes",
        "Array",
        "Map",
        "Function",
        "Opaque",
        "Primitive",
        "Numeric",
        "Structured",
        "Flat",
        "Nonnull",
    ] {
        cases.push(matching(&format!("n is {constraint} => 0,\n2 => 0,")));
    }
    assert_compiles(&cases.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn only_the_first_unreachable_arm_is_reported() {
    let source = r"
        match 1 {
            0 => 'zero',
            _ => 'any',
            1 => 'one',
            x => 'name',
        }
    ";
    assert_eq!(label_line(&narrated(source), "never tried"), 5);
}

#[test]
fn an_error_before_the_unreachable_arm_is_reported_instead() {
    let errors = compile_errors(
        r"
        match 1 {
            2 => nope,
            _ => 1,
            3 => 0,
        }
        ",
    )
    .render_plain();
    assert!(errors.contains("`nope` is not defined"), "{errors}");
    assert!(!errors.contains(ARM), "{errors}");
}

#[test]
fn the_unreachable_arm_itself_is_not_compiled() {
    // Its unbound name and its raising value are never reached.
    assert_rejected(
        ARM,
        &[
            "match 1 { _ => 1, 2 => nope }",
            "match 1 { _ => 1, (1 / 0) => 0 }",
        ],
    );
}

#[test]
fn the_error_names_the_code_the_message_the_labels_and_the_help() {
    let text = narrated(
        r"
        match 1 {
            x => 0,
            2 => 0,
        }
        ",
    );
    for expected in [
        ARM,
        "severity: error",
        "this match arm is never reached",
        "never tried",
        "an earlier match arm matches every value",
        "matches every value",
        "remove the match arm, or move it before the one that matches every value",
    ] {
        assert!(text.contains(expected), "mentions {expected:?}:\n{text}");
    }
}

#[test]
fn the_labels_cover_exactly_the_unreachable_pattern_and_the_catch_all_pattern() {
    let source = r"
        match 1 {
            whole_thing => 0,
            [first, ...rest] => 0,
        }
    ";
    let plain = compile_errors(source).render_plain();
    assert_eq!(
        underlined(&plain, "            [first, ...rest] => 0,"),
        "[first, ...rest]",
        "the unreachable pattern, not the whole arm:\n{plain}"
    );
    assert_eq!(
        underlined(&plain, "            whole_thing => 0,"),
        "whole_thing",
        "the catch-all pattern, not the whole arm:\n{plain}"
    );
    let text = narrated(source);
    assert_eq!(label_line(&text, "never tried"), 4, "in:\n{text}");
    assert_eq!(label_line(&text, "matches every value"), 3, "in:\n{text}");
    assert!(
        text.contains("columns 13 to 28: never tried"),
        "the label spans the pattern, after the indent:\n{text}"
    );
}

#[test]
fn an_alternative_catch_all_is_labeled_across_the_whole_alternative() {
    let plain = compile_errors(
        r"
        match 1 {
            1 | _ => 0,
            2 => 0,
        }
        ",
    )
    .render_plain();
    assert_eq!(
        underlined(&plain, "            1 | _ => 0,"),
        "1 | _",
        "the whole alternative:\n{plain}"
    );
}

/// Assert the narrated `text` gives `hint` as help on the advice that points at
/// the catch-all, which follows the error itself, and not on the error.
fn assert_hint_on_the_catch_all(text: &str, hint: &str) {
    let advice = text
        .find("an earlier ")
        .unwrap_or_else(|| panic!("no advice in:\n{text}"));
    assert!(
        text[advice..].contains(hint),
        "the advice gives {hint:?}:\n{text}"
    );
    assert!(
        !text[..advice].contains(hint),
        "the error itself does not give {hint:?}:\n{text}"
    );
}

#[test]
fn a_catch_all_naming_a_bound_name_hints_at_pinning_it() {
    // The trap: `limit` reads as a comparison against the outer `limit`.
    let bound = narrated(
        r"
        def limit = 10
        match 5 {
            limit => 'at the limit',
            _ => 'other',
        }
        ",
    );
    assert_hint_on_the_catch_all(
        &bound,
        "to compare against an existing `limit`, write `(limit)`",
    );
    let global = narrated(
        r"
        match 5 {
            print => 0,
            _ => 1,
        }
        ",
    );
    assert_hint_on_the_catch_all(
        &global,
        "to compare against an existing `print`, write `(print)`",
    );
    // No hint for `_`, or for a name not bound before the pattern, which in an
    // alternative is the catch-all branch binding the name itself.
    for source in [
        r"
        match 5 {
            fresh => 0,
            _ => 1,
        }
        ",
        r"
        match 5 {
            _ => 0,
            2 => 1,
        }
        ",
        r"
        match [5] {
            x | [x] => x,
        }
        ",
        r"
        match [5] {
            [x | [x]] => x,
        }
        ",
        "def x | [x] = [1]",
    ] {
        let text = narrated(source);
        assert!(
            !text.contains("to compare against"),
            "no hint for `_` or a name not bound before:\n{text}"
        );
    }
}

#[test]
fn a_catch_all_naming_a_type_hints_at_testing_for_it() {
    // A bare type name binds every value, as any name does.
    let arm = narrated(
        r"
        match 'a' {
            String => 1,
            _ => 2,
        }
        ",
    );
    assert_hint_on_the_catch_all(&arm, "to test for the type, write `_ is String`");
    let branch = narrated(
        r"
        match ['a'] {
            String | [String] => String,
        }
        ",
    );
    assert_hint_on_the_catch_all(&branch, "to test for the type, write `_ is String`");
}

#[test]
fn an_unreachable_arm_is_rejected_wherever_its_match_is() {
    assert_rejected(
        ARM,
        &[
            // In an arm's result.
            r"
            match 1 {
                1 => match 2 {
                    _ => 'a',
                    3 => 'b',
                },
                _ => 'c',
            }
            ",
            // In a do block.
            r"
            do {
                def v = 5
                match v {
                    _ => 1,
                    5 => 2,
                }
            }
            ",
            // In a target and in a guard.
            r"
            match (match 1 { _ => 2, 3 => 4 }) {
                2 => 'two',
                _ => 'other',
            }
            ",
            r"
            match 1 {
                x if: (match x { _ => true, 2 => false }) => 'a',
                _ => 'b',
            }
            ",
            // In lambdas of each kind.
            r"
            def f = fn x -> match x {
                _ => 1,
                2 => 2,
            }
            f(2)
            ",
            r"
            defn f(x) -> match x {
                n => n,
                2 => 2,
            }
            f(2)
            ",
            r"
            def f = fn x -> fn y -> match y {
                _ => x,
                2 => 0,
            }
            f(1)(2)
            ",
            r"
            def f = $(match $ {
                _ => 1,
                2 => 2,
            })
            f(2)
            ",
            r"
            map([1, 2], fn x -> match x {
                _ => 1,
                2 => 2,
            })
            ",
        ],
    );
}

#[test]
fn a_match_on_a_constant_is_still_rejected() {
    // Branch elimination could drop the unreachable arm; the error is about the
    // source, not the emitted code.
    assert_rejected(
        ARM,
        &[r"
        match 3 {
            3 => 'three',
            _ => 'any',
            4 => 'four',
        }
        "],
    );
}

#[test]
fn the_error_does_not_depend_on_lints() {
    let source = "match 1 { _ => 0, 2 => 0 }";
    let errors = compile_program(
        "test.frst",
        source,
        CompilerOptions::new().with_lints(false),
    )
    .expect_err("rejected with lints off");
    let plain = errors.render_plain();
    assert!(plain.contains(ARM), "rejected for {ARM:?}:\n{plain}");
}

#[test]
fn an_alternative_with_a_catch_all_branch_last_matches_every_value_at_run_time() {
    let source = r"
        match 7 {
            [x] | x => x,
        }
    ";
    assert_eq!(Script::new(source).run(), Value::Int(7));
}

// --- Alternative branches ---

#[test]
fn a_branch_after_one_that_cannot_fail_is_rejected() {
    assert_rejected(
        BRANCH,
        &[
            "match [5] { x | [x] => x }",
            "match 5 { x | x => x }",
            "match 5 { _ | 1 => 0 }",
            "match 5 { 1 | _ | 2 => 0 }",
            // Nested in a structure.
            "match [5] { [x | [x]] => x }",
            "match {k: 5} { {k: _ | 1} => 0 }",
            // In a `def` pattern.
            "def x | [x] = [1]",
            "def [x | [x]] = [[1]]",
        ],
    );
}

#[test]
fn a_branch_after_branches_that_can_fail_compiles() {
    assert_compiles(&[
        "match 5 { x is Int | x => x }",
        "match [5] { [x] | x => x }",
        "match 5 { 1 | _ => 0 }",
        "match 5 { 1 | 2 => 0, _ => 1 }",
        r"
        def [x] | x = [1]
        x
        ",
        r"
        def x is Int | x = 1
        x
        ",
    ]);
}

#[test]
fn only_the_first_unreachable_branch_is_reported() {
    let text = narrated(
        r"
        match 5 {
            1 | _ | 2 | 3 => 0,
        }
        ",
    );
    assert!(text.contains(BRANCH), "{text}");
    assert!(
        text.contains("label at line 3, column 21: never tried"),
        "the `2`:\n{text}"
    );
}

#[test]
fn a_catch_all_arm_holding_an_unreachable_branch_reports_the_branch() {
    // The arm's own pattern comes first in the source, so its unreachable
    // branch is reported, not the arm after it.
    let text = narrated(
        r"
        match 1 {
            x | [x] => 0,
            2 => 0,
        }
        ",
    );
    assert!(text.contains(BRANCH), "{text}");
    assert!(!text.contains(ARM), "{text}");
}

#[test]
fn an_error_before_the_unreachable_branch_is_reported_instead() {
    let errors = compile_errors("match 5 { (nope) | _ | 1 => 0 }").render_plain();
    assert!(errors.contains("`nope` is not defined"), "{errors}");
    assert!(!errors.contains(BRANCH), "{errors}");
}

#[test]
fn the_branch_error_names_the_code_the_message_the_labels_and_the_help() {
    let source = r"
        def x = 1
        match [5] {
            x | [x] => x,
        }
    ";
    let text = narrated(source);
    for expected in [
        BRANCH,
        "severity: error",
        "this alternative branch is never reached",
        "never tried",
        "an earlier alternative branch matches every value",
        "matches every value",
        "remove the alternative branch, or move it before the one that matches every value",
    ] {
        assert!(text.contains(expected), "mentions {expected:?}:\n{text}");
    }
    assert_hint_on_the_catch_all(&text, "to compare against an existing `x`, write `(x)`");
    assert!(
        text.contains("label at line 4, column 13: matches every value"),
        "the catch-all label covers the `x` branch alone:\n{text}"
    );
    let plain = compile_errors(source).render_plain();
    assert_eq!(
        underlined(&plain, "            x | [x] => x,"),
        "[x]",
        "{plain}"
    );
}
