//! Lints: nonfatal diagnostics in `CompilerOutput::lints`.
//!
//! The one lint so far is `unreachable match arm`: every arm after the first
//! arm that matches everything. An arm matches everything when it has no guard
//! and its pattern cannot fail: a name or `_` with no `is` constraint, or an
//! alternative with such a branch. Every other pattern can fail.
//!
//! A lint never changes what a program does, so the cases that run a program
//! check its result too.

use crate::script;

use frostlang::Value;
use frostlang::compile::{
    CompilerOptions, CompilerOutput, Diagnostic, Diagnostics, compile_in_scope, compile_program,
};
use script::Script;

const FILENAME: &str = "lints.frst";
const CODE: &str = "unreachable match arm";

fn compile_with(source: &str, options: CompilerOptions) -> CompilerOutput {
    compile_program(FILENAME, source, options)
        .unwrap_or_else(|errors| panic!("{source:?} should compile:\n{}", errors.render_plain()))
}

/// The lints of `source`, compiled with the default options.
fn lints(source: &str) -> Diagnostics {
    compile_with(source, CompilerOptions::new()).lints
}

/// The narrated text of each lint of `source`, in order.
fn narrated(source: &str) -> Vec<String> {
    lints(source)
        .iter()
        .map(Diagnostic::render_narrated)
        .collect()
}

/// Assert `source` has exactly `count` lints, each an unreachable match arm.
fn assert_unreachable_arms(source: &str, count: usize) {
    let found = narrated(source);
    assert_eq!(
        found.len(),
        count,
        "{source:?} should have {count} lint(s), found:\n{}",
        found.join("\n")
    );
    for text in &found {
        assert!(
            text.contains(CODE),
            "{source:?}: a lint should name its code {CODE:?}:\n{text}"
        );
    }
}

/// Assert each `(arms, count)` case, with the arms in a match on `1`, lints
/// `count` times. The result of each arm is `0`.
fn assert_arm_counts(cases: &[(&str, usize)]) {
    for (arms, count) in cases {
        let source = format!("match 1 {{\n{arms}\n}}");
        assert_unreachable_arms(&source, *count);
    }
}

#[test]
fn a_name_arm_first_makes_the_next_arm_unreachable() {
    assert_arm_counts(&[("x => 0,\n2 => 0,", 1)]);
}

#[test]
fn an_underscore_arm_first_makes_the_next_arm_unreachable() {
    assert_arm_counts(&[("_ => 0,\n2 => 0,", 1)]);
}

#[test]
fn a_catch_all_as_the_last_arm_is_not_linted() {
    assert_arm_counts(&[
        ("2 => 0,\n_ => 0,", 0),
        ("2 => 0,\n3 => 0,\nx => 0,", 0),
        ("_ => 0,", 0),
        ("x => 0,", 0),
    ]);
}

#[test]
fn a_match_with_no_catch_all_is_not_linted() {
    assert_arm_counts(&[
        ("1 => 0,\n2 => 0,\n3 => 0,", 0),
        ("n is Int => 0,\n[a, b] => 0,\n{k} => 0,", 0),
    ]);
}

#[test]
fn a_single_arm_match_is_not_linted() {
    assert_arm_counts(&[("1 => 0,", 0), ("_ => 0,", 0)]);
}

#[test]
fn each_arm_after_the_catch_all_is_linted_once_in_order() {
    let source = r"
        match 1 {
            0 => 'zero',
            _ => 'any',
            1 => 'one',
            2 => 'two',
            x => 'name',
            [a] => 'array',
        }
    ";
    assert_eq!(
        never_tried_lines(source),
        [5, 6, 7, 8],
        "one lint per arm after the catch-all, in source order"
    );
}

/// The source line of each lint's "never tried" label, in report order, read
/// from the narrated text.
fn never_tried_lines(source: &str) -> Vec<usize> {
    narrated(source)
        .iter()
        .map(|text| label_line(text, "never tried"))
        .collect()
}

/// The line of the label `label` in the narrated `text`, which has a line like
/// `label at line 3, column 5: never tried`.
fn label_line(text: &str, label: &str) -> usize {
    let prefix = "label at line ";
    text.lines()
        .filter(|line| line.trim_end().ends_with(&format!(": {label}")))
        .find_map(|line| {
            let rest = line.trim().strip_prefix(prefix)?;
            rest.split(',').next()?.parse().ok()
        })
        .unwrap_or_else(|| panic!("no label {label:?} in:\n{text}"))
}

#[test]
fn a_second_catch_all_is_itself_unreachable() {
    assert_arm_counts(&[("_ => 0,\n_ => 0,", 1), ("x => 0,\ny => 0,\nz => 0,", 2)]);
}

#[test]
fn patterns_that_can_fail_are_not_catch_alls() {
    // Each first arm can fail, so the value arm after it is reachable.
    assert_arm_counts(&[
        ("1 => 0,\n2 => 0,", 0),
        ("'s' => 0,\n2 => 0,", 0),
        ("null => 0,\n2 => 0,", 0),
        ("true => 0,\n2 => 0,", 0),
        ("1.5 => 0,\n2 => 0,", 0),
        ("[a, b] => 0,\n2 => 0,", 0),
        ("[] => 0,\n2 => 0,", 0),
        ("[...rest] => 0,\n2 => 0,", 0),
        ("[a, ...rest] => 0,\n2 => 0,", 0),
        ("{k} => 0,\n2 => 0,", 0),
        ("{} => 0,\n2 => 0,", 0),
        ("{k: v} => 0,\n2 => 0,", 0),
        ("{k} as whole => 0,\n2 => 0,", 0),
    ]);
}

#[test]
fn a_parenthesized_value_pattern_is_not_a_catch_all() {
    let source = r"
        def y = 1
        match 1 {
            (y) => 0,
            2 => 0,
        }
    ";
    assert_unreachable_arms(source, 0);
}

#[test]
fn any_type_constraint_stops_a_name_from_being_a_catch_all() {
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
        let source = format!("match 1 {{\nn is {constraint} => 0,\n2 => 0,\n}}");
        assert_unreachable_arms(&source, 0);
    }
}

#[test]
fn an_underscore_with_a_type_constraint_is_not_a_catch_all() {
    assert_arm_counts(&[
        ("_ is Nonnull => 0,\n2 => 0,", 0),
        ("_ is Int => 0,\n2 => 0,", 0),
    ]);
}

#[test]
fn a_guarded_catch_all_does_not_count() {
    assert_arm_counts(&[
        ("x if: x > 0 => 0,\n2 => 0,", 0),
        ("_ if: false => 0,\n2 => 0,", 0),
        ("_ if: true => 0,\n2 => 0,", 0),
    ]);
}

#[test]
fn an_unguarded_catch_all_after_a_guarded_one_makes_later_arms_unreachable() {
    assert_arm_counts(&[("x if: x > 0 => 0,\n_ => 0,\n2 => 0,", 1)]);
}

#[test]
fn an_alternative_with_a_catch_all_branch_is_a_catch_all() {
    // Every alternative must bind the same names, so a catch-all branch that
    // binds a name needs the others to bind it too.
    assert_arm_counts(&[
        ("1 | _ => 0,\n2 => 0,", 1),
        ("_ | 1 => 0,\n2 => 0,", 1),
        ("1 | 2 | _ => 0,\n3 => 0,", 1),
        ("1 | _ | 2 => 0,\n3 => 0,", 1),
        ("[x] | x => 0,\n3 => 0,", 1),
        ("x | [x] => 0,\n3 => 0,", 1),
        ("x | x => 0,\n3 => 0,", 1),
        ("[x] | [x, _] | x => 0,\n3 => 0,", 1),
    ]);
}

#[test]
fn an_alternative_of_failing_branches_is_not_a_catch_all() {
    assert_arm_counts(&[
        ("1 | 2 => 0,\n3 => 0,", 0),
        ("x is Int | x is Float => 0,\n3 => 0,", 0),
        ("[a] | [a, _] => 0,\n3 => 0,", 0),
        ("1 | _ is Nonnull => 0,\n3 => 0,", 0),
        ("[x] | x is Int => 0,\n3 => 0,", 0),
    ]);
}

#[test]
fn a_guarded_alternative_with_a_catch_all_branch_is_not_a_catch_all() {
    assert_arm_counts(&[
        ("1 | _ if: false => 0,\n2 => 0,", 0),
        ("[x] | x if: x > 0 => 0,\n2 => 0,", 0),
    ]);
}

#[test]
fn an_alternative_nested_in_an_array_or_map_does_not_make_it_a_catch_all() {
    assert_arm_counts(&[
        ("[1 | _] => 0,\n2 => 0,", 0),
        ("{k: 1 | _} => 0,\n2 => 0,", 0),
    ]);
}

#[test]
fn an_alternative_with_a_catch_all_branch_matches_every_value_at_run_time() {
    let source = r"
        match 7 {
            [x] | x => x,
            7 => 0,
        }
    ";
    assert_unreachable_arms(source, 1);
    assert_eq!(Script::new(source).run(), Value::Int(7));
}

#[test]
fn the_lint_names_the_code_the_message_the_labels_and_the_help() {
    let source = r"
match 1 {
    x => 0,
    2 => 0,
}";
    let found = narrated(source);
    assert_eq!(found.len(), 1);
    let text = &found[0];
    for expected in [
        CODE,
        "this match arm is never reached",
        "never tried",
        "an earlier arm matches every value",
        "matches every value",
        "remove the arm, or move it before the one that matches everything",
        FILENAME,
    ] {
        assert!(
            text.contains(expected),
            "the lint should mention {expected:?}:\n{text}"
        );
    }
}

#[test]
fn the_plain_rendering_shows_both_labels_and_the_help() {
    let source = r"
match 1 {
    x => 0,
    2 => 0,
}";
    let output = compile_with(source, CompilerOptions::new());
    let text = output.lints.render_plain();
    for expected in [
        CODE,
        "never tried",
        "matches every value",
        "help: remove the arm",
    ] {
        assert!(
            text.contains(expected),
            "the plain rendering should mention {expected:?}:\n{text}"
        );
    }
}

#[test]
fn the_lint_is_a_warning_not_an_error() {
    // It sits in the output of a successful compile; it is not the error
    // channel of a failed one.
    let output = compile_with("match 1 { _ => 0, 2 => 0 }", CompilerOptions::new());
    assert_eq!(output.lints.len(), 1);
    assert!(
        output.lints.render_narrated().contains("severity: warning"),
        "the lint is a warning:\n{}",
        output.lints.render_narrated()
    );
}

/// The text a plain rendering underlines on the source line `line`: what
/// sits between the first and last mark of the marker row under it.
fn underlined(plain: &str, line: &str) -> String {
    let rows: Vec<&str> = plain.lines().collect();
    rows.windows(2)
        .find_map(|pair| {
            let (_, shown) = pair[0].split_once("| ")?;
            let (_, marks) = pair[1].split_once(": ")?;
            if shown != line {
                return None;
            }
            let start = marks.find(|c| c != ' ')?;
            let end = marks.rfind(|c| c != ' ')?;
            shown.get(start..=end).map(str::to_owned)
        })
        .unwrap_or_else(|| panic!("no underlined {line:?} in:\n{plain}"))
}

#[test]
fn the_primary_label_covers_exactly_the_unreachable_arms_pattern() {
    let source = "match 1 {\n    x => 0,\n    [first, ...rest] => 0,\n}";
    let plain = lints(source).render_plain();
    assert_eq!(
        underlined(&plain, "    [first, ...rest] => 0,"),
        "[first, ...rest]",
        "the pattern, not the whole arm:\n{plain}"
    );
}

#[test]
fn the_related_label_covers_exactly_the_catch_alls_pattern() {
    let source = "match 1 {\n    whole_thing => 0,\n    2 => 0,\n}";
    let plain = lints(source).render_plain();
    assert_eq!(
        underlined(&plain, "    whole_thing => 0,"),
        "whole_thing",
        "the pattern, not the whole arm:\n{plain}"
    );
}

#[test]
fn the_labels_point_at_the_unreachable_pattern_and_the_catch_all_pattern() {
    let source = "match 1 {\n    x => 0,\n    2 => 0,\n}";
    let text = narrated(source).remove(0);
    assert_eq!(label_line(&text, "never tried"), 3, "in:\n{text}");
    assert_eq!(label_line(&text, "matches every value"), 2, "in:\n{text}");
    assert!(
        text.contains("column 5: never tried"),
        "the label starts at the pattern, after the indent:\n{text}"
    );
}

#[test]
fn an_alternative_catch_all_is_labeled_across_the_whole_alternative() {
    let source = "match 1 {\n    1 | _ => 0,\n    2 => 0,\n}";
    let plain = lints(source).render_plain();
    assert_eq!(
        underlined(&plain, "    1 | _ => 0,"),
        "1 | _",
        "the whole alternative:\n{plain}"
    );
}

#[test]
fn lints_in_a_nested_match_in_an_arm_result_are_reported() {
    let source = r"
        match 1 {
            1 => match 2 {
                _ => 'a',
                3 => 'b',
            },
            _ => 'c',
        }
    ";
    assert_unreachable_arms(source, 1);
}

#[test]
fn an_outer_and_an_inner_match_each_lint_in_source_order() {
    let source = r"
        match 1 {
            1 => match 2 {
                _ => 'a',
                3 => 'inner',
            },
            _ => 'c',
            4 => 'outer',
        }
    ";
    let found = narrated(source);
    assert_eq!(found.len(), 2, "one inner and one outer lint");
    assert!(
        found[0].contains("3 => 'inner'"),
        "the inner arm, earlier in the source, comes first:\n{}",
        found[0]
    );
    assert!(
        found[1].contains("4 => 'outer'"),
        "the outer arm comes second:\n{}",
        found[1]
    );
}

#[test]
fn a_match_in_a_do_block_is_linted() {
    let source = r"
        def r = do {
            def v = 5
            match v {
                _ => 1,
                5 => 2,
            }
        }
        r
    ";
    assert_unreachable_arms(source, 1);
}

#[test]
fn a_match_in_a_guard_or_a_target_is_linted() {
    let in_target = r"
        match (match 1 { _ => 2, 3 => 4 }) {
            2 => 'two',
            _ => 'other',
        }
    ";
    assert_unreachable_arms(in_target, 1);
    let in_guard = r"
        match 1 {
            x if: (match x { _ => true, 2 => false }) => 'a',
            _ => 'b',
        }
    ";
    assert_unreachable_arms(in_guard, 1);
}

#[test]
fn a_match_in_a_lambda_is_linted() {
    let source = r"
        def f = fn x -> match x {
            _ => 1,
            2 => 2,
        }
        f(2)
    ";
    assert_unreachable_arms(source, 1);
}

#[test]
fn a_match_in_a_defn_is_linted() {
    let source = r"
        defn f(x) -> match x {
            n => n,
            2 => 2,
        }
        f(2)
    ";
    assert_unreachable_arms(source, 1);
}

#[test]
fn a_match_in_a_curried_lambda_is_linted() {
    let source = r"
        def f = fn x -> fn y -> match y {
            _ => x,
            2 => 0,
        }
        f(1)(2)
    ";
    assert_unreachable_arms(source, 1);
}

#[test]
fn a_match_in_an_abbreviated_lambda_is_linted() {
    let source = r"
        def f = $(match $ {
            _ => 1,
            2 => 2,
        })
        f(2)
    ";
    assert_unreachable_arms(source, 1);
}

#[test]
fn a_match_in_a_lambda_passed_to_map_is_linted() {
    let source = r"
        map [1, 2] with fn x -> match x {
            _ => 1,
            2 => 2,
        }
    ";
    assert_unreachable_arms(source, 1);
}

#[test]
fn lints_before_in_and_after_a_lambda_are_in_source_order() {
    let source = r"
        def before = match 1 {
            _ => 'a',
            1 => 'before',
        }
        def f = fn x -> match x {
            _ => 'b',
            2 => 'inside',
        }
        def after = match 1 {
            _ => 'c',
            3 => 'after',
        }
        [before, f, after]
    ";
    let found = narrated(source);
    assert_eq!(found.len(), 3, "one lint per match");
    for (text, arm) in found.iter().zip(["'before'", "'inside'", "'after'"]) {
        assert!(
            text.contains(arm),
            "the lints are in source order; expected {arm} in:\n{text}"
        );
    }
}

#[test]
fn lints_in_sibling_lambdas_are_in_source_order() {
    let source = r"
        def f = fn x -> match x {
            _ => 'b',
            2 => 'first',
        }
        def g = fn x -> match x {
            _ => 'b',
            3 => 'second',
        }
        [f, g]
    ";
    let found = narrated(source);
    assert_eq!(found.len(), 2);
    assert!(found[0].contains("'first'"), "{}", found[0]);
    assert!(found[1].contains("'second'"), "{}", found[1]);
}

#[test]
fn a_lint_does_not_change_what_the_match_does() {
    // The unreachable arms are not run, and the catch-all's result is the value.
    let source = r"
        match 7 {
            n => n + 1,
            7 => 0,
            _ => -1,
        }
    ";
    assert_unreachable_arms(source, 2);
    assert_eq!(Script::new(source).run(), Value::Int(8));
}

#[test]
fn a_linted_match_in_a_lambda_still_runs() {
    let source = r"
        def f = fn x -> match x {
            _ => 'first',
            2 => 'second',
        }
        f(2)
    ";
    assert_unreachable_arms(source, 1);
    assert_eq!(Script::new(source).run(), Value::from("first"));
}

#[test]
fn a_guarded_catch_all_falls_through_when_the_guard_fails() {
    // Not linted, and the later arm is reached.
    let source = r"
        match 2 {
            x if: x > 5 => 'big',
            2 => 'two',
        }
    ";
    assert_unreachable_arms(source, 0);
    assert_eq!(Script::new(source).run(), Value::from("two"));
}

#[test]
fn a_program_with_no_lint_has_no_lints() {
    let output = compile_with("1 + 2", CompilerOptions::new());
    assert!(output.lints.is_empty(), "nothing to lint");
    assert_eq!(output.lints.len(), 0);
}

#[test]
fn turning_lints_off_reports_none() {
    let source = "match 1 { _ => 0, 2 => 0 }";
    let on = compile_with(source, CompilerOptions::new());
    let off = compile_with(source, CompilerOptions::new().with_lints(false));
    assert_eq!(on.lints.len(), 1, "on, the lint is reported");
    assert!(off.lints.is_empty(), "off, no lint is reported");
}

#[test]
fn turning_lints_off_reports_none_in_nested_functions() {
    let source = r"
        def f = fn x -> match x {
            _ => 1,
            2 => 2,
        }
        f
    ";
    let off = compile_with(source, CompilerOptions::new().with_lints(false));
    assert!(off.lints.is_empty());
}

#[test]
fn lints_off_compiles_a_program_that_behaves_the_same() {
    let source = r"
        match 7 {
            n => n + 1,
            7 => 0,
        }
    ";
    let run_with = |options: CompilerOptions| {
        compile_program(FILENAME, source, options)
            .expect("compiles")
            .code
            .close(Default::default())
            .expect("closes")
    };
    let behaviors: Vec<Value> = [
        CompilerOptions::new(),
        CompilerOptions::new().with_lints(false),
    ]
    .into_iter()
    .map(|options| {
        frostlang::Vm::factory()
            .build(run_with(options))
            .run()
            .expect("runs")
            .tail()
            .clone()
    })
    .collect();
    assert_eq!(behaviors[0], Value::Int(8));
    assert_eq!(
        behaviors[0], behaviors[1],
        "the program does the same with lints off"
    );
}

#[test]
fn a_failed_compilation_returns_only_its_errors() {
    let source = r"
        match 1 {
            _ => 0,
            2 => 0,
        }
        undefined_name
    ";
    let errors = compile_program(FILENAME, source, CompilerOptions::new())
        .expect_err("an unbound name fails to compile");
    let text = errors.render_plain();
    assert!(
        !text.contains(CODE),
        "the lint is dropped from a failed compile:\n{text}"
    );
    assert!(
        text.contains("unbound name"),
        "the error is reported:\n{text}"
    );
}

#[test]
fn a_failed_compilation_drops_lints_found_after_the_error_too() {
    let source = r"
        undefined_name
        match 1 {
            _ => 0,
            2 => 0,
        }
    ";
    let errors = compile_program(FILENAME, source, CompilerOptions::new())
        .expect_err("an unbound name fails to compile");
    assert!(!errors.render_plain().contains(CODE));
}

#[test]
fn a_parse_error_returns_no_lints() {
    let errors = compile_program(FILENAME, "match 1 {", CompilerOptions::new())
        .expect_err("an unclosed match fails to parse");
    assert!(!errors.render_plain().contains(CODE));
}

#[test]
fn compile_in_scope_reports_lints() {
    let source = r"
        match outer {
            _ => 0,
            2 => 0,
        }
    ";
    let output =
        compile_in_scope(FILENAME, source, CompilerOptions::new(), &["outer"]).expect("compiles");
    assert_eq!(output.lints.len(), 1);
    assert!(output.lints.render_narrated().contains(CODE));
}

#[test]
fn compile_in_scope_honors_lints_off() {
    let source = "match outer { _ => 0, 2 => 0 }";
    let options = CompilerOptions::new().with_lints(false);
    let output = compile_in_scope(FILENAME, source, options, &["outer"]).expect("compiles");
    assert!(output.lints.is_empty());
}

#[test]
fn compile_in_scope_drops_lints_on_failure() {
    let source = r"
        match outer {
            _ => 0,
            2 => 0,
        }
        undefined_name
    ";
    let errors = compile_in_scope(FILENAME, source, CompilerOptions::new(), &["outer"])
        .expect_err("an unbound name fails to compile");
    assert!(!errors.render_plain().contains(CODE));
}

#[test]
fn lints_are_independent_of_the_optimizations() {
    use frostlang::compile::OptimizationOptions;
    let source = r"
        match 1 {
            _ => 0,
            2 => 0,
            3 => 0,
        }
    ";
    for optimization in script::optimization_permutations() {
        let options = CompilerOptions::new().with_optimization(optimization);
        let output = compile_with(source, options);
        assert_eq!(output.lints.len(), 2, "two lints under {optimization:?}");
    }
    let none = CompilerOptions::new().with_optimization(OptimizationOptions::NONE);
    assert_eq!(compile_with(source, none).lints.len(), 2);
}

#[test]
fn lints_are_the_same_on_every_compile() {
    let source = "match 1 { _ => 0, 2 => 0 }";
    assert_eq!(
        lints(source).render_plain(),
        lints(source).render_plain(),
        "compiling the same source reports the same lints"
    );
}

#[test]
fn a_match_over_a_constant_is_still_linted() {
    // Branch elimination may remove the unreachable arm; the lint is about the
    // source, not the emitted code.
    let source = r"
        match 3 {
            3 => 'three',
            _ => 'any',
            4 => 'four',
        }
    ";
    assert_unreachable_arms(source, 1);
    assert_eq!(Script::new(source).run(), Value::from("three"));
}
