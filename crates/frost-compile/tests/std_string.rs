//! `std.string`, from Frost source.
//!
//! Each case runs with only `std.string` installed, bound as `str`. Positions
//! and widths count code points. The searching functions and `is_empty` also
//! take Bytes, in any mix with a String; where Bytes are involved, positions
//! count bytes.

mod common;

use std::sync::Arc;

use common::Script;
use frost_runtime::{Importer, ImporterBuilder, Stdlib, stdlib};

/// An importer providing only `std.string`.
fn importer() -> Arc<Importer> {
    let stdlib = Stdlib::new()
        .with_module(stdlib::string())
        .expect("a lone module is accepted");
    ImporterBuilder::new().with_stdlib(stdlib).build()
}

/// `expression`, run with `std.string` bound as `str`.
fn script(expression: &str) -> Script {
    Script::new(&format!("def str = import('std.string')\n{expression}")).importer(importer())
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
/// `counts`, where `expects` is how the error states its arity.
fn assert_arity(function: &str, expects: &str, counts: &[usize]) {
    for &argc in counts {
        let expression = format!("str.{function}({})", vec!["null"; argc].join(", "));
        assert_raises(&[(
            &expression,
            &format!(
                "Function string.{function} expects {expects} arguments, but was called with {argc}"
            ),
        )]);
    }
}

const CLASSIFIERS: [&str; 7] = [
    "is_ascii",
    "is_digit",
    "is_alpha",
    "is_alphanumeric",
    "is_whitespace",
    "is_uppercase",
    "is_lowercase",
];

// --- The module ---

#[test]
fn the_module_holds_its_functions() {
    assert_values(&[(
        "sorted(keys(str))",
        "['center', 'chars', 'count', 'index_of', 'is_alpha', 'is_alphanumeric', 'is_ascii', \
         'is_digit', 'is_empty', 'is_lowercase', 'is_uppercase', 'is_whitespace', \
         'last_index_of', 'pad_left', 'pad_right']",
    )]);
}

#[test]
fn the_module_is_pure() {
    let pure = ImporterBuilder::new().with_stdlib(Stdlib::pure()).build();
    let result = Script::new("import('std.string').count('banana', 'a')")
        .importer(pure)
        .run();
    assert_eq!(result, frost_runtime::Value::Int(3));
}

// --- index_of, last_index_of, count ---

#[test]
fn index_of_finds_the_first_occurrence() {
    assert_values(&[
        ("str.index_of('hello world', 'world')", "6"),
        ("str.index_of('abcabc', 'bc')", "1"),
        ("str.index_of('hello', 'xyz')", "null"),
        ("str.index_of('ab', 'abc')", "null"),
        ("str.index_of('abc', '')", "0"),
        ("str.index_of('', '')", "0"),
        // Code points, not bytes.
        (r"str.index_of('h\u{e9}llo', 'l')", "2"),
        (r"str.index_of('\u{1f600}x', 'x')", "1"),
    ]);
}

#[test]
fn last_index_of_finds_the_last_occurrence() {
    assert_values(&[
        ("str.last_index_of('abcabc', 'bc')", "4"),
        ("str.last_index_of('abc', 'x')", "null"),
        ("str.last_index_of('abc', '')", "3"),
        (r"str.last_index_of('h\u{e9}l\u{e9}', '\u{e9}')", "3"),
    ]);
}

#[test]
fn searches_in_bytes_count_bytes() {
    assert_values(&[
        ("str.index_of(x'00ff00', x'00')", "0"),
        ("str.last_index_of(x'00ff00', x'00')", "2"),
        ("str.index_of(x'00ff', x'aa')", "null"),
        ("str.last_index_of(x'0102', x'')", "2"),
        // Mixed, positions count the String's bytes.
        (r"str.index_of('h\u{e9}llo', x'6c')", "3"),
        ("str.index_of(x'68c3a96c', 'l')", "3"),
        (r"str.last_index_of('\u{e9}\u{e9}', x'c3')", "2"),
    ]);
}

#[test]
fn count_counts_occurrences_that_do_not_overlap() {
    assert_values(&[
        ("str.count('banana', 'an')", "2"),
        ("str.count('aaa', 'aa')", "1"),
        ("str.count('aaaa', 'aa')", "2"),
        ("str.count('', 'a')", "0"),
        ("str.count('abc', 'd')", "0"),
        (r"str.count('\u{e9}x\u{e9}', '\u{e9}')", "2"),
        ("str.count(x'000000', x'00')", "3"),
        ("str.count('aXa', x'61')", "2"),
    ]);
    for needle in ["''", "x''"] {
        assert_raises(&[(
            &format!("str.count('abc', {needle})"),
            "Function string.count requires argument 2 to be non-empty",
        )]);
    }
}

#[test]
fn is_empty_tests_for_no_content() {
    assert_values(&[
        ("str.is_empty('')", "true"),
        ("str.is_empty(' ')", "false"),
        ("str.is_empty(x'')", "true"),
        ("str.is_empty(x'00')", "false"),
    ]);
}

#[test]
fn searches_check_their_arguments() {
    for function in ["index_of", "last_index_of", "count"] {
        assert_raises(&[
            (
                &format!("str.{function}(1, 'a')"),
                &format!(
                    "Function string.{function} requires String or Bytes as argument 1, got Int"
                ),
            ),
            (
                &format!("str.{function}('a', ['a'])"),
                &format!(
                    "Function string.{function} requires String or Bytes as argument 2, got Array"
                ),
            ),
        ]);
        assert_arity(function, "2", &[0, 1, 3]);
    }
    assert_raises(&[(
        "str.is_empty([])",
        "Function string.is_empty requires String or Bytes as argument 1, got Array",
    )]);
    assert_arity("is_empty", "1", &[0, 2]);
}

// --- chars ---

#[test]
fn chars_splits_into_code_points() {
    assert_values(&[
        ("str.chars('abc')", "['a', 'b', 'c']"),
        ("str.chars('')", "[]"),
        (r"str.chars('\u{e9}\u{1f600}')", r"['\u{e9}', '\u{1f600}']"),
        // A combining accent is a code point of its own.
        (r"str.chars('e\u{301}')", r"['e', '\u{301}']"),
    ]);
}

// --- Classifiers ---

#[test]
fn every_classifier_accepts_the_empty_string() {
    for function in CLASSIFIERS {
        assert_values(&[(&format!("str.{function}('')"), "true")]);
    }
}

#[test]
fn is_ascii_and_is_digit_are_ascii_tests() {
    assert_values(&[
        ("str.is_ascii('abc ~')", "true"),
        (r"str.is_ascii('a\u{7f}')", "true"),
        (r"str.is_ascii('\u{e9}')", "false"),
        ("str.is_digit('0123456789')", "true"),
        ("str.is_digit('12a')", "false"),
        ("str.is_digit(' 1')", "false"),
        // Other scripts' digits and other numerals are not ASCII digits.
        (r"str.is_digit('\u{663}')", "false"),
        (r"str.is_digit('\u{bd}')", "false"),
    ]);
}

#[test]
fn is_alpha_is_alphanumeric_and_is_whitespace_are_unicode_aware() {
    assert_values(&[
        ("str.is_alpha('abcXYZ')", "true"),
        (r"str.is_alpha('\u{e9}\u{3b1}')", "true"),
        ("str.is_alpha('ab1')", "false"),
        ("str.is_alpha('a b')", "false"),
        ("str.is_alphanumeric('abc123')", "true"),
        (r"str.is_alphanumeric('\u{e9}\u{663}')", "true"),
        ("str.is_alphanumeric('a_b')", "false"),
        (r"str.is_whitespace(' \t\n')", "true"),
        (r"str.is_whitespace('\u{a0}\u{3000}')", "true"),
        ("str.is_whitespace(' a ')", "false"),
    ]);
}

#[test]
fn case_tests_ignore_characters_without_case() {
    assert_values(&[
        ("str.is_uppercase('ABC')", "true"),
        ("str.is_uppercase('ABC 123!')", "true"),
        ("str.is_uppercase('AbC')", "false"),
        ("str.is_uppercase('123')", "true"),
        (r"str.is_uppercase('\u{c9}')", "true"),
        (r"str.is_uppercase('\u{e9}')", "false"),
        ("str.is_lowercase('abc')", "true"),
        ("str.is_lowercase('abc 123!')", "true"),
        ("str.is_lowercase('aBc')", "false"),
        ("str.is_lowercase('123')", "true"),
        (r"str.is_lowercase('\u{e9}')", "true"),
        (r"str.is_lowercase('\u{c9}')", "false"),
    ]);
}

#[test]
fn text_functions_take_only_a_string() {
    for function in CLASSIFIERS.iter().chain(&["chars"]) {
        assert_raises(&[(
            &format!("str.{function}(x'61')"),
            &format!("Function string.{function} requires String as argument 1, got Bytes"),
        )]);
        assert_arity(function, "1", &[0, 2]);
    }
}

// --- pad_left, pad_right, center ---

#[test]
fn padding_fills_to_a_width() {
    assert_values(&[
        ("str.pad_left('42', 5, '0')", "'00042'"),
        ("str.pad_left('42', 5)", "'   42'"),
        ("str.pad_right('hi', 5, '.')", "'hi...'"),
        ("str.pad_right('hi', 5)", "'hi   '"),
        ("str.center('hi', 6, '-')", "'--hi--'"),
        // The extra character goes on the right.
        ("str.center('hi', 5, '-')", "'-hi--'"),
        ("str.center('hi', 3)", "'hi '"),
        ("str.center('', 3, '*')", "'***'"),
    ]);
}

#[test]
fn padding_leaves_a_string_already_wide_enough() {
    for function in ["pad_left", "pad_right", "center"] {
        assert_values(&[
            (&format!("str.{function}('hello', 3)"), "'hello'"),
            (&format!("str.{function}('42', 2, '0')"), "'42'"),
            (&format!("str.{function}('42', 0)"), "'42'"),
        ]);
    }
}

#[test]
fn padding_counts_code_points() {
    assert_values(&[
        (r"str.pad_left('\u{e9}', 3, '*')", r"'**\u{e9}'"),
        (r"str.pad_right('x', 3, '\u{e9}')", r"'x\u{e9}\u{e9}'"),
        (
            r"str.center('\u{1f600}', 3, '\u{b7}')",
            r"'\u{b7}\u{1f600}\u{b7}'",
        ),
    ]);
}

#[test]
fn padding_checks_its_arguments() {
    for function in ["pad_left", "pad_right", "center"] {
        assert_raises(&[
            (
                &format!("str.{function}('a', -1)"),
                &format!(
                    "Function string.{function} requires argument 2 (width) to be at least 0, \
                     got -1"
                ),
            ),
            (
                &format!("str.{function}('a', 3, '')"),
                &format!(
                    "Function string.{function} requires a single character as argument 3 \
                     (fill), got \"\""
                ),
            ),
            (
                &format!("str.{function}('a', 3, 'ab')"),
                &format!(
                    "Function string.{function} requires a single character as argument 3 \
                     (fill), got \"ab\""
                ),
            ),
            (
                &format!("str.{function}(x'61', 3)"),
                &format!("Function string.{function} requires String as argument 1, got Bytes"),
            ),
            (
                &format!("str.{function}('a', 3.0)"),
                &format!(
                    "Function string.{function} requires Int as argument 2 (width), got Float"
                ),
            ),
            (
                &format!("str.{function}('a', 3, 0)"),
                &format!(
                    "Function string.{function} requires String as argument 3 (fill), got Int"
                ),
            ),
        ]);
        assert_arity(function, "between 2 and 3", &[0, 1, 4]);
    }
}
