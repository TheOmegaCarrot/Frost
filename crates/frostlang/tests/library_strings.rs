//! The Strings globals, from Frost source.
//!
//! `split`, `split_once`, `join`, `replace`, `contains`, `starts_with`, and
//! `ends_with` accept String and Bytes in any mix. Mixed, they work on a String's
//! UTF-8 bytes, and a result is a String only when every content argument is a
//! String, else Bytes.
//! `lines`, `trim`, `trim_left`, `trim_right`, `to_upper`, and `to_lower` accept
//! only a String.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over literals is checked both folded and at run time. Cases that capture their
//! input are never folded.

use crate::script;

use frostlang::Value;
use script::assertions::{Library, library_assertions};
use script::{Script, raises, run};

library_assertions!(Library::GLOBALS);

/// The type error `function` raises for an argument of type `got`, where it
/// requires `requires` as `position` (such as `argument 2 (delimiter)`).
fn type_error(function: &str, requires: &str, position: &str, got: &str) -> String {
    format!("Function {function} requires {requires} as {position}, got {got}")
}

/// Assert each `(source, position, got)` raises `function`'s type error for a
/// non-Flat argument.
fn assert_rejects_non_flat(function: &str, cases: &[(&str, &str, &str)]) {
    for (source, position, got) in cases {
        assert_eq!(
            raises(source),
            type_error(function, "String or Bytes", position, got),
            "{source:?}"
        );
    }
}

/// Assert `function`, applied to each non-String argument, raises its type error.
fn assert_rejects_non_string(function: &str) {
    for (argument, got) in [
        ("x'61'", "Bytes"),
        ("null", "Null"),
        ("1", "Int"),
        ("['a']", "Array"),
        ("{a: 'a'}", "Map"),
    ] {
        let source = format!("{function}({argument})");
        assert_eq!(
            raises(&source),
            type_error(function, "String", "argument 1", got),
            "{source:?}"
        );
    }
}

// --- split ---

#[test]
fn split_splits_a_string_on_a_delimiter() {
    assert_values(&[
        ("split('a,b,c', ',')", "['a', 'b', 'c']"),
        ("split('a::b::c', '::')", "['a', 'b', 'c']"),
        (r"split('a\u{e9}b', '\u{e9}')", "['a', 'b']"),
        // Not found: the whole string, alone.
        ("split('abc', ',')", "['abc']"),
        ("split('', ',')", "['']"),
        // Every delimiter splits, so empty pieces are kept.
        ("split(',a,,b,', ',')", "['', 'a', '', 'b', '']"),
        ("split('abc', 'abc')", "['', '']"),
        // Occurrences do not overlap, and are found left to right.
        ("split('aaa', 'aa')", "['', 'a']"),
        ("split('aab', 'ab')", "['a', '']"),
    ]);
}

#[test]
fn split_on_an_empty_string_splits_into_characters() {
    assert_values(&[
        ("split('abc', '')", "['a', 'b', 'c']"),
        (
            r"split('\u{e9}\u{1f600}!', '')",
            r"['\u{e9}', '\u{1f600}', '!']",
        ),
        ("split('', '')", "[]"),
    ]);
}

#[test]
fn split_splits_bytes() {
    assert_values(&[
        ("split(x'610062', x'00')", "[x'61', x'62']"),
        ("split(x'61000062', x'0000')", "[x'61', x'62']"),
        ("split(x'6162', x'00')", "[x'6162']"),
        ("split(x'', x'00')", "[x'']"),
        ("split(x'00', x'00')", "[x'', x'']"),
        ("split(x'000000', x'0000')", "[x'', x'00']"),
        // An empty delimiter splits into single bytes.
        ("split(x'ff00c3', x'')", "[x'ff', x'00', x'c3']"),
        ("split(x'', x'')", "[]"),
    ]);
}

#[test]
fn split_with_any_bytes_argument_splits_into_bytes() {
    assert_values(&[
        ("split('a,b', x'2c')", "[x'61', x'62']"),
        ("split(x'612c62', ',')", "[x'61', x'62']"),
        ("split('abc', x'00')", "[x'616263']"),
        // An empty delimiter splits a String's bytes, not its characters.
        (r"split('\u{e9}', x'')", "[x'c3', x'a9']"),
        // A byte-level split may cut a character apart.
        (r"split('\u{e9}', x'a9')", "[x'c3', x'']"),
    ]);
}

#[test]
fn split_splits_runtime_values() {
    let split = |text: Value, delimiter: Value| {
        Script::new("split(t, d)")
            .capture("t", text)
            .capture("d", delimiter)
            .run()
    };
    assert_eq!(
        split(Value::from("x-y"), Value::from("-")),
        Value::array(["x", "y"])
    );
    assert_eq!(
        split(Value::from("x-y"), Value::from(&b"-"[..])),
        Value::array([&b"x"[..], &b"y"[..]])
    );
}

#[test]
fn split_checks_its_argument_types() {
    assert_rejects_non_flat(
        "split",
        &[
            ("split(1, ',')", "argument 1", "Int"),
            ("split(null, ',')", "argument 1", "Null"),
            ("split(['a,b'], ',')", "argument 1", "Array"),
            ("split('a,b', 44)", "argument 2 (delimiter)", "Int"),
            ("split('a,b', null)", "argument 2 (delimiter)", "Null"),
            ("split('a,b', [','])", "argument 2 (delimiter)", "Array"),
        ],
    );
}

#[test]
fn split_takes_exactly_two_arguments() {
    assert_arity("split", 2, &[0, 1, 3]);
}

// --- split_once ---

#[test]
fn split_once_splits_a_string_at_the_first_delimiter() {
    assert_values(&[
        ("split_once('key=a=b', '=')", "['key', 'a=b']"),
        ("split_once('a::b::c', '::')", "['a', 'b::c']"),
        (
            r"split_once('a\u{e9}b\u{e9}c', '\u{e9}')",
            r"['a', 'b\u{e9}c']",
        ),
        // A delimiter at either edge leaves an empty piece.
        ("split_once('=a', '=')", "['', 'a']"),
        ("split_once('a=', '=')", "['a', '']"),
        ("split_once('abc', 'abc')", "['', '']"),
        // Occurrences are found left to right, and the rest is never searched.
        ("split_once('aaa', 'aa')", "['', 'a']"),
        ("split_once('aab', 'ab')", "['a', '']"),
    ]);
}

#[test]
fn split_once_without_the_delimiter_is_null() {
    assert_values(&[
        ("split_once('abc', ',')", "null"),
        ("split_once('', ',')", "null"),
        ("split_once('ab', 'abc')", "null"),
        ("split_once(x'6162', x'00')", "null"),
        ("split_once(x'', x'00')", "null"),
        ("split_once('abc', x'00')", "null"),
    ]);
}

#[test]
fn split_once_on_an_empty_string_splits_off_the_first_character() {
    assert_values(&[
        ("split_once('abc', '')", "['a', 'bc']"),
        (
            r"split_once('\u{e9}\u{1f600}!', '')",
            r"['\u{e9}', '\u{1f600}!']",
        ),
        (r"split_once('\u{1f600}a', '')", r"['\u{1f600}', 'a']"),
        // With fewer than two characters there is nothing to split.
        ("split_once('a', '')", "null"),
        (r"split_once('\u{1f600}', '')", "null"),
        ("split_once('', '')", "null"),
    ]);
}

#[test]
fn split_once_splits_bytes() {
    assert_values(&[
        ("split_once(x'61006200', x'00')", "[x'61', x'6200']"),
        ("split_once(x'0000', x'00')", "[x'', x'00']"),
        ("split_once(x'000000', x'0000')", "[x'', x'00']"),
        // An empty delimiter splits off the first byte.
        ("split_once(x'ff00c3', x'')", "[x'ff', x'00c3']"),
        ("split_once(x'ff', x'')", "null"),
        ("split_once(x'', x'')", "null"),
    ]);
}

#[test]
fn split_once_with_any_bytes_argument_splits_into_bytes() {
    assert_values(&[
        ("split_once('a,b,c', x'2c')", "[x'61', x'622c63']"),
        ("split_once(x'612c62', ',')", "[x'61', x'62']"),
        // An empty delimiter splits off a String's first byte, not its first character.
        (r"split_once('\u{e9}', x'')", "[x'c3', x'a9']"),
        // A byte-level split may cut a character apart.
        (r"split_once('\u{e9}', x'a9')", "[x'c3', x'']"),
    ]);
}

#[test]
fn split_once_agrees_with_split() {
    // `split_once` gives the first piece of `split` and the rest rejoined, or Null
    // where `split` gives fewer than two pieces. Any case breaking that is returned.
    let disagreements = run(r#"
        defn expected(s, d) -> {
            def pieces = split(s, d)
            if len(pieces) < 2: null
            else: [pieces[0], join(drop(pieces, 1), d)]
        }
        def cases = [
            ['a,b,c', ','], [',a,,b,', ','], ['abc', ','], ['', ','],
            ['aaaa', 'aa'], ['abc', ''], ['a', ''], ['', ''], ['\u{e9}\u{1f600}', ''],
            [x'610062', x'00'], [x'ff00c3', x''], [x'ff', x''], ['a,b', x'2c'],
        ]
        reject(cases, fn c -> split_once(c[0], c[1]) == expected(c[0], c[1]))
    "#);
    assert_eq!(disagreements, Value::array::<Value, 0>([]));
}

#[test]
fn split_once_inverts_with_join() {
    assert_values(&[
        ("join(split_once('k=v=w', '='), '=')", "'k=v=w'"),
        (r"join(split_once('\u{e9}x', ''), '')", r"'\u{e9}x'"),
        ("join(split_once(x'610062', x'00'), x'00')", "x'610062'"),
    ]);
}

#[test]
fn split_once_splits_runtime_values() {
    let split_once = |text: Value, delimiter: Value| {
        Script::new("split_once(t, d)")
            .capture("t", text)
            .capture("d", delimiter)
            .run()
    };
    assert_eq!(
        split_once(Value::from("x-y-z"), Value::from("-")),
        Value::array(["x", "y-z"])
    );
    assert_eq!(
        split_once(Value::from("x-y"), Value::from(&b"-"[..])),
        Value::array([&b"x"[..], &b"y"[..]])
    );
    assert_eq!(split_once(Value::from("xy"), Value::from("-")), Value::Null);
}

#[test]
fn split_once_checks_its_argument_types() {
    assert_rejects_non_flat(
        "split_once",
        &[
            ("split_once(1, ',')", "argument 1", "Int"),
            ("split_once(null, ',')", "argument 1", "Null"),
            ("split_once(['a,b'], ',')", "argument 1", "Array"),
            ("split_once({a: 'b'}, ',')", "argument 1", "Map"),
            ("split_once('a,b', 44)", "argument 2 (delimiter)", "Int"),
            ("split_once('a,b', null)", "argument 2 (delimiter)", "Null"),
            (
                "split_once('a,b', [','])",
                "argument 2 (delimiter)",
                "Array",
            ),
            ("split_once('a,b', true)", "argument 2 (delimiter)", "Bool"),
        ],
    );
}

#[test]
fn split_once_takes_exactly_two_arguments() {
    assert_arity("split_once", 2, &[0, 1, 3]);
}

// --- lines ---

#[test]
fn lines_splits_on_each_line_ending() {
    assert_values(&[
        (r"lines('a\nb\nc')", "['a', 'b', 'c']"),
        (r"lines('a\r\nb\r\nc')", "['a', 'b', 'c']"),
        (r"lines('a\r\nb\nc')", "['a', 'b', 'c']"),
        ("lines('abc')", "['abc']"),
        // Blank lines are kept.
        (r"lines('a\n\nb')", "['a', '', 'b']"),
        (r"lines('\n\n')", "['', '']"),
        (r"lines('\r\n')", "['']"),
    ]);
}

#[test]
fn lines_treats_a_final_line_ending_as_optional() {
    assert_values(&[
        (r"lines('a\n')", "['a']"),
        (r"lines('a\r\n')", "['a']"),
        (r"lines('a\nb\n')", "['a', 'b']"),
        (r"lines('a\nb\r\n')", "['a', 'b']"),
        ("lines('')", "[]"),
    ]);
}

#[test]
fn lines_ends_a_line_only_at_a_newline() {
    assert_values(&[
        // A carriage return not followed by a newline is ordinary text.
        (r"lines('a\rb')", r"['a\rb']"),
        (r"lines('a\r')", r"['a\r']"),
        (r"lines('a\r\rb\nc')", r"['a\r\rb', 'c']"),
        // Only the one carriage return right before a newline belongs to the ending.
        (r"lines('a\r\r\nb')", r"['a\r', 'b']"),
    ]);
}

#[test]
fn lines_splits_a_runtime_string() {
    let split = Script::new("lines(s)")
        .capture("s", Value::from("one\r\ntwo\n"))
        .run();
    assert_eq!(split, Value::array(["one", "two"]));
}

#[test]
fn lines_checks_its_argument_type() {
    assert_rejects_non_string("lines");
}

#[test]
fn lines_takes_exactly_one_argument() {
    assert_arity("lines", 1, &[0, 2]);
}

// --- join ---

#[test]
fn join_joins_strings_with_a_separator() {
    assert_values(&[
        ("join(['a', 'b', 'c'], ', ')", "'a, b, c'"),
        ("join(['a', 'b'], '')", "'ab'"),
        ("join(['a'], ',')", "'a'"),
        ("join([], ',')", "''"),
        ("join(['', ''], ',')", "','"),
        (
            r"join(['\u{e9}', '\u{1f600}'], '\u{b7}')",
            r"'\u{e9}\u{b7}\u{1f600}'",
        ),
    ]);
}

#[test]
fn join_joins_bytes() {
    assert_values(&[
        ("join([x'61', x'62'], x'00')", "x'610062'"),
        ("join([x'ff'], x'00')", "x'ff'"),
        ("join([], x'00')", "x''"),
        ("join([x'', x''], x'00')", "x'00'"),
    ]);
}

#[test]
fn join_with_any_bytes_argument_joins_into_bytes() {
    assert_values(&[
        ("join(['a', 'b'], x'2c')", "x'612c62'"),
        ("join(['a', x'62'], ',')", "x'612c62'"),
        ("join([x'61', 'b'], x'')", "x'6162'"),
        ("join(['a'], x'')", "x'61'"),
        ("join([], x'')", "x''"),
    ]);
}

#[test]
fn join_joins_runtime_values() {
    let joined = Script::new("join(parts, '/')")
        .capture("parts", Value::array(["usr", "local"]))
        .run();
    assert_eq!(joined, Value::from("usr/local"));
}

#[test]
fn join_requires_every_element_to_be_a_string_or_bytes() {
    let message = |got: &str| {
        format!(
            "Function join requires Array of String or Bytes as argument 1, got Array containing {got}"
        )
    };
    for (source, got) in [
        ("join(['a', 1], ',')", "Int"),
        ("join([null], ',')", "Null"),
        ("join(['a', ['b']], ',')", "Array"),
        ("join([{a: 'a'}, 'b'], ',')", "Map"),
        // The first element that is neither is the one reported.
        ("join(['a', true, 1], ',')", "Bool"),
    ] {
        assert_eq!(raises(source), message(got), "{source:?}");
    }
}

#[test]
fn join_checks_its_argument_types() {
    assert_raises(&[(
        "join('abc', ',')",
        &type_error("join", "Array", "argument 1", "String"),
    )]);
    assert_raises(&[(
        "join(x'61', ',')",
        &type_error("join", "Array", "argument 1", "Bytes"),
    )]);
    assert_rejects_non_flat(
        "join",
        &[
            ("join(['a'], 44)", "argument 2 (separator)", "Int"),
            ("join(['a'], null)", "argument 2 (separator)", "Null"),
            ("join(['a'], [','])", "argument 2 (separator)", "Array"),
        ],
    );
}

#[test]
fn join_checks_its_separator_before_its_elements() {
    assert_raises(&[(
        "join([1], 2)",
        &type_error("join", "String or Bytes", "argument 2 (separator)", "Int"),
    )]);
}

#[test]
fn join_takes_exactly_two_arguments() {
    assert_arity("join", 2, &[0, 1, 3]);
}

#[test]
fn join_inverts_split() {
    assert_values(&[
        ("join(split('a,,b,', ','), ',')", "'a,,b,'"),
        (r"join(split('\u{e9}x', ''), '')", r"'\u{e9}x'"),
        ("join(split(x'610062', x'00'), x'00')", "x'610062'"),
    ]);
}

// --- replace ---

#[test]
fn replace_replaces_every_occurrence_in_a_string() {
    assert_values(&[
        ("replace('a-b-c', '-', '+')", "'a+b+c'"),
        ("replace('a::b', '::', '')", "'ab'"),
        ("replace('abc', 'abc', 'x')", "'x'"),
        (r"replace('caf\u{e9}', '\u{e9}', 'e')", "'cafe'"),
        // Not found: unchanged.
        ("replace('abc', 'z', 'x')", "'abc'"),
        ("replace('', 'z', 'x')", "''"),
        // Occurrences do not overlap, and are found left to right.
        ("replace('aaa', 'aa', 'b')", "'ba'"),
        // A replacement is never searched again.
        ("replace('a', 'a', 'aa')", "'aa'"),
    ]);
}

#[test]
fn replace_with_an_empty_find_changes_nothing() {
    assert_values(&[
        ("replace('abc', '', 'x')", "'abc'"),
        ("replace('', '', 'x')", "''"),
        ("replace(x'61', x'', x'00')", "x'61'"),
        // Unchanged content, but a Bytes argument still makes the result Bytes.
        ("replace('abc', '', x'00')", "x'616263'"),
        ("replace('abc', x'', 'x')", "x'616263'"),
    ]);
}

#[test]
fn replace_replaces_in_bytes() {
    assert_values(&[
        ("replace(x'610061', x'00', x'ffff')", "x'61ffff61'"),
        ("replace(x'000000', x'0000', x'01')", "x'0100'"),
        ("replace(x'6162', x'00', x'01')", "x'6162'"),
        ("replace(x'', x'00', x'01')", "x''"),
    ]);
}

#[test]
fn replace_with_any_bytes_argument_returns_bytes() {
    assert_values(&[
        ("replace('a-b', x'2d', '+')", "x'612b62'"),
        ("replace('a-b', '-', x'2b')", "x'612b62'"),
        ("replace(x'612d62', '-', '+')", "x'612b62'"),
        // The result need not be UTF-8.
        ("replace('a-b', '-', x'ff')", "x'61ff62'"),
        // A byte-level replace may cut a character apart.
        (r"replace('\u{e9}', x'a9', x'')", "x'c3'"),
    ]);
}

#[test]
fn replace_replaces_in_runtime_values() {
    let replaced = Script::new("replace(t, '.', '/')")
        .capture("t", Value::from("a.b.c"))
        .run();
    assert_eq!(replaced, Value::from("a/b/c"));
}

#[test]
fn replace_checks_its_argument_types() {
    assert_rejects_non_flat(
        "replace",
        &[
            ("replace(1, '-', '+')", "argument 1", "Int"),
            ("replace(null, '-', '+')", "argument 1", "Null"),
            ("replace('a', 1, '+')", "argument 2 (find)", "Int"),
            ("replace('a', ['-'], '+')", "argument 2 (find)", "Array"),
            ("replace('a', '-', 1)", "argument 3 (replacement)", "Int"),
            (
                "replace('a', '-', null)",
                "argument 3 (replacement)",
                "Null",
            ),
        ],
    );
}

#[test]
fn replace_takes_exactly_three_arguments() {
    assert_arity("replace", 3, &[0, 1, 2, 4]);
}

// --- trim, trim_left, trim_right ---

#[test]
fn trim_removes_leading_and_trailing_whitespace() {
    assert_values(&[
        ("trim('  a b  ')", "'a b'"),
        ("trim_left('  a b  ')", "'a b  '"),
        ("trim_right('  a b  ')", "'  a b'"),
        (r"trim('\t\n a \r\n')", "'a'"),
        (r"trim_left('\t\n a \r\n')", r"'a \r\n'"),
        (r"trim_right('\t\n a \r\n')", r"'\t\n a'"),
    ]);
}

#[test]
fn trim_removes_unicode_whitespace() {
    // A no-break space, an ideographic space, and a line separator.
    assert_values(&[
        (r"trim('\u{a0}\u{3000}a\u{2028}')", "'a'"),
        (r"trim_left('\u{a0}a\u{a0}')", r"'a\u{a0}'"),
        (r"trim_right('\u{a0}a\u{a0}')", r"'\u{a0}a'"),
    ]);
}

#[test]
fn trim_leaves_a_string_without_edge_whitespace_unchanged() {
    for function in ["trim", "trim_left", "trim_right"] {
        assert_values(&[
            (&format!("{function}('a')"), "'a'"),
            (&format!("{function}('a  b')"), "'a  b'"),
            (&format!("{function}('')"), "''"),
        ]);
    }
}

#[test]
fn trim_of_only_whitespace_is_empty() {
    for function in ["trim", "trim_left", "trim_right"] {
        assert_values(&[(&format!(r"{function}(' \t\n ')"), "''")]);
    }
}

#[test]
fn trim_trims_runtime_values() {
    let trimmed = Script::new("[trim(s), trim_left(s), trim_right(s)]")
        .capture("s", Value::from(" x "))
        .run();
    assert_eq!(trimmed, Value::array(["x", "x ", " x"]));
}

#[test]
fn trim_checks_its_argument_type() {
    for function in ["trim", "trim_left", "trim_right"] {
        assert_rejects_non_string(function);
    }
}

#[test]
fn trim_takes_exactly_one_argument() {
    for function in ["trim", "trim_left", "trim_right"] {
        assert_arity(function, 1, &[0, 2]);
    }
}

// --- to_upper, to_lower ---

#[test]
fn to_upper_and_to_lower_convert_case() {
    assert_values(&[
        ("to_upper('Hello, World 1!')", "'HELLO, WORLD 1!'"),
        ("to_lower('Hello, World 1!')", "'hello, world 1!'"),
        ("to_upper('')", "''"),
        ("to_lower('')", "''"),
    ]);
}

#[test]
fn to_upper_and_to_lower_convert_unicode_case() {
    assert_values(&[
        (r"to_upper('caf\u{e9}')", r"'CAF\u{c9}'"),
        (r"to_lower('CAF\u{c9}')", r"'caf\u{e9}'"),
        // A character may change length: sharp s uppercases to two characters.
        (r"to_upper('stra\u{df}e')", "'STRASSE'"),
        // Characters without case are unchanged.
        (r"to_upper('\u{1f600}1')", r"'\u{1f600}1'"),
    ]);
}

#[test]
fn to_upper_and_to_lower_convert_runtime_values() {
    let converted = Script::new("[to_upper(s), to_lower(s)]")
        .capture("s", Value::from("aB"))
        .run();
    assert_eq!(converted, Value::array(["AB", "ab"]));
}

#[test]
fn to_upper_and_to_lower_check_their_argument_type() {
    assert_rejects_non_string("to_upper");
    assert_rejects_non_string("to_lower");
}

#[test]
fn to_upper_and_to_lower_take_exactly_one_argument() {
    for function in ["to_upper", "to_lower"] {
        assert_arity(function, 1, &[0, 2]);
    }
}

// --- contains, starts_with, ends_with ---

#[test]
fn contains_finds_a_substring() {
    assert_values(&[
        ("contains('hello', 'ell')", "true"),
        ("contains('hello', 'hello')", "true"),
        ("contains('hello', 'hellos')", "false"),
        ("contains('hello', 'L')", "false"),
        // A partial match does not hide a later full one.
        ("contains('aab', 'ab')", "true"),
        (r"contains('caf\u{e9}!', '\u{e9}')", "true"),
        // The empty string is in every string.
        ("contains('hello', '')", "true"),
        ("contains('', '')", "true"),
        ("contains('', 'a')", "false"),
    ]);
}

#[test]
fn starts_with_and_ends_with_test_an_edge() {
    assert_values(&[
        ("starts_with('hello', 'he')", "true"),
        ("starts_with('hello', 'lo')", "false"),
        ("starts_with('hello', 'hello')", "true"),
        ("starts_with('he', 'hello')", "false"),
        ("starts_with('hello', '')", "true"),
        ("starts_with('', '')", "true"),
        ("ends_with('hello', 'lo')", "true"),
        ("ends_with('hello', 'he')", "false"),
        ("ends_with('hello', 'hello')", "true"),
        ("ends_with('lo', 'hello')", "false"),
        ("ends_with('hello', '')", "true"),
        ("ends_with('', '')", "true"),
    ]);
}

#[test]
fn searches_work_on_bytes() {
    assert_values(&[
        ("contains(x'616162', x'6162')", "true"),
        ("contains(x'6100ff', x'00ff')", "true"),
        ("contains(x'6162', x'6261')", "false"),
        ("contains(x'61', x'')", "true"),
        ("contains(x'', x'')", "true"),
        ("contains(x'', x'00')", "false"),
        ("starts_with(x'ff00', x'ff')", "true"),
        ("starts_with(x'ff00', x'00')", "false"),
        ("ends_with(x'ff00', x'00')", "true"),
        ("ends_with(x'ff00', x'ff')", "false"),
    ]);
}

#[test]
fn searches_mix_strings_and_bytes() {
    assert_values(&[
        ("contains('abc', x'62')", "true"),
        ("contains(x'616263', 'b')", "true"),
        ("contains('abc', x'00')", "false"),
        ("starts_with('abc', x'61')", "true"),
        ("starts_with(x'616263', 'ab')", "true"),
        ("ends_with('abc', x'63')", "true"),
        ("ends_with(x'616263', 'bc')", "true"),
        // Mixed, a search matches bytes, even within a character.
        (r"contains('\u{e9}', x'a9')", "true"),
        (r"starts_with('\u{e9}', x'c3')", "true"),
        (r"ends_with('\u{e9}', x'a9')", "true"),
    ]);
}

#[test]
fn searches_search_runtime_values() {
    let found = Script::new("[contains(s, 'b'), starts_with(s, x'61'), ends_with(s, 'c')]")
        .capture("s", Value::from("abc"))
        .run();
    assert_eq!(found, run("[true, true, true]"));
}

#[test]
fn contains_points_an_array_search_at_includes() {
    let message = "Function contains requires String or Bytes as argument 1, got Array \
                   (use includes to search an Array)";
    assert_raises(&[
        ("contains(['a'], 'a')", message),
        // The hint comes first, even when the second argument is also wrong.
        ("contains(['a'], 1)", message),
        ("contains([], null)", message),
    ]);
}

#[test]
fn searches_check_their_argument_types() {
    for function in ["contains", "starts_with", "ends_with"] {
        assert_rejects_non_flat(
            function,
            &[
                (&format!("{function}(1, 'a')"), "argument 1", "Int"),
                (&format!("{function}(null, 'a')"), "argument 1", "Null"),
                (&format!("{function}({{a: 1}}, 'a')"), "argument 1", "Map"),
                (&format!("{function}('a', 1)"), "argument 2", "Int"),
                (&format!("{function}('a', null)"), "argument 2", "Null"),
                (&format!("{function}('a', ['a'])"), "argument 2", "Array"),
            ],
        );
    }
    assert_rejects_non_flat(
        "starts_with",
        &[("starts_with(['a'], 'a')", "argument 1", "Array")],
    );
    assert_rejects_non_flat(
        "ends_with",
        &[("ends_with(['a'], 'a')", "argument 1", "Array")],
    );
}

#[test]
fn searches_take_exactly_two_arguments() {
    for function in ["contains", "starts_with", "ends_with"] {
        assert_arity(function, 2, &[0, 1, 3]);
    }
}
