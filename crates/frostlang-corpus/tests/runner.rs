//! The corpus runner's own tests: that it passes what behaves as expected,
//! and fails, flags, or rejects everything else, each for the right reason.
//!
//! Scripts here are written inline and checked by name, as if they sat at
//! that path in the corpus, so the corpus itself holds only real entries.

use std::path::Path;

use frostlang::Value;
use frostlang::compile::{Optimization, OptimizationOptions};
use frostlang_corpus::{
    Benchmark, Entry, Kind, KnownIssue, MAX_ID_PART, Mode, OptionSet, Report, Tally, check,
    check_source, discover, option_sets, prepare,
};

/// The report for `source`, named `name`, under the default options.
fn report(name: &str, source: &str) -> Report {
    check_source(name, source, Mode::Default).report
}

/// The reason `source` fails, which it must.
fn failure(name: &str, source: &str) -> String {
    match report(name, source) {
        Report::Failed(reason) => reason,
        other => panic!("{name} should fail, but was reported as {other:?}"),
    }
}

/// The reason `source` is malformed, which it must be.
fn malformation(name: &str, source: &str) -> String {
    match report(name, source) {
        Report::Malformed(reason) => reason,
        other => panic!("{name} should be malformed, but was reported as {other:?}"),
    }
}

fn assert_mentions(text: &str, fragment: &str) {
    assert!(
        text.contains(fragment),
        "expected the report to mention {fragment:?}, but it reads:\n{text}"
    );
}

// -- Passing --

#[test]
fn a_good_script_matching_its_expectation_passes() {
    let source = r"
#= {result: [1, 'two', 3.0, null]}
[1, 'two', 3.0, null]
";
    assert_eq!(report("area/good/script", source), Report::Passed);
}

#[test]
fn a_null_result_is_an_expectation_not_an_absent_one() {
    let source = r"
#= {result: null}
null
";
    assert_eq!(report("area/good/script", source), Report::Passed);
}

#[test]
fn exports_and_printed_output_are_compared() {
    let source = r"
#= {
#=     result: 3,
#=     exports: {a: 1, b: 'b'},
#=     printed: ['first', 'second'],
#= }
print('first')
export def a = 1
print('second')
export def b = 'b'
3
";
    assert_eq!(report("area/good/script", source), Report::Passed);
}

#[test]
fn a_runtime_error_passes_when_its_message_contains_every_fragment_ignoring_case() {
    let source = r"
#= {raises: ['NO MATCH', 'arm']}
match 5 {
    1 => 'one',
}
";
    assert_eq!(report("area/runtime-error/script", source), Report::Passed);
}

#[test]
fn a_compile_error_passes_when_its_diagnostics_contain_every_fragment() {
    let source = r"
#= {rejects: ['`missing`', 'not defined']}
missing
";
    assert_eq!(report("area/compile-error/script", source), Report::Passed);
}

#[test]
fn a_script_passes_under_every_permutation_mode() {
    let source = r"
#= {result: 6}
def add = fn x, y -> x + y
add(1, 5)
";
    for mode in [Mode::Default, Mode::Permutations, Mode::Exhaustive] {
        assert_eq!(
            check_source("area/good/script", source, mode).report,
            Report::Passed,
            "under {mode:?}"
        );
    }
}

#[test]
fn inputs_are_imported_with_their_values() {
    let source = r"
#= {
#=     result: [3, 'xy', 7],
#=     inputs: {n: 3, s: 'x', f: fn v -> v + 1},
#= }
def {n, s, f} = import('inputs')
[n, s + 'y', f(6)]
";
    assert_eq!(report("area/good/script", source), Report::Passed);
}

#[test]
fn inputs_reach_every_kind_of_script() {
    let raises = r"
#= {raises: 'no match', inputs: {n: 2}}
def {n} = import('inputs')
match n {
    1 => 1,
}
";
    assert_eq!(report("area/runtime-error/script", raises), Report::Passed);
    // The input is bound, so only the other name is unbound.
    let rejects = r"
#= {rejects: '`other`', inputs: {n: 2}}
def {n} = import('inputs')
n + other
";
    assert_eq!(report("area/compile-error/script", rejects), Report::Passed);
}

#[test]
fn a_script_compiles_on_its_own_so_an_input_not_imported_is_unbound() {
    let source = r"
#= {rejects: '`n` is not defined', inputs: {n: 2}}
n + 1
";
    assert_eq!(report("area/compile-error/script", source), Report::Passed);
}

#[test]
fn inputs_agree_under_every_permutation() {
    let source = r"
#= {result: 10, inputs: {n: 4}}
def {n} = import('inputs')
def double = fn x -> x * 2
double(n) + 2
";
    assert_eq!(
        check_source("area/good/script", source, Mode::Exhaustive).report,
        Report::Passed
    );
}

#[test]
fn an_input_must_be_a_bindable_name() {
    for name in ["'if'", "'1x'", "'$'", "'a b'"] {
        let reason = malformation(
            "area/good/script",
            &format!("#= {{result: 1, inputs: {{[{name}]: 1}}}}\n1\n"),
        );
        assert_mentions(&reason, "not a name a script can bind");
    }
}

// -- Failing --

#[test]
fn a_wrong_result_fails_showing_both_values() {
    let reason = failure("area/good/script", "#= {result: 'one'}\n'two'\n");
    assert_mentions(&reason, r#"expected "one", got "two""#);
}

#[test]
fn equality_is_frosts_own_so_an_int_never_matches_a_float() {
    let reason = failure("area/good/script", "#= {result: 3}\n3.0\n");
    assert_mentions(&reason, "result: expected 3, got 3.0");
}

#[test]
fn unexpected_printed_output_fails() {
    let reason = failure("area/good/script", "#= {result: null}\nprint('surprise')\n");
    assert_mentions(&reason, "printed");
    assert_mentions(&reason, "surprise");
}

#[test]
fn an_unexpected_export_fails() {
    let reason = failure("area/good/script", "#= {result: 1}\nexport def a = 1\na\n");
    assert_mentions(&reason, "exports");
}

#[test]
fn a_good_script_that_raises_fails() {
    let reason = failure(
        "area/good/script",
        "#= {result: 1}\nmatch 2 {\n    1 => 1,\n}\n",
    );
    assert_mentions(&reason, "expected it to complete");
}

#[test]
fn a_good_script_that_does_not_compile_fails() {
    let reason = failure("area/good/script", "#= {result: 1}\nmissing\n");
    assert_mentions(&reason, "expected it to compile");
}

#[test]
fn a_runtime_error_script_that_completes_fails() {
    let reason = failure("area/runtime-error/script", "#= {raises: 'boom'}\n1\n");
    assert_mentions(&reason, "expected it to raise");
}

#[test]
fn a_runtime_error_missing_a_fragment_fails() {
    let reason = failure(
        "area/runtime-error/script",
        "#= {raises: ['no match', 'unrelated']}\nmatch 2 {\n    1 => 1,\n}\n",
    );
    assert_mentions(&reason, r#"does not mention ["unrelated"]"#);
}

#[test]
fn a_compile_error_script_that_compiles_fails() {
    let reason = failure("area/compile-error/script", "#= {rejects: 'nope'}\n1\n");
    assert_mentions(&reason, "expected it not to compile");
}

#[test]
fn a_compile_error_fragment_cannot_match_the_echoed_source() {
    // The diagnostics quote the offending line, which holds the fragment.
    let source = r"
#= {rejects: 'sentinel_text'}
missing + 'sentinel_text'
";
    let reason = failure("area/compile-error/script", source);
    assert_mentions(&reason, r#"does not mention ["sentinel_text"]"#);
}

#[test]
fn a_compile_error_fragment_cannot_match_the_scripts_name() {
    // The diagnostics name the file, which holds the fragment.
    let source = r"
#= {rejects: 'sentinel_name'}
missing
";
    let reason = failure("area/compile-error/sentinel_name", source);
    assert_mentions(&reason, r#"does not mention ["sentinel_name"]"#);
}

// -- Lints --

#[test]
fn a_script_compiling_with_its_declared_lints_passes() {
    let source = r"
#= {result: 3, lints: [['unused binding', '`first`'], ['UNUSED BINDING', '`second`']]}
def first = 1
def second = 2
3
";
    assert_eq!(report("area/good/script", source), Report::Passed);
}

#[test]
fn a_runtime_error_script_may_declare_lints() {
    let source = r"
#= {raises: 'division by zero', lints: [['unused binding', '`unused`']]}
def unused = 1 / 0
2
";
    assert_eq!(report("area/runtime-error/script", source), Report::Passed);
}

#[test]
fn an_undeclared_lint_fails_a_good_script() {
    let source = r"
#= {result: 2}
def unused = 1
2
";
    let reason = failure("area/good/script", source);
    assert_mentions(&reason, "lints: expected 0, got 1");
    assert_mentions(&reason, "`unused` is never used");
}

#[test]
fn an_undeclared_lint_fails_a_runtime_error_script() {
    let source = r"
#= {raises: 'division by zero'}
def unused = 1 / 0
2
";
    let reason = failure("area/runtime-error/script", source);
    assert_mentions(&reason, "lints: expected 0, got 1");
}

#[test]
fn a_declared_lint_that_does_not_appear_fails() {
    let source = r"
#= {result: 1, lints: ['unused binding']}
def used = 1
used
";
    let reason = failure("area/good/script", source);
    assert_mentions(&reason, "lints: expected 1, got 0");
}

#[test]
fn lints_are_matched_in_source_order() {
    let source = r"
#= {result: 3, lints: [['unused binding', '`second`'], ['unused binding', '`first`']]}
def first = 1
def second = 2
3
";
    let reason = failure("area/good/script", source);
    assert_mentions(&reason, r#"lint 1 does not mention ["`second`"]"#);
}

#[test]
fn a_lint_fragment_cannot_match_the_echoed_source() {
    // The lint quotes the line binding the name, which holds the fragment.
    let source = r"
#= {result: 2, lints: [['unused binding', 'sentinel_text']]}
def unused = 'sentinel_text'
2
";
    let reason = failure("area/good/script", source);
    assert_mentions(&reason, r#"lint 1 does not mention ["sentinel_text"]"#);
}

#[test]
fn a_lint_mismatch_is_reported_beside_a_wrong_result() {
    let source = r"
#= {result: 3}
def unused = 1
2
";
    let reason = failure("area/good/script", source);
    assert_mentions(&reason, "result: expected 3, got 2");
    assert_mentions(&reason, "lints: expected 0, got 1");
}

// -- Known issues --

#[test]
fn a_known_issue_that_still_fails_counts_as_a_success() {
    let source = r"
#= {result: 1, known_issue: 'FROST-1'}
2
";
    let report = report("area/good/script", source);
    let Report::KnownIssue { issue, detail } = &report else {
        panic!("expected a known issue, got {report:?}");
    };
    assert_eq!(
        issue,
        &KnownIssue {
            task: "FROST-1".to_string(),
            regression: false
        }
    );
    assert_mentions(detail, "result");
    assert!(report.is_success(), "a known issue does not fail the run");
}

#[test]
fn a_known_issue_can_be_marked_a_regression() {
    let source = r"
#= {result: 1, known_issue: 'FROST-12.3', regression: true}
2
";
    let report = report("area/good/script", source);
    assert!(
        matches!(&report, Report::KnownIssue { issue, .. } if issue.regression && issue.task == "FROST-12.3"),
        "expected a regression known issue, got {report:?}"
    );
}

#[test]
fn a_known_issue_that_now_passes_fails_the_run() {
    let source = r"
#= {result: 1, known_issue: 'FROST-1'}
1
";
    let report = report("area/good/script", source);
    assert!(
        matches!(&report, Report::Resolved(issue) if issue.task == "FROST-1"),
        "expected a resolved known issue, got {report:?}"
    );
    assert!(!report.is_success(), "a stale marker fails the run");
    assert_mentions(
        &report.failure().unwrap(),
        "remove its `known_issue` marker",
    );
}

#[test]
fn a_known_issue_does_not_excuse_a_malformed_script() {
    let reason = malformation(
        "area/runtime-error/script",
        "#= {result: 1, known_issue: 'FROST-1'}\n1\n",
    );
    assert_mentions(&reason, "`runtime-error` directory");
}

// -- Malformed scripts --

#[test]
fn a_script_outside_an_outcome_directory_is_malformed() {
    let reason = malformation("area/script", "#= {result: 1}\n1\n");
    assert_mentions(&reason, "good, runtime-error, compile-error");
}

#[test]
fn a_script_in_the_wrong_outcome_directory_is_malformed() {
    let reason = malformation("area/compile-error/script", "#= {result: 1}\n1\n");
    assert_mentions(&reason, "good script");
}

#[test]
fn a_script_without_an_expectation_is_malformed() {
    let reason = malformation("area/good/script", "# just prose\n1\n");
    assert_mentions(&reason, "no `#=` expectation block");
}

#[test]
fn an_expectation_split_in_two_is_malformed() {
    let source = r"
#= {
#=     result: 1,
# a gap
#= }
1
";
    let reason = malformation("area/good/script", source);
    assert_mentions(&reason, "contiguous");
}

#[test]
fn an_expectation_must_name_exactly_one_outcome() {
    for expectation in [
        "{}",
        "{result: 1, raises: 'x'}",
        "{raises: 'x', rejects: 'y'}",
    ] {
        let reason = malformation("area/good/script", &format!("#= {expectation}\n1\n"));
        assert_mentions(&reason, "exactly one of");
    }
}

#[test]
fn an_expectation_with_an_unknown_field_is_malformed() {
    let reason = malformation("area/good/script", "#= {result: 1, reslut: 1}\n1\n");
    assert_mentions(&reason, "reslut");
}

#[test]
fn exports_and_printed_belong_only_to_a_good_script() {
    let reason = malformation(
        "area/runtime-error/script",
        "#= {raises: 'x', printed: []}\n1\n",
    );
    assert_mentions(&reason, "only an expectation with `result`");
}

#[test]
fn lints_do_not_belong_to_a_compile_error_script() {
    let reason = malformation(
        "area/compile-error/script",
        "#= {rejects: 'x', lints: ['unused binding']}\nmissing\n",
    );
    assert_mentions(&reason, "only an expectation with `result` or `raises`");
}

#[test]
fn an_empty_lints_list_is_malformed() {
    let reason = malformation("area/good/script", "#= {result: 1, lints: []}\n1\n");
    assert_mentions(&reason, "leave it out for none");
}

#[test]
fn a_lint_entry_with_blank_or_empty_fragments_is_malformed() {
    for entry in ["''", "' '", "[]", "['ok', '']"] {
        let reason = malformation(
            "area/good/script",
            &format!("#= {{result: 1, lints: [{entry}]}}\n1\n"),
        );
        assert_mentions(&reason, "at least one fragment");
    }
}

#[test]
fn blank_or_empty_fragments_are_malformed() {
    for fragments in ["''", "' '", "[]", "['ok', '']"] {
        let reason = malformation(
            "area/runtime-error/script",
            &format!("#= {{raises: {fragments}}}\n1\n"),
        );
        assert_mentions(&reason, "at least one fragment");
    }
}

#[test]
fn a_known_issue_must_name_a_task() {
    for task in ["'123'", "'FROST-'", "'FROST-1.'", "'frost-1'", "1"] {
        let reason = malformation(
            "area/good/script",
            &format!("#= {{result: 1, known_issue: {task}}}\n1\n"),
        );
        assert!(
            reason.contains("known_issue"),
            "{task}: expected the report to mention known_issue, but it reads:\n{reason}"
        );
    }
}

#[test]
fn a_regression_without_a_known_issue_is_malformed() {
    let reason = malformation("area/good/script", "#= {result: 1, regression: true}\n1\n");
    assert_mentions(&reason, "requires a `known_issue`");
}

#[test]
fn an_expectation_that_is_not_a_map_is_malformed() {
    let reason = malformation("area/good/script", "#= [1]\n1\n");
    assert_mentions(&reason, "malformed");
}

#[test]
fn an_expectation_that_does_not_compile_is_malformed() {
    let reason = malformation("area/good/script", "#= {result: missing}\n1\n");
    assert_mentions(&reason, "does not compile");
}

#[test]
fn an_expectation_that_raises_is_malformed() {
    let reason = malformation("area/good/script", "#= {result: match 1 { 2 => 2 }}\n1\n");
    assert_mentions(&reason, "raised");
}

// -- Benchmarks --

/// A fixture corpus root under `tests/fixtures/`.
fn fixture(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

/// The labels of `sets`, in order.
fn labels(sets: &[OptionSet]) -> Vec<&str> {
    sets.iter().map(|set| set.label.as_str()).collect()
}

#[test]
fn a_benchmark_is_checked_like_any_good_script() {
    for bench in ["true", "{inputs: {n: 100}}"] {
        let source = format!(
            r"#= {{result: 6, inputs: {{n: 3}}, bench: {bench}}}
def {{n}} = import('inputs')
n * 2
"
        );
        assert_eq!(
            report("benchmarks/area/good/script", &source),
            Report::Passed,
            "bench: {bench}"
        );
        // Its own inputs are what its check runs with.
        let wrong = source.replace("result: 6", "result: 200");
        assert_mentions(
            &failure("benchmarks/area/good/script", &wrong),
            "expected 200, got 6",
        );
    }
}

#[test]
fn bench_is_allowed_only_in_a_good_script_under_benchmarks() {
    let outside = r"#= {result: 1, bench: true}
1
";
    assert_mentions(
        &malformation("area/good/script", outside),
        "only a script under `benchmarks/` may have `bench`",
    );
    let raising = r"#= {raises: 'no match', bench: true}
match 1 {
    2 => 2,
}
";
    assert_mentions(
        &malformation("benchmarks/area/runtime-error/script", raising),
        "only an expectation with `result` may have `bench`",
    );
}

#[test]
fn a_benchmark_may_not_have_a_known_issue() {
    let source = r"#= {result: 1, bench: true, known_issue: 'FROST-1'}
1
";
    assert_mentions(
        &malformation("benchmarks/area/good/script", source),
        "a benchmark may not have a `known_issue`",
    );
}

#[test]
fn bench_is_true_or_a_map_of_bindable_inputs() {
    for (bench, fragment) in [
        (
            "false",
            "`bench` is `true` or `{inputs: {...}}`; leave it out",
        ),
        ("{inputs: {['1x']: 1}}", "not a name a script can bind"),
        ("'yes'", "`true` or `{inputs: {...}}`"),
        ("{n: 1}", "`true` or `{inputs: {...}}`"),
        // A Map with any field besides `inputs`, such as a misspelling.
        ("{inputs: {n: 1}, warmup: 3}", "`true` or `{inputs: {...}}`"),
    ] {
        let reason = malformation(
            "benchmarks/area/good/script",
            &format!("#= {{result: 1, bench: {bench}}}\n1\n"),
        );
        assert_mentions(&reason, fragment);
        assert!(
            !reason.contains("BenchField"),
            "no Rust type name reaches the script author: {reason}"
        );
    }
}

#[test]
fn bench_inputs_may_hold_any_value_inputs_may() {
    // A Function among them, as `inputs` may hold one.
    let source = r"#= {
#=     result: 2,
#=     inputs: {f: fn x -> x + 1},
#=     bench: {inputs: {f: fn x -> x * 2}},
#= }
def {f} = import('inputs')
f(1)
";
    assert_eq!(
        report("benchmarks/area/good/script", source),
        Report::Passed
    );
}

#[test]
fn a_null_bench_is_malformed() {
    // As with the other optional fields, Null is given, not left out.
    let source = r"#= {result: 1, bench: null}
1
";
    assert_eq!(
        report("area/good/script", source),
        Report::Malformed("`bench` is `true` or `{inputs: {...}}`, not Null".to_string())
    );
}

#[test]
fn a_null_optional_field_is_malformed() {
    let source = r"#= {result: 1, known_issue: null}
1
";
    assert_eq!(
        report("area/good/script", source),
        Report::Malformed(
            "the expectation is malformed: expected String, got Null (at `known_issue`)"
                .to_string()
        )
    );
}

#[test]
fn a_null_result_is_a_result() {
    let source = r"#= {result: null}
null
";
    assert_eq!(report("area/good/script", source), Report::Passed);
}

#[test]
fn benchmarks_are_the_marked_scripts_under_benchmarks() {
    let names: Vec<String> = Benchmark::discover(&fixture("bench"))
        .expect("the fixtures are readable")
        .into_iter()
        .map(|benchmark| benchmark.entry.name)
        .collect();
    assert_eq!(
        names,
        ["benchmarks/area/good/marked", "benchmarks/area/good/scaled"],
        "only marked scripts under `benchmarks/`, in name order"
    );
}

#[test]
fn a_benchmark_runs_with_its_bench_inputs() {
    let benchmarks = prepare(&fixture("bench"), &option_sets(None).unwrap())
        .expect("the fixture benchmarks pass");
    let results: Vec<Value> = benchmarks
        .iter()
        .map(|benchmark| {
            let closure = benchmark.closure(Default::default()).expect("it compiles");
            benchmark.run(&closure).expect("it completes")
        })
        .collect();
    assert_eq!(
        results,
        [Value::Int(6), Value::Int(5050)],
        "`bench: true` keeps the script's inputs; `bench: {{inputs}}` replaces them"
    );
}

#[test]
fn a_benchmark_id_is_its_directory_then_its_file() {
    let benchmarks = Benchmark::discover(&fixture("bench")).expect("the fixtures are readable");
    let parts: Vec<(&str, &str)> = benchmarks
        .iter()
        .map(|benchmark| (benchmark.group(), benchmark.file()))
        .collect();
    assert_eq!(
        parts,
        [
            ("benchmarks/area/good", "marked"),
            ("benchmarks/area/good", "scaled")
        ]
    );
}

#[test]
fn prepare_refuses_to_time_anything_while_a_benchmark_fails() {
    let sets = option_sets(None).unwrap();
    for (root, fragment) in [
        ("bench-gate/wrong-result", "expected 7, got 6"),
        (
            "bench-gate/raises",
            "with its bench inputs: Division by zero",
        ),
        // Malformed, so no benchmark, yet it blocks the run rather than
        // silently dropping out of the timings.
        ("bench-gate/malformed", "`true` or `{inputs: {...}}`"),
        ("bench-gate/long-name", "longer than 64 bytes"),
    ] {
        let Err(error) = prepare(&fixture(root), &sets) else {
            panic!("{root}: prepare should refuse");
        };
        assert_mentions(&error, "none is timed");
        assert_mentions(&error, fragment);
    }
}

#[test]
fn prepare_checks_every_selected_option_set() {
    // The long label fails only once the option sets include it.
    let long = "none, constant-fold=true, constant-propagate=true, branch-eliminate=true";
    let sets = option_sets(Some(long)).unwrap();
    let Err(error) = prepare(&fixture("bench"), &sets) else {
        panic!("a label this long makes an ID part too long");
    };
    assert_mentions(&error, "longer than 64 bytes");
    assert_eq!(MAX_ID_PART, 64, "criterion's directory-name limit");
}

#[test]
fn benchmarks_run_under_the_default_options_unless_asked() {
    for value in [None, Some(""), Some("   ")] {
        let sets = option_sets(value).unwrap();
        assert_eq!(labels(&sets), ["default"], "{value:?}");
        assert_eq!(sets[0].options, Default::default(), "{value:?}");
    }
}

#[test]
fn each_selects_default_none_and_the_defaults_without_each_optimization() {
    let sets = option_sets(Some("each")).unwrap();
    let mut expected = vec!["default".to_string(), "none".to_string()];
    expected.extend(
        Optimization::ALL
            .iter()
            .map(|optimization| format!("{}=false", optimization.name())),
    );
    assert_eq!(labels(&sets), expected);
    assert_eq!(
        sets[1].options.optimization_options,
        OptimizationOptions::NONE
    );
    for (optimization, set) in Optimization::ALL.iter().zip(&sets[2..]) {
        assert!(
            !set.options.optimization_options.get(*optimization),
            "{} is off",
            optimization.name()
        );
    }
}

#[test]
fn a_list_of_option_sets_applies_each_over_the_defaults() {
    let sets = option_sets(Some(" default ; constant-fold = false;none ")).unwrap();
    assert_eq!(
        labels(&sets),
        ["default", "constant-fold=false", "none"],
        "labels have their spacing normalized"
    );
    let without_fold = sets[1].options.optimization_options;
    assert!(!without_fold.get(Optimization::ConstantFold));
    assert!(
        without_fold.get(Optimization::ConstantPropagate),
        "the rest stay at their defaults"
    );
}

#[test]
fn a_malformed_list_of_option_sets_is_an_error() {
    for (value, fragment) in [
        ("default; folding=off", "folding"),
        ("default;", "an option set is empty"),
        ("default;;none", "an option set is empty"),
        ("each; none", "`each` must stand alone"),
        (
            "default; default",
            "`default` and `default` select the same options",
        ),
        (
            "constant-fold=false; constant-fold = false",
            "select the same options",
        ),
    ] {
        let error = option_sets(Some(value)).unwrap_err();
        assert_mentions(&error, "FROST_BENCH_OPTIONS");
        assert_mentions(&error, fragment);
    }
}

// -- Discovery and summary --

#[test]
fn discovery_names_scripts_by_path_and_skips_documentation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery");
    let names: Vec<String> = discover(&root)
        .expect("the fixture is readable")
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert_eq!(names, ["a/good/first", "b/nested/runtime-error/second"]);
}

#[test]
fn discovery_rejects_a_file_that_is_neither_script_nor_documentation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/misnamed");
    let error = discover(&root).expect_err("a misnamed script is an error");
    assert!(
        error.to_string().contains("script.frost"),
        "expected the error to name the file, but it reads: {error}"
    );
}

#[test]
fn an_unreadable_entry_is_malformed() {
    let entry = Entry {
        name: "area/good/absent".to_string(),
        path: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/absent.frst"),
    };
    let verdict = check(&entry, Mode::Default);
    assert_eq!(verdict.kind, Some(Kind::Good));
    assert!(
        matches!(&verdict.report, Report::Malformed(reason) if reason.contains("cannot read")),
        "expected an unreadable script to be malformed, got {:?}",
        verdict.report
    );
}

#[test]
fn the_tally_counts_each_verdict_in_its_bucket() {
    let scripts = [
        ("a/good/pass", "#= {result: 1}\n1\n"),
        ("a/good/fail", "#= {result: 1}\n2\n"),
        (
            "a/good/known",
            "#= {result: 1, known_issue: 'FROST-1', regression: true}\n2\n",
        ),
        (
            "a/good/resolved",
            "#= {result: 1, known_issue: 'FROST-2'}\n1\n",
        ),
        (
            "a/runtime-error/pass",
            "#= {raises: 'no match'}\nmatch 1 {\n    2 => 2,\n}\n",
        ),
        (
            "a/compile-error/pass",
            "#= {rejects: 'not defined'}\nmissing\n",
        ),
        ("a/compile-error/malformed", "1\n"),
        ("a/elsewhere", "#= {result: 1}\n1\n"),
    ];
    let mut tally = Tally::new(Mode::Default);
    for (name, source) in scripts {
        tally.record(name, &check_source(name, source, Mode::Default));
    }
    let expected = r"corpus summary (default optimizations)
                    pass    fail   panic   known issue  (regression)
good                   1       1       0             1             1
runtime-error          1       0       0             0             0
compile-error          1       0       0             0             0
malformed: 2
known issues now behaving as expected: 1
    a/good/resolved (FROST-2)";
    assert_eq!(tally.to_string(), expected);
}
