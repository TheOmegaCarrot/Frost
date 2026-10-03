//! `std.regex`, from Frost source.
//!
//! Each case runs with only `std.regex` installed, bound as `re`. Patterns are
//! written as raw strings, `R'(...)'`, so backslashes reach the regex engine as
//! written.

mod source;

use std::sync::Arc;

use frost_runtime::stdlib::RandomConfig;
use frost_runtime::{Importer, ImporterBuilder, Stdlib, stdlib};
use source::Script;

/// An importer providing only `std.regex`.
fn importer() -> Arc<Importer> {
    let stdlib = Stdlib::new()
        .with_module(stdlib::regex())
        .expect("a lone module is accepted");
    ImporterBuilder::new().with_stdlib(stdlib).build()
}

/// `expression`, run with `std.regex` bound as `re`.
fn script(expression: &str) -> Script {
    let source = format!(
        r"
        def re = import('std.regex')
        {expression}
        "
    );
    Script::new(&source).importer(importer())
}

/// Assert each `expression` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (expression, expected) in cases {
        assert_eq!(
            script(expression).run(),
            script(expected).run(),
            "{expression:?} is {expected}"
        );
    }
}

/// Assert each `expression` raises exactly `message`.
fn assert_raises(cases: &[(&str, &str)]) {
    for (expression, message) in cases {
        assert_eq!(script(expression).raises(), *message, "{expression:?}");
    }
}

/// Assert `function` raises its arity error when called with each count in
/// `counts`, where it takes exactly `arity` arguments.
fn assert_arity(function: &str, arity: usize, counts: &[usize]) {
    for &argc in counts {
        let expression = format!("re.{function}({})", vec!["null"; argc].join(", "));
        assert_raises(&[(
            &expression,
            &format!(
                "Function regex.{function} expects {arity} arguments, but was called with {argc}"
            ),
        )]);
    }
}

// --- The module ---

#[test]
fn the_module_holds_its_functions() {
    assert_values(&[(
        "sorted(keys(re))",
        "['compile', 'contains', 'matches', 'replace', 'replace_first', 'replace_with', \
         'scan_matches', 'split']",
    )]);
}

#[test]
fn the_module_is_contained() {
    let stdlib = Stdlib::contained(RandomConfig::default());
    let contained = ImporterBuilder::new().with_stdlib(stdlib).build();
    let result = Script::new("import('std.regex').contains('abc', 'b')")
        .importer(contained)
        .run();
    assert_eq!(result, frost_runtime::Value::Bool(true));
}

// --- matches, contains ---

#[test]
fn matches_requires_the_whole_text_to_match() {
    assert_values(&[
        ("re.matches('abc', 'a.c')", "true"),
        ("re.matches('abcd', 'a.c')", "false"),
        ("re.matches('xabc', 'a.c')", "false"),
        // Some alternative must span the text, not merely the first found.
        ("re.matches('ab', 'a|ab')", "true"),
        ("re.matches('xab', 'a|ab')", "false"),
        ("re.matches('', '')", "true"),
        ("re.matches('', 'a*')", "true"),
        ("re.matches('A', '(?i)a')", "true"),
        (r"re.matches('2024', R'(\d{4})')", "true"),
    ]);
}

#[test]
fn matches_rejects_a_pattern_that_is_invalid_alone() {
    // Wrapped to anchor it, this would become a valid pattern of another meaning.
    let raised = script("re.matches('b', 'a)|(?:b')").raises();
    assert!(
        raised.starts_with(r#"Function regex.matches got an invalid pattern "a)|(?:b": "#),
        "{raised}"
    );
}

#[test]
fn contains_finds_a_match_anywhere() {
    assert_values(&[
        ("re.contains('hello world', 'o w')", "true"),
        ("re.contains('hello', '^h')", "true"),
        ("re.contains('hello', 'x')", "false"),
        ("re.contains('', '')", "true"),
        (r"re.contains('caf\u{e9}', '\u{e9}$')", "true"),
        // `.` matches a whole code point.
        (r"re.contains('\u{e9}', '^.$')", "true"),
        // Character classes such as `\w` are ASCII only.
        (r"re.contains('\u{e9}', R'(\w)')", "false"),
    ]);
}

// --- replace, replace_first, replace_with ---

#[test]
fn replace_replaces_every_match() {
    assert_values(&[
        (r"re.replace('a1b22c', R'(\d+)', '#')", "'a#b#c'"),
        ("re.replace('abc', 'x', '#')", "'abc'"),
        ("re.replace('abc', '', '-')", "'-a-b-c-'"),
        (r"re.replace_first('a1b2', R'(\d)', '#')", "'a#b2'"),
        ("re.replace_first('abc', 'x', '#')", "'abc'"),
    ]);
}

#[test]
fn a_replacement_refers_to_groups() {
    assert_values(&[
        (
            r"re.replace('john smith', R'((\w+) (\w+))', '$2 $1')",
            "'smith john'",
        ),
        (
            r"re.replace('2024-03', R'((?P<y>\d+)-(?P<m>\d+))', '${m}/${y}')",
            "'03/2024'",
        ),
        ("re.replace('ab', 'b', '[$0]')", "'a[b]'"),
        ("re.replace('a', 'a', '$$')", "'$'"),
        ("re.replace('ab', '(a)', '${1}x')", "'axb'"),
        // `$1x` names a group `1x`, which does not exist: it is empty.
        ("re.replace('ab', '(a)', '$1x')", "'b'"),
    ]);
}

#[test]
fn replace_with_replaces_each_match_by_a_function() {
    assert_values(&[
        (
            r"re.replace_with('hello world', R'(\w+)', to_upper)",
            "'HELLO WORLD'",
        ),
        (
            r"re.replace_with('a1b2', R'(\d)', fn d -> to_int(d) * 10)",
            "'a10b20'",
        ),
        ("re.replace_with('abc', 'x', fn m -> 'never')", "'abc'"),
        ("re.replace_with('ab', '', fn m -> '-')", "'-a-b-'"),
        // The result is converted as `to_string` does.
        (
            "re.replace_with('a', 'a', fn m -> [m])",
            r#"to_string(['a'])"#,
        ),
    ]);
}

#[test]
fn replace_with_calls_its_function_in_order_until_one_raises() {
    let probe = r"fn m -> { print(m); if m == '2': error('boom') else: m }";
    let script = script(&format!(r"re.replace_with('1 2 3', R'(\d)', {probe})"));
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1", "2"]);
}

// --- split ---

#[test]
fn split_splits_around_each_match() {
    assert_values(&[
        ("re.split('one,two,three', ',')", "['one', 'two', 'three']"),
        ("re.split('a,,b,,,c', ',+')", "['a', 'b', 'c']"),
        ("re.split('hello', 'x')", "['hello']"),
        // Every match splits, so empty pieces are kept, as the global `split` keeps them.
        (r"re.split('a1b2c3', R'(\d)')", "['a', 'b', 'c', '']"),
        ("re.split(',a', ',')", "['', 'a']"),
        ("re.split('', ',')", "['']"),
    ]);
}

// --- scan_matches ---

#[test]
fn scan_matches_describes_every_match() {
    assert_values(&[
        (
            r"re.scan_matches('a1b22', R'(\d+)')",
            r"{
                found: true,
                count: 2,
                matches: [
                    {full: '1', groups: [{matched: true, value: '1', index: 0}], named: {}},
                    {full: '22', groups: [{matched: true, value: '22', index: 0}], named: {}},
                ],
            }",
        ),
        (
            "re.scan_matches('abc', 'x')",
            "{found: false, count: 0, matches: []}",
        ),
    ]);
}

#[test]
fn scan_matches_describes_each_group() {
    assert_values(&[
        (
            "re.scan_matches('a', '(a)|(b)').matches[0].groups",
            "[
                {matched: true, value: 'a', index: 0},
                {matched: true, value: 'a', index: 1},
                {matched: false, value: null, index: 2},
            ]",
        ),
        (
            r"re.scan_matches('k=v', R'((?P<key>\w)=(?P<val>\w))').matches[0].named",
            "{key: {matched: true, value: 'k'}, val: {matched: true, value: 'v'}}",
        ),
        (
            "re.scan_matches('a', '(?P<x>a)|(?P<y>b)').matches[0].named",
            "{x: {matched: true, value: 'a'}, y: {matched: false, value: null}}",
        ),
    ]);
}

// --- Arguments ---

#[test]
fn an_invalid_pattern_is_an_error() {
    for call in [
        "matches('a', '(')",
        "contains('a', '(')",
        "replace('a', '(', '')",
        "replace_first('a', '(', '')",
        "replace_with('a', '(', id)",
        "split('a', '(')",
        "scan_matches('a', '(')",
    ] {
        let name = call.split('(').next().expect("a call has a name");
        assert_raises(&[(
            &format!("re.{call}"),
            &format!(
                r#"Function regex.{name} got an invalid pattern "(": found open group without closing ')'"#
            ),
        )]);
    }
}

#[test]
fn every_function_checks_its_arguments() {
    for function in ["matches", "contains", "split", "scan_matches"] {
        assert_raises(&[
            (
                &format!("re.{function}(x'61', 'a')"),
                &format!("Function regex.{function} requires String as argument 1, got Bytes"),
            ),
            (
                &format!("re.{function}('a', 1)"),
                &format!(
                    "Function regex.{function} requires String as argument 2 (pattern), got Int"
                ),
            ),
        ]);
        assert_arity(function, 2, &[0, 1, 3]);
    }
    for function in ["replace", "replace_first"] {
        assert_raises(&[(
            &format!("re.{function}('a', 'a', 1)"),
            &format!(
                "Function regex.{function} requires String as argument 3 (replacement), got Int"
            ),
        )]);
        assert_arity(function, 3, &[0, 2, 4]);
    }
    assert_raises(&[(
        "re.replace_with('a', 'a', 'b')",
        "Function regex.replace_with requires Function as argument 3 (callback), got String",
    )]);
    assert_arity("replace_with", 3, &[0, 2, 4]);
}

// --- compile ---

/// The functions of a compiled pattern, by name.
const COMPILED: [&str; 7] = [
    "contains",
    "matches",
    "replace",
    "replace_first",
    "replace_with",
    "scan_matches",
    "split",
];

#[test]
fn compile_returns_the_functions_less_the_pattern() {
    let names: Vec<String> = COMPILED.iter().map(|name| format!("'{name}'")).collect();
    assert_values(&[(
        "sorted(keys(re.compile('a')))",
        &format!("[{}]", names.join(", ")),
    )]);
}

#[test]
fn a_compiled_pattern_does_what_the_module_functions_do() {
    let source = r"
        def pattern = R'((\d+))'
        def p = re.compile(pattern)
        def compiled = [
            p.matches('12'),
            p.matches('a12'),
            p.contains('a12'),
            p.replace('a1b22', '<$1>'),
            p.replace_first('a1b22', '#'),
            p.replace_with('a1b22', fn m -> len(m)),
            p.split('a1b22c'),
            p.scan_matches('a1b22'),
        ]
        def direct = [
            re.matches('12', pattern),
            re.matches('a12', pattern),
            re.contains('a12', pattern),
            re.replace('a1b22', pattern, '<$1>'),
            re.replace_first('a1b22', pattern, '#'),
            re.replace_with('a1b22', pattern, fn m -> len(m)),
            re.split('a1b22c', pattern),
            re.scan_matches('a1b22', pattern),
        ]
        [compiled == direct, compiled]
    ";
    assert_values(&[(
        source,
        r"[true, [
            true,
            false,
            true,
            'a<1>b<22>',
            'a#b22',
            'a1b2',
            ['a', 'b', 'c'],
            re.scan_matches('a1b22', R'((\d+))'),
        ]]",
    )]);
}

#[test]
fn a_compiled_pattern_is_reusable() {
    let source = r"
        def p = re.compile('a|ab')
        [p.matches('ab'), p.matches('a'), p.matches('b'), p.contains('xab'), p.matches('ab')]
    ";
    assert_values(&[(source, "[true, true, false, true, true]")]);
}

#[test]
fn compile_rejects_an_invalid_pattern() {
    assert_raises(&[(
        "re.compile('(')",
        r#"Function regex.compile got an invalid pattern "(": found open group without closing ')'"#,
    )]);
    // Valid only if wrapped, as `matches` wraps it: still invalid.
    let raised = script("re.compile('a)|(?:b')").raises();
    assert!(
        raised.starts_with(r#"Function regex.compile got an invalid pattern "a)|(?:b": "#),
        "{raised}"
    );
}

#[test]
fn a_compiled_replace_with_stops_at_the_first_raise() {
    let source = r"
        def p = re.compile(R'(\d)')
        p.replace_with('1 2 3', fn m -> { print(m); if m == '2': error('boom') else: m })
    ";
    let script = script(source);
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1", "2"]);
}

#[test]
fn compiled_functions_check_their_arguments() {
    for function in ["matches", "contains", "split", "scan_matches"] {
        assert_raises(&[
            (
                &format!("re.compile('a').{function}(1)"),
                &format!(
                    "Function compiled_regex.{function} requires String as argument 1, got Int"
                ),
            ),
            (
                &format!("re.compile('a').{function}()"),
                &format!(
                    "Function compiled_regex.{function} expects 1 arguments, \
                     but was called with 0"
                ),
            ),
        ]);
    }
    for function in ["replace", "replace_first"] {
        assert_raises(&[
            (
                &format!("re.compile('a').{function}('a', 1)"),
                &format!(
                    "Function compiled_regex.{function} requires String as argument 2 \
                     (replacement), got Int"
                ),
            ),
            (
                &format!("re.compile('a').{function}('a')"),
                &format!(
                    "Function compiled_regex.{function} expects 2 arguments, \
                     but was called with 1"
                ),
            ),
        ]);
    }
    assert_raises(&[
        (
            "re.compile('a').replace_with('a', 'b')",
            "Function compiled_regex.replace_with requires Function as argument 2 (callback), \
             got String",
        ),
        (
            "re.compile('a').replace_with('a')",
            "Function compiled_regex.replace_with expects 2 arguments, but was called with 1",
        ),
        (
            "re.compile(1)",
            "Function regex.compile requires String as argument 1 (pattern), got Int",
        ),
    ]);
    assert_arity("compile", 1, &[0, 2]);
}
