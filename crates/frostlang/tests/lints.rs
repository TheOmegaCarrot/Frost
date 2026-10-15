//! Lints: nonfatal diagnostics in `CompilerOutput::lints`.
//!
//! `unreachable match arm`: every arm after the first arm that matches
//! everything. An arm matches everything when it has no guard and its pattern
//! cannot fail: a name or `_` with no `is` constraint, or an alternative with
//! such a branch. Every other pattern can fail.
//!
//! `unused binding`: a `def` or match name that its function never reads and
//! that is not exported. `lambda in abbreviated lambda`: a `$(...)` that holds
//! a lambda.
//!
//! Each test filters to the lint it covers, since one source may raise several.
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

/// The narrated text of each lint of `source` that names `code`, in order.
/// Other lints of `source` are left out, so a test sees only the lint it
/// covers.
fn lints_with_code(source: &str, code: &str) -> Vec<String> {
    lints(source)
        .iter()
        .map(Diagnostic::render_narrated)
        .filter(|text| text.contains(code))
        .collect()
}

/// The narrated text of each unreachable match arm lint of `source`, in order.
fn narrated(source: &str) -> Vec<String> {
    lints_with_code(source, CODE)
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
/// `label at line 3, columns 5 to 9: never tried`, or `label starting at line
/// 3, column 5: never tried` when the label spans lines.
fn label_line(text: &str, label: &str) -> usize {
    text.lines()
        .filter(|line| line.trim_end().ends_with(&format!(": {label}")))
        .find_map(|line| {
            let line = line.trim();
            let rest = line
                .strip_prefix("label at line ")
                .or_else(|| line.strip_prefix("label starting at line "))?;
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

// Unused bindings.
//
// A binding is a name `def`, a destructuring, or a match pattern introduces. A
// binding that no code in its function reads is linted, unless it is exported.

const UNUSED: &str = "unused binding";

/// The names `source` is linted for never using, in report order, read from
/// each lint's message.
fn unused_names(source: &str) -> Vec<String> {
    unused_names_with(source, CompilerOptions::new(), &[])
}

/// [`unused_names`] under `options`, with `outer` names already in scope.
fn unused_names_with(source: &str, options: CompilerOptions, outer: &[&str]) -> Vec<String> {
    let output = compile_in_scope(FILENAME, source, options, outer)
        .unwrap_or_else(|errors| panic!("{source:?} should compile:\n{}", errors.render_plain()));
    output
        .lints
        .iter()
        .map(Diagnostic::render_narrated)
        .filter(|text| text.contains(UNUSED))
        .map(|text| {
            let (name, _) = text
                .split_once("` is never used")
                .unwrap_or_else(|| panic!("no 'is never used' message in:\n{text}"));
            let (_, name) = name.rsplit_once('`').expect("a quoted name");
            name.to_owned()
        })
        .collect()
}

/// Assert each `(source, names)` case is linted for exactly `names`, in order.
fn assert_unused_cases(cases: &[(&str, &[&str])]) {
    for (source, names) in cases {
        assert_eq!(
            unused_names(source),
            *names,
            "wrong unused bindings for:\n{source}"
        );
    }
}

#[test]
fn an_unread_def_is_linted() {
    assert_unused_cases(&[
        ("def a = 1\n0", &["a"]),
        ("def a = 1", &["a"]),
        ("def a = 1\ndef b = 2\n0", &["a", "b"]),
    ]);
}

#[test]
fn an_unread_defn_is_linted() {
    assert_unused_cases(&[
        ("defn f(x) -> x\n0", &["f"]),
        // The function's own name inside its body is not a read of the def.
        ("defn f(x) -> f(x)\n0", &["f"]),
    ]);
}

#[test]
fn unread_array_destructured_names_are_linted() {
    assert_unused_cases(&[
        ("def [a, b] = [1, 2]\n0", &["a", "b"]),
        ("def [a, b] = [1, 2]\na", &["b"]),
        ("def [a, ...rest] = [1, 2]\na", &["rest"]),
        ("def [a, ...rest] = [1, 2]\nrest", &["a"]),
        ("def [[a, b], c] = [[1, 2], 3]\nb", &["a", "c"]),
    ]);
}

#[test]
fn unread_map_destructured_names_are_linted() {
    assert_unused_cases(&[
        ("def {k} = {k: 1}\n0", &["k"]),
        ("def {k: v} = {k: 1}\n0", &["v"]),
        ("def {k, j: v} = {k: 1, j: 2}\nk", &["v"]),
        ("def {k} as whole = {k: 1}\nk", &["whole"]),
        ("def {k} as whole = {k: 1}\nwhole", &["k"]),
    ]);
}

#[test]
fn unread_match_pattern_names_are_linted() {
    assert_unused_cases(&[
        ("match 1 {\nn => 0,\n}", &["n"]),
        ("match [1] {\n[a, ...r] => a,\n}", &["r"]),
        ("match [1, 2] {\n[a, b] => b,\n_ => 0,\n}", &["a"]),
        ("match {k: 1} {\n{k} => 0,\n_ => 1,\n}", &["k"]),
        ("match {k: 1} {\n{k: v} => 0,\n_ => 1,\n}", &["v"]),
        ("match {k: 1} {\n{k} as whole => k,\n_ => 1,\n}", &["whole"]),
        ("match 1 {\nn is Int => 0,\n_ => 1,\n}", &["n"]),
    ]);
}

#[test]
fn an_unread_name_in_every_alternative_is_linted_once() {
    assert_unused_cases(&[
        ("match [1] {\n[x] | x => 0,\n}", &["x"]),
        ("match [1] {\n[x] | x => x,\n}", &[]),
        ("match [1] {\n[x] | [x, _] | x => 0,\n}", &["x"]),
    ]);
}

#[test]
fn lambda_parameters_are_never_linted() {
    assert_unused_cases(&[
        ("fn a, b -> 0", &[]),
        ("fn a, ...rest -> 0", &[]),
        ("fn ...rest -> 0", &[]),
        ("$(0)", &[]),
        ("$($1)", &[]),
        ("$($2)", &[]),
        ("$($$)", &[]),
        ("fn a -> fn b -> 0", &[]),
        ("map [1] with fn x -> 0", &[]),
        ("reduce [1] with fn (acc, x) -> 0", &[]),
    ]);
}

#[test]
fn a_named_lambdas_own_name_is_never_linted() {
    assert_unused_cases(&[
        ("fn fact(n) -> 0", &[]),
        ("fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)", &[]),
    ]);
}

#[test]
fn an_underscore_is_never_linted() {
    assert_unused_cases(&[
        ("def [_, a] = [1, 2]\na", &[]),
        ("def [_, _, a] = [1, 2, 3]\na", &[]),
        ("def [a, _] = [1, 2]\na", &[]),
        ("match 1 {\n_ => 0,\n}", &[]),
        ("match [1, 2] {\n[_, ...rest] => rest,\n}", &[]),
    ]);
}

#[test]
fn a_captured_name_is_not_a_binding_of_the_capturing_function() {
    // `a` is read by the lambda, so the top level's binding is used; the
    // lambda's capture is not itself a binding to lint.
    assert_unused_cases(&[
        ("def a = 1\nfn -> a", &[]),
        ("def a = 1\nfn -> fn -> a", &[]),
        ("def a = 1\n$(a)", &[]),
    ]);
}

#[test]
fn a_binding_inside_a_lambda_is_linted_in_that_lambda() {
    assert_unused_cases(&[
        ("fn -> {\ndef a = 1\n0\n}", &["a"]),
        ("fn x -> {\ndef a = x\n0\n}", &["a"]),
        ("fn x -> {\ndef a = x\na\n}", &[]),
        ("defn f(x) -> {\ndef a = x\n0\n}\nf", &["a"]),
    ]);
}

#[test]
fn a_read_in_a_later_statement_counts() {
    assert_unused_cases(&[
        ("def a = 1\na", &[]),
        ("def a = 1\ndef b = a\nb", &[]),
        ("def a = 1\nprint(a)\n0", &[]),
        ("def a = 1\n[a]", &[]),
        ("def a = 1\na + 1", &[]),
    ]);
}

#[test]
fn a_read_in_a_guard_counts() {
    assert_unused_cases(&[
        (
            "def limit = 5\nmatch 1 {\nn if: n < limit => n,\n_ => 0,\n}",
            &[],
        ),
        ("match 1 {\nn if: n > 0 => 0,\n_ => 1,\n}", &[]),
    ]);
}

#[test]
fn a_read_in_a_nested_lambda_counts() {
    assert_unused_cases(&[
        ("def a = 1\ndef f = fn -> a\nf", &[]),
        ("def a = 1\ndef f = fn x -> fn y -> a + x + y\nf", &[]),
        ("def a = 1\ndef f = $($ + a)\nf", &[]),
        ("def a = 1\nmap [1] with fn x -> x + a", &[]),
    ]);
}

#[test]
fn a_read_in_a_format_string_counts() {
    assert_unused_cases(&[
        ("def name = 'x'\n$'hello, ${name}'", &[]),
        ("def a = 1\ndef b = 2\n$'${a + b}'", &[]),
    ]);
}

#[test]
fn a_read_as_a_match_target_counts() {
    assert_unused_cases(&[
        ("def v = 1\nmatch v {\n_ => 0,\n}", &[]),
        ("def v = 1\nmatch v {\n(v) => 0,\n_ => 1,\n}", &[]),
    ]);
}

#[test]
fn a_read_in_a_value_pattern_counts() {
    assert_unused_cases(&[("def y = 1\nmatch 1 {\n(y) => 0,\n_ => 1,\n}", &[])]);
}

#[test]
fn a_read_in_an_iterative_expression_counts() {
    assert_unused_cases(&[
        ("def xs = [1, 2]\nmap xs with fn x -> x", &[]),
        ("def xs = [1, 2]\nfilter xs with fn x -> true", &[]),
        (
            "def xs = [1, 2]\nreduce xs with fn (acc, x) -> acc + x",
            &[],
        ),
        ("def n = 2\nmap [1, 2] with $($ * n)", &[]),
        ("def f = fn x -> x\nmap [1, 2] with f", &[]),
    ]);
}

#[test]
fn a_read_in_every_kind_of_expression_counts() {
    assert_unused_cases(&[
        ("def a = [1]\na[0]", &[]),
        ("def m = {k: 1}\nm.k", &[]),
        ("def a = 1\n-a", &[]),
        ("def a = true\nnot a", &[]),
        ("def a = true\na and 1", &[]),
        ("def a = true\nif a: 1 else: 2", &[]),
        ("def f = fn x -> x\nf(1)", &[]),
        ("def f = fn x -> x\n1 @ f()", &[]),
        ("def a = 1\n{k: a}", &[]),
        ("def a = 1\ndo {\na\n}", &[]),
    ]);
}

#[test]
fn a_read_in_dead_code_still_counts() {
    // The lint is about the source, not about what the optimizer keeps.
    assert_unused_cases(&[
        ("def a = 1\nif false: a else: 0", &[]),
        ("def a = 1\nmatch 3 {\n3 => 0,\n_ => a,\n}", &[]),
    ]);
}

#[test]
fn a_binding_read_only_by_itself_in_a_nested_def_is_still_unused_outside() {
    // The inner `a` shadows the outer; only the inner is read.
    let source = r"
        def a = 1
        def r = do {
            def a = 2
            a
        }
        r
    ";
    assert_unused_cases(&[(source, &["a"])]);
}

#[test]
fn shadowing_in_a_do_block_lints_each_binding_on_its_own() {
    let outer_read = r"
        def a = 1
        def r = do {
            def a = 2
            0
        }
        [a, r]
    ";
    assert_unused_cases(&[(outer_read, &["a"])]);
    let both_unread = r"
        def a = 1
        def r = do {
            def a = 2
            0
        }
        r
    ";
    assert_unused_cases(&[(both_unread, &["a", "a"])]);
}

#[test]
fn a_shadowed_binding_is_labeled_where_it_is_bound() {
    let source = "def a = 1\ndef r = do {\n    def a = 2\n    a\n}\nr";
    let plain = lints_with_code(source, UNUSED);
    assert_eq!(plain.len(), 1, "{plain:?}");
    assert_eq!(
        label_line(&plain[0], "bound here"),
        1,
        "the outer a:\n{}",
        plain[0]
    );
}

#[test]
fn a_binding_shadowing_a_global_is_linted_when_unread() {
    assert_unused_cases(&[
        ("def print = 1\n0", &["print"]),
        ("def print = 1\nprint", &[]),
    ]);
}

#[test]
fn an_unread_binding_in_a_match_arm_result_is_linted() {
    let source = r"
        match 1 {
            1 => do {
                def inside = 5
                0
            },
            _ => 2,
        }
    ";
    assert_unused_cases(&[(source, &["inside"])]);
}

#[test]
fn a_match_name_is_scoped_to_its_arm() {
    // Each arm's `n` is its own binding.
    let source = r"
        match 1 {
            2 => 0,
            n if: n > 5 => 1,
            n => 2,
        }
    ";
    assert_unused_cases(&[(source, &["n"])]);
}

#[test]
fn an_explicit_export_is_not_linted() {
    assert_unused_cases(&[
        ("export def a = 1", &[]),
        ("export defn f(x) -> x", &[]),
        ("export def a = 1\ndef b = 2\n0", &["b"]),
        ("export def [a, b] = [1, 2]", &[]),
        ("export def {k, j: v} = {k: 1, j: 2}", &[]),
        ("export def {k} as whole = {k: 1}", &[]),
    ]);
}

#[test]
fn with_implicit_export_every_top_level_binding_is_exported() {
    let options = || CompilerOptions::new().with_implicit_export(true);
    for source in [
        "def a = 1",
        "defn f(x) -> x",
        "def [a, b] = [1, 2]",
        "def {k, j: v} = {k: 1, j: 2}",
        "def a = 1\ndef b = 2",
    ] {
        assert_eq!(
            unused_names_with(source, options(), &[]),
            Vec::<String>::new(),
            "implicitly exported, so not linted:\n{source}"
        );
        assert!(
            !unused_names(source).is_empty(),
            "without implicit export it is linted:\n{source}"
        );
    }
}

#[test]
fn implicit_export_does_not_excuse_a_binding_in_a_nested_scope() {
    let source = r"
        def f = fn -> {
            def inner = 1
            0
        }
        f
    ";
    let options = CompilerOptions::new().with_implicit_export(true);
    assert_eq!(unused_names_with(source, options, &[]), ["inner"]);
}

#[test]
fn match_names_are_still_linted_with_implicit_export() {
    let source = "match 1 {\nn => 0,\n}";
    let options = CompilerOptions::new().with_implicit_export(true);
    assert_eq!(unused_names_with(source, options, &[]), ["n"]);
}

#[test]
fn names_supplied_by_the_outer_scope_are_not_linted() {
    let options = CompilerOptions::new;
    assert_eq!(
        unused_names_with("0", options(), &["outer"]),
        Vec::<String>::new()
    );
    assert_eq!(
        unused_names_with("outer", options(), &["outer"]),
        Vec::<String>::new()
    );
    assert_eq!(
        unused_names_with("def mine = outer\n0", options(), &["outer"]),
        ["mine"],
        "a binding of the script is still linted"
    );
    assert_eq!(
        unused_names_with("fn -> outer", options(), &["a", "b", "outer"]),
        Vec::<String>::new()
    );
}

#[test]
fn the_unused_binding_lint_names_the_code_the_message_the_label_and_the_help() {
    let texts = lints_with_code("def spare = 1\n0", UNUSED);
    assert_eq!(texts.len(), 1);
    let text = &texts[0];
    for expected in [
        UNUSED,
        "`spare` is never used",
        "bound here",
        "remove it, or bind `_` instead",
        "severity: warning",
        FILENAME,
    ] {
        assert!(
            text.contains(expected),
            "the lint should mention {expected:?}:\n{text}"
        );
    }
    assert_eq!(label_line(text, "bound here"), 1);
    assert!(
        text.contains("columns 5 to 9: bound here"),
        "the label starts at the name:\n{text}"
    );
}

#[test]
fn the_unused_binding_label_covers_exactly_the_name() {
    let plain = lints_with_code("def spare = 1\n0", UNUSED);
    assert_eq!(plain.len(), 1);
    let rendered = lints("def spare = 1\n0").render_plain();
    assert_eq!(
        underlined(&rendered, "def spare = 1"),
        "spare",
        "the name, not the whole definition:\n{rendered}"
    );
    let destructured = lints("def [first, second] = [1, 2]\nsecond").render_plain();
    assert_eq!(
        underlined(&destructured, "def [first, second] = [1, 2]"),
        "first",
        "the destructured name:\n{destructured}"
    );
}

#[test]
fn the_unused_binding_lint_is_a_warning() {
    let output = compile_with("def a = 1\n0", CompilerOptions::new());
    assert_eq!(output.lints.len(), 1);
    let narrated = output.lints.render_narrated();
    assert!(narrated.contains("severity: warning"), "{narrated}");
}

#[test]
fn unused_binding_lints_are_in_source_order() {
    let source = r"
        def first = 1
        def second = fn -> {
            def inner = 1
            0
        }
        def third = 3
        second
    ";
    assert_eq!(unused_names(source), ["first", "inner", "third"]);
}

#[test]
fn lints_across_nested_functions_and_kinds_are_in_source_order() {
    let source = r"
        def a = 1
        def f = fn x -> {
            def b = 2
            match x {
                _ => 0,
                2 => 0,
            }
        }
        def c = 3
        f
    ";
    let order: Vec<String> = lints(source)
        .iter()
        .map(|lint| {
            let text = lint.render_narrated();
            if text.contains(UNUSED) {
                format!("unused line {}", label_line(&text, "bound here"))
            } else {
                format!("arm line {}", label_line(&text, "never tried"))
            }
        })
        .collect();
    assert_eq!(
        order,
        [
            "unused line 2",
            "unused line 4",
            "arm line 7",
            "unused line 10"
        ],
        "source order holds across nesting and across lint kinds"
    );
}

#[test]
fn lints_of_a_lambda_nested_deeper_are_in_source_order() {
    let source = r"
        def outer = fn -> {
            def one = 1
            def inner = fn -> {
                def two = 2
                0
            }
            def three = 3
            inner
        }
        outer
    ";
    assert_eq!(unused_names(source), ["one", "two", "three"]);
}

#[test]
fn unused_bindings_behave_the_same_with_lints_off() {
    let source = "def a = 1\nmatch 1 {\nn => 0,\n}";
    let on = compile_with(source, CompilerOptions::new());
    let off = compile_with(source, CompilerOptions::new().with_lints(false));
    assert_eq!(on.lints.len(), 2, "on, both are reported");
    assert!(off.lints.is_empty(), "off, none is reported");
}

#[test]
fn lints_off_reports_no_unused_binding_in_nested_functions() {
    let source = r"
        def f = fn -> {
            def inner = 1
            0
        }
        f
    ";
    let off = compile_with(source, CompilerOptions::new().with_lints(false));
    assert!(off.lints.is_empty());
    let off_scoped = compile_in_scope(
        FILENAME,
        "def a = 1",
        CompilerOptions::new().with_lints(false),
        &[],
    )
    .expect("compiles");
    assert!(off_scoped.lints.is_empty());
}

#[test]
fn an_unused_binding_does_not_change_what_the_program_does() {
    let source = r"
        def spare = 100
        def [a, extra] = [1, 2]
        def f = fn x -> {
            def unused_inner = x
            x + a
        }
        f(10)
    ";
    assert_eq!(unused_names(source), ["spare", "extra", "unused_inner"]);
    assert_eq!(Script::new(source).run(), Value::Int(11));
}

#[test]
fn an_unused_binding_is_linted_under_every_optimization() {
    let source = "def a = 1\ndef b = a\n0";
    for optimization in script::optimization_permutations() {
        let options = CompilerOptions::new().with_optimization(optimization);
        assert_eq!(
            unused_names_with(source, options, &[]),
            ["b"],
            "under {optimization:?}"
        );
    }
}

#[test]
fn a_binding_read_only_where_constant_propagation_inlines_it_is_used() {
    // Propagation replaces the read with the constant; the read still counts.
    let source = "def a = 1\ndef b = a + 1\nb";
    for optimization in script::optimization_permutations() {
        let options = CompilerOptions::new().with_optimization(optimization);
        assert_eq!(
            unused_names_with(source, options, &[]),
            Vec::<String>::new(),
            "under {optimization:?}"
        );
    }
}

#[test]
fn a_failed_compilation_returns_no_unused_binding_lint() {
    let errors = compile_program(
        FILENAME,
        "def a = 1\nundefined_name",
        CompilerOptions::new(),
    )
    .expect_err("an unbound name fails to compile");
    assert!(!errors.render_plain().contains(UNUSED));
}

// Lambda in abbreviated lambda.

const IN_ABBREVIATED: &str = "lambda in abbreviated lambda";

fn abbreviated_lints(source: &str) -> Vec<String> {
    lints_with_code(source, IN_ABBREVIATED)
}

#[test]
fn an_abbreviated_lambda_holding_a_lambda_is_linted() {
    for source in [
        "$(fn x -> x)",
        "$(fn -> 0)",
        "$(fn x -> fn y -> y)",
        "def f = $(fn x -> x)\nf",
        "map [1] with $(fn x -> x)",
        "$($1 + 1 @ transform(fn x -> x))",
    ] {
        assert_eq!(
            abbreviated_lints(source).len(),
            1,
            "one lint for:\n{source}"
        );
    }
}

#[test]
fn an_abbreviated_lambda_with_no_lambda_is_not_linted() {
    for source in [
        "$(0)",
        "$($ * 2)",
        "$($1 + $2)",
        "$($$)",
        "$([$, 1])",
        "def f = fn x -> $(x)\nf",
        "map [1, 2] with $($ * 2)",
    ] {
        assert!(
            abbreviated_lints(source).is_empty(),
            "no lint for:\n{source}"
        );
    }
}

#[test]
fn an_abbreviated_lambda_is_linted_once_however_many_lambdas_it_holds() {
    let source = "$([fn a -> a, fn b -> b, fn c -> c])";
    assert_eq!(abbreviated_lints(source).len(), 1);
    let nested = "$(fn a -> fn b -> fn c -> c)";
    assert_eq!(abbreviated_lints(nested).len(), 1);
}

#[test]
fn a_lambda_nested_deeper_does_not_lint_the_abbreviated_lambda_again() {
    let source = "$(fn a -> [fn b -> b, fn c -> fn d -> d])";
    assert_eq!(abbreviated_lints(source).len(), 1);
}

#[test]
fn a_fn_lambda_holding_an_abbreviated_lambda_is_not_linted_when_it_holds_no_lambda() {
    let source = "fn x -> $(0)";
    assert!(abbreviated_lints(source).is_empty());
}

#[test]
fn an_abbreviated_lambda_inside_a_fn_lambda_lints_on_its_own() {
    let source = "def f = fn a -> $(fn b -> b)\nf";
    let found = abbreviated_lints(source);
    assert_eq!(
        found.len(),
        1,
        "the abbreviated lambda holds a lambda:\n{found:?}"
    );
    assert!(
        found[0].contains("columns 17 to 18: abbreviated lambda"),
        "the inner abbreviated lambda is the primary label, not the outer fn:\n{}",
        found[0]
    );
}

#[test]
fn an_abbreviated_lambda_cannot_be_nested_in_another_so_it_never_nests_the_lint() {
    for source in ["$($(1))", "$(fn x -> $(x + $))"] {
        let errors = compile_program(FILENAME, source, CompilerOptions::new())
            .expect_err("a nested abbreviated lambda is an error");
        assert!(
            errors
                .render_plain()
                .contains("an abbreviated lambda cannot be nested in another"),
            "{source}:\n{}",
            errors.render_plain()
        );
    }
}

#[test]
fn the_abbreviated_lambda_lint_names_the_code_the_message_the_labels_and_the_help() {
    let found = abbreviated_lints("def f = $(fn x -> x)\nf");
    assert_eq!(found.len(), 1);
    let text = &found[0];
    for expected in [
        IN_ABBREVIATED,
        "this abbreviated lambda holds a lambda",
        "abbreviated lambda",
        "the lambda it holds",
        "write it as a `fn` lambda, naming its parameters",
        FILENAME,
    ] {
        assert!(
            text.contains(expected),
            "the lint should mention {expected:?}:\n{text}"
        );
    }
}

#[test]
fn the_abbreviated_lambda_lint_is_advice_not_a_warning() {
    let output = compile_with("$(fn x -> x)", CompilerOptions::new());
    assert_eq!(output.lints.len(), 1);
    let narrated = output.lints.render_narrated();
    assert!(narrated.contains("severity: advice"), "{narrated}");
    assert!(!narrated.contains("severity: warning"), "{narrated}");
}

/// The primary label covers only the `$(`, not the whole abbreviated lambda,
/// so that it does not overlap the label on the lambda it holds.
#[test]
fn the_abbreviated_lambda_lint_labels_its_opener_and_the_lambda_it_holds() {
    let source = "def f = $(fn x -> x)\nf";
    let plain = lints(source).render_plain();
    let found = abbreviated_lints(source);
    assert_eq!(label_line(&found[0], "abbreviated lambda"), 1);
    assert_eq!(label_line(&found[0], "the lambda it holds"), 1);
    assert!(
        found[0].contains("columns 9 to 10: abbreviated lambda"),
        "the primary label is the `$(`:\n{}",
        found[0]
    );
    assert!(
        found[0].contains("columns 11 to 19: the lambda it holds"),
        "the secondary label starts at `fn`:\n{}",
        found[0]
    );
    assert!(plain.contains("$(fn x -> x)"), "{plain}");
}

#[test]
fn the_abbreviated_lambda_lint_is_off_with_lints_off() {
    let off = compile_with("$(fn x -> x)", CompilerOptions::new().with_lints(false));
    assert!(off.lints.is_empty());
    let off_scoped = compile_in_scope(
        FILENAME,
        "$(fn x -> outer)",
        CompilerOptions::new().with_lints(false),
        &["outer"],
    )
    .expect("compiles");
    assert!(off_scoped.lints.is_empty());
}

#[test]
fn the_abbreviated_lambda_lint_is_reported_by_compile_in_scope() {
    let output = compile_in_scope(
        FILENAME,
        "$(fn x -> outer)",
        CompilerOptions::new(),
        &["outer"],
    )
    .expect("compiles");
    assert_eq!(output.lints.len(), 1);
    assert!(output.lints.render_narrated().contains(IN_ABBREVIATED));
}

#[test]
fn the_abbreviated_lambda_lint_does_not_change_what_the_program_does() {
    let source = "$(fn x -> x + $)(3)(4)";
    assert_eq!(abbreviated_lints(source).len(), 1);
    assert_eq!(Script::new(source).run(), Value::Int(7));
}

#[test]
fn the_abbreviated_lambda_lint_is_independent_of_the_optimizations() {
    let source = "def f = $(fn x -> x)\nf";
    for optimization in script::optimization_permutations() {
        let options = CompilerOptions::new().with_optimization(optimization);
        let output = compile_with(source, options);
        assert_eq!(output.lints.len(), 1, "under {optimization:?}");
    }
}

#[test]
fn lints_of_an_abbreviated_lambda_are_in_source_order_with_the_others() {
    let source = r"
        def first = 1
        def f = $(fn x -> {
            def inner = 2
            match x {
                _ => 0,
                3 => 0,
            }
        })
        def last = 4
        f
    ";
    let order: Vec<String> = lints(source)
        .iter()
        .map(|lint| {
            let text = lint.render_narrated();
            let (kind, label) = if text.contains(UNUSED) {
                ("unused", "bound here")
            } else if text.contains(IN_ABBREVIATED) {
                ("abbreviated", "abbreviated lambda")
            } else {
                ("arm", "never tried")
            };
            format!("{kind} {}", label_line(&text, label))
        })
        .collect();
    assert_eq!(
        order,
        [
            "unused 2",
            "abbreviated 3",
            "unused 4",
            "arm 7",
            "unused 10"
        ],
        "sorted by the start of each primary label"
    );
}

#[test]
fn an_abbreviated_lambda_holding_a_lambda_that_binds_an_unused_name_lints_both() {
    let source = "def f = $(fn x -> {\ndef spare = 1\nx\n})\nf";
    assert_eq!(unused_names(source), ["spare"]);
    assert_eq!(abbreviated_lints(source).len(), 1);
}
