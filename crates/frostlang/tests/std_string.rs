//! `std.string`, from Frost source.
//!
//! Each case runs with only `std.string` installed, bound as `str`. Positions
//! and widths count code points. The searching functions and `is_empty` also
//! take Bytes, in any mix with a String; where Bytes are involved, positions
//! count bytes.

use crate::script;

use frostlang::stdlib::{self, StdlibConfig};
use frostlang::{ImporterBuilder, Stdlib, Value};
use script::Script;
use script::assertions::{Library, library_assertions};

library_assertions!(Library::module(stdlib::string, "str"));

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
         'last_index_of', 'pad_left', 'pad_right', 'reader', 'strip_prefix', \
         'strip_suffix', 'writer']",
    )]);
}

#[test]
fn the_module_is_contained() {
    let stdlib = Stdlib::contained(StdlibConfig::default());
    let contained = ImporterBuilder::new().with_stdlib(stdlib).build();
    let result = Script::new("import('std.string').count('banana', 'a')")
        .importer(contained)
        .run();
    assert_eq!(result, Value::Int(3));
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
        (r"str.last_index_of('\u{e9}', '')", "1"),
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

// --- strip_prefix, strip_suffix ---

#[test]
fn stripping_removes_a_present_affix() {
    assert_values(&[
        ("str.strip_prefix('--opt', '--')", "'opt'"),
        ("str.strip_suffix('file.frst', '.frst')", "'file'"),
        ("str.strip_prefix('hello', 'hello')", "''"),
        ("str.strip_suffix('hello', 'hello')", "''"),
        (r"str.strip_prefix('\u{e9}t\u{e9}', '\u{e9}')", r"'t\u{e9}'"),
        (r"str.strip_suffix('\u{e9}t\u{e9}', '\u{e9}')", r"'\u{e9}t'"),
        // Only one occurrence is removed.
        ("str.strip_prefix('aaa', 'a')", "'aa'"),
        ("str.strip_suffix('aaa', 'a')", "'aa'"),
    ]);
}

#[test]
fn stripping_leaves_an_absent_affix_unchanged() {
    assert_values(&[
        ("str.strip_prefix('hello', 'lo')", "'hello'"),
        ("str.strip_suffix('hello', 'he')", "'hello'"),
        ("str.strip_prefix('he', 'hello')", "'he'"),
        ("str.strip_suffix('lo', 'hello')", "'lo'"),
        ("str.strip_prefix('', 'a')", "''"),
        ("str.strip_suffix('', 'a')", "''"),
        // The empty affix is always present, and removing it changes nothing.
        ("str.strip_prefix('hello', '')", "'hello'"),
        ("str.strip_suffix('hello', '')", "'hello'"),
        ("str.strip_prefix('', '')", "''"),
        ("str.strip_suffix('', '')", "''"),
    ]);
}

#[test]
fn stripping_strips_bytes() {
    assert_values(&[
        ("str.strip_prefix(x'ff0061', x'ff00')", "x'61'"),
        ("str.strip_suffix(x'ff0061', x'0061')", "x'ff'"),
        ("str.strip_prefix(x'ff00', x'ff00')", "x''"),
        ("str.strip_prefix(x'ff00', x'00')", "x'ff00'"),
        ("str.strip_suffix(x'ff00', x'ff')", "x'ff00'"),
        ("str.strip_prefix(x'ff', x'')", "x'ff'"),
        ("str.strip_suffix(x'', x'00')", "x''"),
    ]);
}

#[test]
fn stripping_with_any_bytes_argument_returns_bytes() {
    assert_values(&[
        ("str.strip_prefix('abc', x'61')", "x'6263'"),
        ("str.strip_suffix('abc', x'63')", "x'6162'"),
        ("str.strip_prefix(x'616263', 'ab')", "x'63'"),
        ("str.strip_suffix(x'616263', 'bc')", "x'61'"),
        // Unchanged content, but a Bytes argument still makes the result Bytes.
        ("str.strip_prefix('abc', x'00')", "x'616263'"),
        ("str.strip_suffix('abc', x'')", "x'616263'"),
        ("str.strip_prefix(x'616263', 'z')", "x'616263'"),
        // A byte-level strip may cut a character apart.
        (r"str.strip_prefix('\u{e9}', x'c3')", "x'a9'"),
        (r"str.strip_suffix('\u{e9}', x'a9')", "x'c3'"),
    ]);
}

#[test]
fn stripping_strips_runtime_values() {
    let stripped = LIBRARY
        .script(
            r"
            [
                str.strip_prefix(s, 'ab'),
                str.strip_suffix(s, 'bc'),
                str.strip_prefix(s, 'z'),
                str.strip_suffix(s, x'63'),
            ]
            ",
        )
        .capture("s", Value::from("abc"))
        .run();
    assert_eq!(
        stripped,
        Value::array([
            Value::from("c"),
            Value::from("a"),
            Value::from("abc"),
            Value::from(&b"ab"[..]),
        ])
    );
}

#[test]
fn stripping_checks_its_arguments() {
    for (function, affix) in [("strip_prefix", "prefix"), ("strip_suffix", "suffix")] {
        let rejects = |call: &str, position: &str, got: &str| {
            (
                format!("str.{function}{call}"),
                format!(
                    "Function string.{function} requires String or Bytes as {position}, got {got}"
                ),
            )
        };
        let position = format!("argument 2 ({affix})");
        let cases = [
            rejects("(1, 'a')", "argument 1", "Int"),
            rejects("(null, 'a')", "argument 1", "Null"),
            rejects("(['a'], 'a')", "argument 1", "Array"),
            rejects("({a: 1}, 'a')", "argument 1", "Map"),
            rejects("('a', 1)", &position, "Int"),
            rejects("('a', null)", &position, "Null"),
            rejects("('a', ['a'])", &position, "Array"),
            rejects("('a', false)", &position, "Bool"),
        ];
        for (source, message) in &cases {
            assert_raises(&[(source, message)]);
        }
        assert_arity(function, "2", &[0, 1, 3]);
    }
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
fn only_is_ascii_accepts_the_empty_string() {
    // The rest require a character; `is_ascii` asks only that nothing is
    // outside ASCII.
    for function in CLASSIFIERS {
        let expected = if function == "is_ascii" {
            "true"
        } else {
            "false"
        };
        assert_values(&[(&format!("str.{function}('')"), expected)]);
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
        (r"str.is_uppercase('\u{c9}')", "true"),
        (r"str.is_uppercase('\u{e9}')", "false"),
        ("str.is_lowercase('abc')", "true"),
        ("str.is_lowercase('abc 123!')", "true"),
        ("str.is_lowercase('aBc')", "false"),
        (r"str.is_lowercase('\u{e9}')", "true"),
        (r"str.is_lowercase('\u{c9}')", "false"),
    ]);
}

#[test]
fn case_tests_require_a_character_with_case() {
    for text in ["", "123", " !", r"\u{4e2d}"] {
        assert_values(&[
            (&format!("str.is_uppercase('{text}')"), "false"),
            (&format!("str.is_lowercase('{text}')"), "false"),
        ]);
    }
}

#[test]
fn a_title_case_character_is_neither_upper_nor_lower_case() {
    // `\u{1c5}` is `Dz` with caron as one character; `\u{1f88}` is Greek.
    for text in [r"\u{1c5}", r"\u{1f88}", r"A\u{1c5}", r"a\u{1c5}"] {
        assert_values(&[
            (&format!("str.is_uppercase('{text}')"), "false"),
            (&format!("str.is_lowercase('{text}')"), "false"),
        ]);
    }
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
    // The fill is checked even when no padding is needed.
    for function in ["pad_left", "pad_right", "center"] {
        assert_raises(&[(
            &format!("str.{function}('hello', 3, 'ab')"),
            &format!(
                "Function string.{function} requires a single character as argument 3 \
                 (fill), got \"ab\""
            ),
        )]);
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
fn padding_past_what_memory_can_address_is_an_error() {
    // A two-byte fill doubles the bytes a width needs.
    for function in ["pad_left", "pad_right", "center"] {
        assert_raises(&[(
            &format!(r"str.{function}('a', 9223372036854775807, '\u{{e9}}')"),
            &format!("Function string.{function} cannot make a String that long"),
        )]);
    }
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

// --- reader, writer ---
//
// The buffers are streams like files and the standard streams, which share their
// implementation; these cases cover reading, writing, and positions. Closing and
// flushing, which buffers lack, are covered with the files in `std_fs`.

#[test]
fn a_buffer_reader_and_writer_offer_positions_but_no_closing() {
    assert_values(&[
        (
            "sorted(keys(str.reader('')))",
            "['eof', 'read_bytes', 'read_line', 'read_one', 'read_rest', 'read_rest_bytes', \
             'seek', 'tell']",
        ),
        (
            "sorted(keys(str.writer()))",
            "['get', 'get_bytes', 'seek', 'tell', 'write', 'writeln']",
        ),
    ]);
}

#[test]
fn read_line_reads_each_line_without_its_ending() {
    let source = r"
        def r = str.reader('a\nb\r\n\nc')
        [r.read_line(), r.read_line(), r.read_line(), r.read_line(), r.read_line()]
    ";
    assert_values(&[
        (source, "['a', 'b', '', 'c', null]"),
        ("str.reader('').read_line()", "null"),
        // A lone carriage return does not end a line.
        (
            r"
            def r = str.reader('a\rb')
            [r.read_line(), r.read_line()]
            ",
            r"['a\rb', null]",
        ),
        (
            r"
            def r = str.reader('a\r')
            [r.read_line(), r.read_line()]
            ",
            r"['a\r', null]",
        ),
        // A final line ending ends the last line; it does not start another.
        (
            r"
            def r = str.reader('a\n')
            [r.read_line(), r.read_line()]
            ",
            "['a', null]",
        ),
    ]);
}

#[test]
fn read_one_reads_one_character() {
    let source = r"
        def r = str.reader('\u{e9}x\u{20ac}\u{1f600}')
        [r.read_one(), r.read_one(), r.read_one(), r.read_one(), r.read_one()]
    ";
    assert_values(&[(source, r"['\u{e9}', 'x', '\u{20ac}', '\u{1f600}', null]")]);
    // A Bytes reader's text reads are Strings.
    assert_values(&[
        ("str.reader(x'6162').read_one()", "'a'"),
        ("str.reader(x'6162').read_line()", "'ab'"),
        ("str.reader(x'6162').read_rest()", "'ab'"),
    ]);
}

#[test]
fn read_rest_reads_what_remains() {
    let source = r"
        def r = str.reader('one\ntwo\nthree')
        [r.read_line(), r.read_rest(), r.read_rest(), r.eof()]
    ";
    assert_values(&[(source, r"['one', 'two\nthree', '', true]")]);
}

#[test]
fn eof_tests_for_nothing_left() {
    let source = r"
        def r = str.reader('ab')
        [r.eof(), r.read_one(), r.eof(), r.read_one(), r.eof()]
    ";
    assert_values(&[
        (source, "[false, 'a', false, 'b', true]"),
        ("str.reader('').eof()", "true"),
    ]);
}

#[test]
fn binary_reads_read_bytes() {
    let source = r"
        def r = str.reader(x'0102030405')
        [r.read_bytes(2), r.read_bytes(0), r.read_bytes(10), r.read_bytes(1), r.read_bytes(0)]
    ";
    assert_values(&[
        (source, "[x'0102', x'', x'030405', null, x'']"),
        // A String's reader reads its UTF-8.
        (r"str.reader('\u{e9}').read_rest_bytes()", "x'c3a9'"),
        (
            r"
            def r = str.reader('ab')
            [r.read_bytes(1), r.read_rest_bytes(), r.read_rest_bytes()]
            ",
            "[x'61', x'62', x'']",
        ),
    ]);
}

#[test]
fn text_reads_raise_on_content_that_is_not_utf8() {
    assert_raises(&[
        (
            "str.reader(x'ff').read_line()",
            "Function reader.read_line read text that is not UTF-8",
        ),
        (
            "str.reader(x'ff').read_rest()",
            "Function reader.read_rest read text that is not UTF-8",
        ),
        // A character cut short.
        (
            "str.reader(x'c3').read_one()",
            "Function reader.read_one read text that is not UTF-8",
        ),
        (
            "str.reader(x'80').read_one()",
            "Function reader.read_one read text that is not UTF-8",
        ),
        // A lead byte followed by an invalid continuation.
        (
            "str.reader(x'c341').read_one()",
            "Function reader.read_one read text that is not UTF-8",
        ),
    ]);
}

#[test]
fn tell_and_seek_count_bytes() {
    let source = r"
        def r = str.reader('\u{e9}abc')
        def first = r.read_one()
        def after = r.tell()
        r.seek(0)
        def again = r.read_one()
        r.seek(3)
        [first, after, again, r.read_rest(), r.tell()]
    ";
    assert_values(&[(source, r"['\u{e9}', 2, '\u{e9}', 'bc', 5]")]);
    let past_end = r"
        def r = str.reader('ab')
        r.seek(10)
        [r.eof(), r.read_line(), r.tell()]
    ";
    assert_values(&[(past_end, "[true, null, 10]")]);
}

#[test]
fn a_writer_keeps_what_is_written() {
    let source = r"
        def w = str.writer()
        w.write('a')
        w.writeln('b')
        w.write(x'63')
        w.writeln(x'')
        [w.get(), w.get_bytes(), w.tell()]
    ";
    assert_values(&[
        (source, r"['ab\nc\n', x'61620a630a', 5]"),
        ("str.writer().get()", "''"),
    ]);
}

#[test]
fn a_writer_writes_over_what_is_at_its_position() {
    let source = r"
        def w = str.writer()
        w.write('hello')
        w.seek(0)
        w.write('J')
        w.seek(10)
        w.write('!')
        [w.get_bytes(), w.tell()]
    ";
    // Writing past the end fills the gap with zero bytes.
    assert_values(&[(source, "[x'4a656c6c6f0000000000' + x'21', 11]")]);
}

#[test]
fn a_writer_cannot_grow_past_what_memory_can_address() {
    let source = r"
        def w = str.writer()
        w.seek(9223372036854775807)
        w.write('a')
    ";
    assert_raises(&[(
        source,
        "Function writer.write failed: the buffer cannot grow that large",
    )]);
}

#[test]
fn get_raises_on_content_that_is_not_utf8() {
    let source = r"
        def w = str.writer()
        w.write(x'ff')
        w.get()
    ";
    assert_raises(&[(source, "Function writer.get read text that is not UTF-8")]);
    let source = r"
        def w = str.writer()
        w.write(x'ff')
        w.get_bytes()
    ";
    assert_values(&[(source, "x'ff'")]);
}

#[test]
fn buffers_are_independent() {
    let source = r"
        def a = str.writer()
        def b = str.writer()
        a.write('a')
        b.write('b')
        def r = str.reader('xy')
        def s = str.reader('xy')
        r.read_one()
        [a.get(), b.get(), r.read_one(), s.read_one()]
    ";
    assert_values(&[(source, "['a', 'b', 'y', 'x']")]);
}

#[test]
fn stream_functions_check_their_arguments() {
    assert_raises(&[
        (
            "str.reader(1)",
            "Function string.reader requires String or Bytes as argument 1, got Int",
        ),
        (
            "str.writer().write(1)",
            "Function writer.write requires String or Bytes as argument 1, got Int",
        ),
        (
            "str.writer().writeln([])",
            "Function writer.writeln requires String or Bytes as argument 1, got Array",
        ),
        (
            "str.reader('a').read_bytes(-1)",
            "Function reader.read_bytes requires argument 1 to be at least 0, got -1",
        ),
        (
            "str.reader('a').read_bytes('1')",
            "Function reader.read_bytes requires Int as argument 1, got String",
        ),
        (
            "str.reader('a').read_bytes(1.0)",
            "Function reader.read_bytes requires Int as argument 1, got Float",
        ),
        (
            "str.reader('a').seek('1')",
            "Function reader.seek requires Int as argument 1, got String",
        ),
        (
            "str.writer().seek('1')",
            "Function writer.seek requires Int as argument 1, got String",
        ),
        (
            "str.reader('a').seek(-1)",
            "Function reader.seek requires argument 1 to be at least 0, got -1",
        ),
        (
            "str.writer().seek(-1)",
            "Function writer.seek requires argument 1 to be at least 0, got -1",
        ),
    ]);
    assert_arity("reader", 1, &[0, 2]);
    assert_arity("writer", 0, &[1]);
    for (member, expects, counts) in [
        ("read_line", 0, &[1][..]),
        ("read_one", 0, &[1]),
        ("read_rest", 0, &[1]),
        ("read_rest_bytes", 0, &[1]),
        ("read_bytes", 1, &[0, 2]),
        ("eof", 0, &[1]),
        ("tell", 0, &[1]),
        ("seek", 1, &[0, 2]),
    ] {
        assert_arity_of(
            &format!("str.reader('a').{member}"),
            &format!("reader.{member}"),
            expects,
            counts,
        );
    }
    for (member, expects, counts) in [
        ("write", 1, &[0, 2][..]),
        ("writeln", 1, &[0, 2]),
        ("tell", 0, &[1]),
        ("seek", 1, &[0, 2]),
        ("get", 0, &[1]),
        ("get_bytes", 0, &[1]),
    ] {
        assert_arity_of(
            &format!("str.writer().{member}"),
            &format!("writer.{member}"),
            expects,
            counts,
        );
    }
}

#[test]
fn writes_and_seeks_return_null() {
    assert_values(&[
        ("str.writer().write('a')", "null"),
        ("str.writer().writeln('a')", "null"),
        ("str.writer().seek(0)", "null"),
        ("str.reader('a').seek(0)", "null"),
    ]);
}
