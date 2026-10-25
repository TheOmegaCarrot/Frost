//! YAML: encoding, decoding, and the values YAML has no form for.
//!
//! YAML text is written as Frost String literals, lines joined by `\n`. The
//! decoder's own refusals are pinned; the YAML parser's are only checked to be
//! decode errors.

use crate::common::{
    FormatModule, assert_arity, assert_raises, assert_strings, assert_true, assert_values,
    borrowed, nested_arrays, nested_maps, string, type_errors,
};

const YAML: FormatModule = FormatModule {
    name: "yaml",
    format: "YAML",
};

/// The reason the decoder gives for nesting past the limit.
const TOO_DEEP: &str = "Arrays and Maps nest more than 64 deep";

/// A Frost String literal of YAML text, in flow style, of [`nested_arrays`]:
/// `depth` Arrays, each the only element of the one enclosing it.
fn nested_arrays_text(depth: usize) -> String {
    format!("'{}{}'", "[".repeat(depth), "]".repeat(depth))
}

/// A Frost String literal of YAML text, in block style, of [`nested_maps`]:
/// `depth` Maps, each the only entry of the one enclosing it, under `a`.
fn nested_maps_text(depth: usize) -> String {
    // A line for each Map but the innermost two; the last line holds those.
    let mut lines: Vec<String> = (0..depth - 2)
        .map(|level| format!("{}a:", "  ".repeat(level)))
        .collect();
    lines.push(format!("{}a: {{}}", "  ".repeat(depth - 2)));
    format!("'{}'", lines.join(r"\n"))
}

// --- encode ---

#[test]
fn encode_writes_each_scalar() {
    assert_strings(&[
        ("yaml.encode(null)", "null\n"),
        ("yaml.encode(true)", "true\n"),
        ("yaml.encode(false)", "false\n"),
        ("yaml.encode(0)", "0\n"),
        ("yaml.encode(-42)", "-42\n"),
        ("yaml.encode(9223372036854775807)", "9223372036854775807\n"),
        (
            "yaml.encode(-9223372036854775807 - 1)",
            "-9223372036854775808\n",
        ),
        ("yaml.encode('hello')", "hello\n"),
    ]);
}

#[test]
fn encode_writes_every_float_with_a_fraction_or_exponent() {
    // A Float that is a whole number keeps its `.0`, which is what makes it
    // decode as a Float rather than an Int.
    assert_strings(&[
        ("yaml.encode(1.0)", "1.0\n"),
        ("yaml.encode(-0.0)", "-0.0\n"),
        ("yaml.encode(0.0)", "0.0\n"),
        ("yaml.encode(0.1)", "0.1\n"),
        ("yaml.encode(-2.5)", "-2.5\n"),
        ("yaml.encode(1e300)", "1.0e+300\n"),
        ("yaml.encode(1.5e-7)", "1.5e-7\n"),
    ]);
}

#[test]
fn encode_writes_special_floats() {
    assert_strings(&[
        ("yaml.encode(math.special_float('inf'))", ".inf\n"),
        ("yaml.encode(math.special_float('-inf'))", "-.inf\n"),
        ("yaml.encode(math.special_float('NaN'))", ".nan\n"),
        // YAML has one NaN.
        ("yaml.encode(math.special_float('-NaN'))", ".nan\n"),
    ]);
}

#[test]
fn encode_quotes_a_string_plain_text_would_not_read_back_as_itself() {
    assert_strings(&[
        ("yaml.encode('')", "\"\"\n"),
        ("yaml.encode('true')", "\"true\"\n"),
        ("yaml.encode('null')", "\"null\"\n"),
        ("yaml.encode('~')", "\"~\"\n"),
        ("yaml.encode('1')", "\"1\"\n"),
        ("yaml.encode('1.0')", "\"1.0\"\n"),
        ("yaml.encode('.inf')", "\".inf\"\n"),
        ("yaml.encode('0x1F')", "\"0x1F\"\n"),
        // Read as a Bool by YAML 1.1, which other readers may follow.
        ("yaml.encode('yes')", "\"yes\"\n"),
        // Strings plain text would change: trimmed, or cut at a comment.
        ("yaml.encode(' lead')", "\" lead\"\n"),
        ("yaml.encode('a # b')", "\"a # b\"\n"),
    ]);
}

#[test]
fn encode_writes_a_string_of_lines_as_a_block() {
    assert_strings(&[
        (r"yaml.encode('a\nb')", "|-\n  a\n  b\n"),
        (r"yaml.encode('a\nb\n')", "|\n  a\n  b\n"),
    ]);
}

#[test]
fn encode_writes_block_style() {
    assert_strings(&[
        // An empty Array or Map has no block form.
        ("yaml.encode([])", "[]\n"),
        ("yaml.encode({})", "{}\n"),
        ("yaml.encode([1, [2, 3]])", "- 1\n- - 2\n  - 3\n"),
        ("yaml.encode({a: 1, b: [2]})", "a: 1\nb:\n- 2\n"),
        ("yaml.encode({a: {b: {c: 1}}})", "a:\n  b:\n    c: 1\n"),
        ("yaml.encode([{a: 1, b: 2}])", "- a: 1\n  b: 2\n"),
        ("yaml.encode({a: [], b: {}})", "a: []\nb: {}\n"),
    ]);
}

#[test]
fn encode_writes_every_key_type_as_itself() {
    // Entries come in key order: Bool, Int, Float, then String.
    assert_strings(&[(
        "yaml.encode({s: 4, [1.5]: 3, [-0.0]: 5, [1]: 2, [true]: 1})",
        "true: 1\n1: 2\n-0.0: 5\n1.5: 3\ns: 4\n",
    )]);
}

#[test]
fn encode_refuses_what_yaml_has_no_form_for() {
    assert_raises(&[
        ("yaml.encode(x'00')", &YAML.cannot_encode("Bytes")),
        (
            "yaml.encode({a: [1, x'00']})",
            &YAML.cannot_encode_at("Bytes", "a[1]"),
        ),
        (
            "yaml.encode({[x'00']: 1})",
            &YAML.cannot_encode("the Bytes key x'00'"),
        ),
        ("yaml.encode(fn -> 1)", &YAML.cannot_encode("Function")),
        ("yaml.encode(test.widget)", &YAML.cannot_encode("<Widget>")),
        (
            "yaml.encode(toml.datetime('1979-05-27'))",
            &YAML.cannot_encode("<TOML Datetime: 1979-05-27>"),
        ),
        (
            "yaml.encode([u64.from_string('18446744073709551615')])",
            &YAML.cannot_encode_at("<Uint64: 18446744073709551615>", "[0]"),
        ),
    ]);
}

#[test]
fn encode_nests_as_deep_as_decode_reads() {
    for deepest in [nested_arrays(64), nested_maps(64)] {
        assert_true(&[&format!(
            "
            def v = {deepest}
            yaml.decode(yaml.encode(v)) == v
            "
        )]);
    }
    let too_deep = YAML.too_deep(64);
    assert_raises(&[
        (&format!("yaml.encode({})", nested_arrays(65)), &too_deep),
        (&format!("yaml.encode({})", nested_maps(65)), &too_deep),
    ]);
}

// --- decode ---

#[test]
fn decode_reads_each_scalar() {
    assert_values(&[
        ("yaml.decode('5')", "5"),
        ("yaml.decode('-5')", "-5"),
        ("yaml.decode('2.5')", "2.5"),
        ("yaml.decode('true')", "true"),
        ("yaml.decode('false')", "false"),
        ("yaml.decode('hello')", "'hello'"),
        ("yaml.decode('hello world')", "'hello world'"),
    ]);
}

#[test]
fn decode_reads_true_and_false_however_capitalized() {
    assert_values(&[
        ("yaml.decode('True')", "true"),
        ("yaml.decode('FALSE')", "false"),
        ("yaml.decode('tRUE')", "true"),
    ]);
    // YAML 1.1's other booleans are Strings.
    for word in ["yes", "no", "on", "off", "y", "n", "Yes", "ON"] {
        assert_values(&[(&format!("yaml.decode('{word}')"), &format!("'{word}'"))]);
    }
}

#[test]
fn decode_reads_each_form_of_null() {
    // `null` however capitalized; text that is empty, only a comment, or only
    // a document start.
    for text in ["~", "null", "Null", "NULL", "nULL", "", "# nothing", "---"] {
        assert_values(&[(&format!("yaml.decode('{text}')"), "null")]);
    }
    // An empty value is Null too.
    assert_values(&[(r"yaml.decode('a:\nb: ~')", "{a: null, b: null}")]);
}

#[test]
fn decode_reads_integers_in_each_form() {
    assert_values(&[
        ("yaml.decode('0x1F')", "31"),
        ("yaml.decode('0o17')", "15"),
        ("yaml.decode('0b101')", "5"),
        ("yaml.decode('1_000')", "1000"),
        ("yaml.decode('+1')", "1"),
        ("yaml.decode('-0')", "0"),
        ("yaml.decode('9223372036854775807')", "9223372036854775807"),
        (
            "yaml.decode('-9223372036854775808')",
            "-9223372036854775807 - 1",
        ),
    ]);
}

#[test]
fn decode_reads_floats_in_each_form() {
    assert_values(&[
        ("yaml.decode('1.0')", "1.0"),
        ("yaml.decode('.5')", "0.5"),
        ("yaml.decode('1e3')", "1000.0"),
        ("yaml.decode('1.5e-7')", "1.5e-7"),
    ]);
    assert_true(&[
        "yaml.decode('-0.0') == -0.0",
        "yaml.decode('.inf') == math.special_float('inf')",
        "yaml.decode('-.inf') == math.special_float('-inf')",
        "yaml.decode('.Inf') == math.special_float('inf')",
        "yaml.decode('.nan') == math.special_float('NaN')",
        "yaml.decode('.NaN') == math.special_float('NaN')",
        // Beyond every Float.
        "yaml.decode('1e400') == math.special_float('inf')",
        "yaml.decode('-1e400') == math.special_float('-inf')",
    ]);
}

#[test]
fn decode_reads_an_integer_beyond_the_int_range_as_the_nearest_float() {
    assert_values(&[
        (
            "yaml.decode('9223372036854775808')",
            "9223372036854775808.0",
        ),
        (
            "yaml.decode('-9223372036854775809')",
            "-9223372036854775808.0",
        ),
        (
            "yaml.decode('18446744073709551616')",
            "18446744073709551616.0",
        ),
        (
            "yaml.decode('0xFFFFFFFFFFFFFFFF')",
            "18446744073709551615.0",
        ),
    ]);
}

#[test]
fn decode_reads_a_decimal_with_leading_zeros_as_a_float() {
    assert_values(&[("yaml.decode('017')", "17.0"), ("yaml.decode('00')", "0.0")]);
}

#[test]
fn decode_reads_hexadecimal_octal_or_binary_beyond_64_bits_as_a_string() {
    assert_values(&[
        (
            "yaml.decode('0x10000000000000000')",
            "'0x10000000000000000'",
        ),
        (
            "yaml.decode('0o2000000000000000000000')",
            "'0o2000000000000000000000'",
        ),
        (
            &format!("yaml.decode('0b1{}')", "0".repeat(64)),
            &format!("'0b1{}'", "0".repeat(64)),
        ),
    ]);
}

#[test]
fn decode_reads_a_quoted_scalar_as_a_string() {
    assert_values(&[
        (r"yaml.decode('\'true\'')", "'true'"),
        (r#"yaml.decode('"1"')"#, "'1'"),
        (r"yaml.decode('\'~\'')", "'~'"),
        (r#"yaml.decode('""')"#, "''"),
        (r#"yaml.decode('"a\\tb"')"#, r"'a\tb'"),
    ]);
}

#[test]
fn decode_reads_dates_and_times_as_strings() {
    assert_values(&[
        ("yaml.decode('2001-12-14')", "'2001-12-14'"),
        (
            "yaml.decode('2001-12-14t21:59:43.10-05:00')",
            "'2001-12-14t21:59:43.10-05:00'",
        ),
    ]);
}

#[test]
fn decode_reads_block_scalars() {
    assert_values(&[
        (r"yaml.decode('|\n  a\n  b\n')", r"'a\nb\n'"),
        (r"yaml.decode('|-\n  a\n  b\n')", r"'a\nb'"),
        (r"yaml.decode('>\n  a\n  b\n')", r"'a b\n'"),
    ]);
}

#[test]
fn decode_reads_block_and_flow_collections() {
    assert_values(&[
        (r"yaml.decode('- 1\n- [2, 3]')", "[1, [2, 3]]"),
        (r"yaml.decode('a: 1\nb:\n  c: 2')", "{a: 1, b: {c: 2}}"),
        (
            "yaml.decode('{a: [1, 2], b: {c: 3}}')",
            "{a: [1, 2], b: {c: 3}}",
        ),
        ("yaml.decode('[]')", "[]"),
        ("yaml.decode('{}')", "{}"),
        // A comment is no part of the value.
        (r"yaml.decode('# head\na: 1 # tail')", "{a: 1}"),
    ]);
}

#[test]
fn decode_reads_every_key_type() {
    assert_values(&[(
        r"yaml.decode('true: 1\n1: 2\n1.5: 3\ns: 4\n-0.0: 5')",
        "{[true]: 1, [1]: 2, [1.5]: 3, s: 4, [-0.0]: 5}",
    )]);
    // An Int, a Float, and a String key of one number are distinct, as in a
    // Frost Map.
    assert_values(&[(
        r#"yaml.decode('1: a\n"1": b\n1.0: c')"#,
        "{[1]: 'a', ['1']: 'b', [1.0]: 'c'}",
    )]);
}

#[test]
fn decode_expands_an_alias_to_the_value_its_anchor_marks() {
    assert_values(&[
        (
            r"yaml.decode('a: &x [1, 2]\nb: *x')",
            "{a: [1, 2], b: [1, 2]}",
        ),
        (r"yaml.decode('- &s hi\n- *s')", "['hi', 'hi']"),
    ]);
    // An alias with no anchor, and one within what its own anchor marks.
    YAML.assert_cannot_decode(&["yaml.decode('a: *nowhere')", "yaml.decode('a: &x [*x]')"]);
}

#[test]
fn decode_merges_a_merge_key() {
    assert_values(&[
        // The mapping's own keys win.
        (
            r"yaml.decode('base: &b {x: 1, y: 1}\nd:\n  <<: *b\n  y: 2')",
            "{base: {x: 1, y: 1}, d: {x: 1, y: 2}}",
        ),
        // Of several mappings, the earlier wins.
        (
            r"yaml.decode('a: &a {x: 1}\nb: &b {x: 2, y: 2}\nc:\n  <<: [*a, *b]\n  z: 3')",
            "{a: {x: 1}, b: {x: 2, y: 2}, c: {x: 1, y: 2, z: 3}}",
        ),
        // Quoted, it is an ordinary key.
        (r#"yaml.decode('"<<": 1')"#, "{['<<']: 1}"),
    ]);
    // It names only mappings.
    YAML.assert_cannot_decode(&[r"yaml.decode('a:\n  <<: 1')"]);
}

#[test]
fn decode_refuses_aliases_expanding_to_an_outsize_value() {
    // Each level holds ten of the one before, 20 levels deep: 10^20 scalars.
    let mut lines = vec!["a0: &a0 [x, x, x, x, x, x, x, x, x, x]".to_string()];
    for level in 1..20 {
        let previous = format!("*a{}", level - 1);
        lines.push(format!(
            "a{level}: &a{level} [{}]",
            vec![previous; 10].join(", ")
        ));
    }
    YAML.assert_cannot_decode(&[&format!("yaml.decode('{}')", lines.join(r"\n"))]);
}

#[test]
fn decode_reads_the_core_tags() {
    assert_values(&[
        ("yaml.decode('!!str 1')", "'1'"),
        ("yaml.decode('!!int \"1\"')", "1"),
        ("yaml.decode('!!float 1')", "1.0"),
        ("yaml.decode('!!bool \"true\"')", "true"),
        ("yaml.decode('!!null')", "null"),
        ("yaml.decode('!<tag:yaml.org,2002:int> 5')", "5"),
        // Binary data that is UTF-8 is its text.
        ("yaml.decode('!!binary aGk=')", "'hi'"),
        // And the collection tags, on their own kinds of node.
        ("yaml.decode('!!seq [1]')", "[1]"),
        (r"yaml.decode('a: !!map\n  b: 1')", "{a: {b: 1}}"),
    ]);
    // A tag its node does not fit.
    YAML.assert_cannot_decode(&[
        "yaml.decode('!!int x')",
        "yaml.decode('!!bool x')",
        "yaml.decode('!!map [1]')",
    ]);
}

#[test]
fn decode_refuses_any_other_tag() {
    YAML.assert_cannot_decode(&[
        "yaml.decode('!foo bar')",
        "yaml.decode('!foo 5')",
        "yaml.decode('!<tag:example.com,2000:x> 5')",
        "yaml.decode('!!set {a, b}')",
        "yaml.decode('!!omap [a: 1]')",
        "yaml.decode('!!timestamp 2001-12-14')",
        // Binary data that is not UTF-8.
        "yaml.decode('!!binary /w==')",
        // Within a value.
        r"yaml.decode('a:\n  - !foo 1')",
    ]);
}

#[test]
fn decode_refuses_a_key_frost_cannot_hold() {
    YAML.assert_cannot_decode_because(&[
        ("yaml.decode('null: 1')", "Type Null is not a valid Map key"),
        (
            r"yaml.decode('? [1]\n: 2')",
            "Type Array is not a valid Map key",
        ),
        (
            r"yaml.decode('? {a: 1}\n: 2')",
            "Type Map is not a valid Map key",
        ),
        // An Opaque is named by its rendering.
        (
            "yaml.decode('.nan: 1')",
            "<SpecialFloat: NaN> is not a valid Map key",
        ),
    ]);
}

#[test]
fn decode_refuses_a_key_given_twice() {
    YAML.assert_cannot_decode(&[
        r"yaml.decode('a: 1\na: 2')",
        "yaml.decode('{a: 1, a: 2}')",
        // The same Int, written two ways.
        r"yaml.decode('1: a\n0x1: b')",
    ]);
    // The same Float, written two ways, which only the decoder sees as one.
    YAML.assert_cannot_decode_because(&[(r"yaml.decode('1.0: a\n1.00: b')", "duplicate key 1.0")]);
}

#[test]
fn decode_refuses_a_bool_or_float_key_beside_the_string_spelled_the_same() {
    // `serde-saphyr` takes them for one key, so a Map holding both encodes
    // but does not decode back. Such keys are rare enough to accept this.
    YAML.assert_cannot_decode(&[
        r#"yaml.decode('true: 1\n"true": 2')"#,
        r#"yaml.decode('1.5: 1\n"1.5": 2')"#,
        "yaml.decode(yaml.encode({[true]: 1, ['true']: 2}))",
    ]);
}

#[test]
fn decode_errors_say_where_the_problem_is() {
    let reason = YAML.decode_error_reason(r"yaml.decode('a: 1\nb: 2\n  c: 3')");
    assert!(
        reason.contains("line 3") && reason.contains("column"),
        "{reason:?} gives the line and column of the problem"
    );
    assert!(
        !reason.contains('\n'),
        "{reason:?} is one line, without an excerpt of the text"
    );
}

#[test]
fn decode_errors_are_worded_for_whoever_wrote_the_text() {
    // `serde-saphyr` can advise on its Rust API, which is no help to a script
    // author.
    for expression in [
        r"yaml.decode('a: 1\na: 2')",
        r"yaml.decode('a\n---\nb')",
        "yaml.decode('!!binary /w==')",
    ] {
        let reason = YAML.decode_error_reason(expression);
        assert!(
            !reason.contains("Options") && !reason.contains("from_str"),
            "{expression:?} gives {reason:?}, which speaks of the Rust API"
        );
    }
}

#[test]
fn decode_refuses_malformed_input() {
    YAML.assert_cannot_decode(&[
        "yaml.decode('[1, 2')",
        "yaml.decode('{a: 1')",
        "yaml.decode('a: b: c')",
        r"yaml.decode('a: 1\n b: 2')",
        r#"yaml.decode('"open')"#,
        // A single quote not doubled within a single-quoted scalar.
        r"yaml.decode('a: \'it\'s\'')",
    ]);
}

#[test]
fn decode_refuses_more_than_one_document() {
    YAML.assert_cannot_decode(&[
        r"yaml.decode('a\n---\nb')",
        r"yaml.decode('---\na\n---\nb')",
        // A document end does not end the text.
        r"yaml.decode('a\n...\nb')",
    ]);
    // Markers around one document are fine.
    assert_values(&[(r"yaml.decode('---\na: 1\n...\n')", "{a: 1}")]);
}

#[test]
fn decode_ignores_what_is_not_yaml_after_a_document_end() {
    assert_values(&[(r"yaml.decode('a\n...\n@@@ [')", "'a'")]);
}

#[test]
fn decode_skips_a_leading_byte_order_mark() {
    assert_values(&[(r"yaml.decode('\u{feff}a: 1')", "{a: 1}")]);
}

#[test]
fn decode_reads_as_deep_as_encode_writes() {
    assert_true(&[
        &format!(
            "yaml.decode({}) == {}",
            nested_arrays_text(64),
            nested_arrays(64)
        ),
        &format!(
            "yaml.decode({}) == {}",
            nested_maps_text(64),
            nested_maps(64)
        ),
    ]);
    YAML.assert_cannot_decode_because(&[
        (
            &format!("yaml.decode({})", nested_arrays_text(65)),
            TOO_DEEP,
        ),
        (&format!("yaml.decode({})", nested_maps_text(65)), TOO_DEEP),
        // Far past the limit, the decoder still refuses first, before the
        // budget.
        (&format!("yaml.decode({})", nested_maps_text(100)), TOO_DEEP),
    ]);
    // Farther still, the YAML parser's own flow nesting limit refuses first.
    YAML.assert_cannot_decode(&[&format!("yaml.decode({})", nested_arrays_text(1000))]);
}

#[test]
fn decode_counts_the_depth_an_alias_adds() {
    // `a` is 60 deep, so `b`, holding it 6 deeper, is too deep.
    let anchored = format!("{}1{}", "[".repeat(60), "]".repeat(60));
    YAML.assert_cannot_decode_because(&[(
        &format!(r"yaml.decode('a: &a {anchored}\nb: [[[[[[*a]]]]]]')"),
        TOO_DEEP,
    )]);
}

#[test]
fn decode_limits_the_depth_of_keys_and_merged_mappings() {
    // `serde-saphyr` reads each of these whole before the decoder sees any
    // of it, so it limits their depth itself. Each, read without a limit,
    // would overflow a thread's stack.
    //
    // 170 deep: past what the stack holds unlimited, yet within the parser's
    // own flow nesting limit of 255, so only the depth budget refuses it.
    let deep_key = format!(r"? {}{}\n: 1", "[".repeat(170), "]".repeat(170));
    // No part is past the limit alone: an anchored Array 62 deep, as a key
    // within Maps 60 deep.
    let deep_through_an_alias = format!(
        "[&x {}{}, {}{{*x : 1}}{}]",
        "[".repeat(62),
        "]".repeat(62),
        "{a: ".repeat(60),
        "}".repeat(60)
    );
    // Each merged mapping holds the one before, 100 levels deeper: 3000
    // levels in all, though the text nests only 102 at most.
    let mut merges = vec!["x: 1".to_string()];
    for level in 0..30 {
        let inner = if level == 0 {
            "0".to_string()
        } else {
            format!("*a{}", level - 1)
        };
        merges.push(format!(
            "<<: {{x: &a{level} {}{inner}{}}}",
            "[".repeat(100),
            "]".repeat(100)
        ));
    }
    YAML.assert_cannot_decode(&[
        &format!("yaml.decode('{deep_key}')"),
        &format!("yaml.decode('{deep_through_an_alias}')"),
        &format!("yaml.decode('{}')", merges.join(r"\n")),
    ]);
}

// --- round trips ---

/// Values covering every type YAML has a form for, and their edges.
const REPRESENTABLE: &[&str] = &[
    "null",
    "true",
    "false",
    "0",
    "-1",
    "9223372036854775807",
    "-9223372036854775807 - 1",
    "0.0",
    "-0.0",
    "1.0",
    "0.1",
    "1e300",
    "-1.7976931348623157e308",
    "5e-324",
    "math.special_float('inf')",
    "math.special_float('-inf')",
    "{a: [math.special_float('inf'), math.special_float('-inf')]}",
    "''",
    "'hello'",
    // Strings that read as something else, or not at all, unquoted.
    "['true', 'False', 'tRuE', 'null', 'nULL', '~', '1', '-1.5', '.nan', '0x1F', '0o17', '017']",
    "['1_000', 'yes', 'no', 'on', 'off', 'y', 'n', '<<', '=', '-', '- a', '? a', ': a']",
    "['a: b', 'a #b', '#a', '[a]', '{a}', '&a', '*a', '!a', '%a', '@a', '`a', '|', '>']",
    r#"[' lead', 'trail ', '---', '...', '\'', '"', '2001-12-14']"#,
    r"'\'\n\t\\\u{0}\u{1f}\u{7f}\u{85}\u{2028}\u{feff}'",
    r"['a\nb', 'a\nb\n', 'a\n\n', '\na', ' a\nb', 'a \nb']",
    "'caf\u{e9} \u{1f600} \u{10ffff}'",
    // Long enough to be folded across lines.
    "tile('word ', 40)",
    // Keys written apart from their values, as `? key`: one too long to share
    // its line, and one of several lines.
    r"{[tile('k', 1100)]: 1, ['a\nb']: 2}",
    // A merge key's text, as an ordinary key.
    "{['<<']: {a: 1}}",
    "[]",
    "{}",
    "[[], {}, [[]], [{}]]",
    "[1, 1.0, '1', null, true]",
    "{a: 1, b: 1.0, c: '1', d: null, e: false}",
    "{[true]: 1, [0]: 2, [-0.0]: 3, [0.0]: 4, ['']: 5, ['my key']: 6, ['1']: 7}",
    "{a: {b: [{c: [1, 2.5, 'x']}]}, l: [[1, [2]], {m: {}}]}",
];

#[test]
fn decode_reads_a_value_of_any_size() {
    // Past every size limit `serde-saphyr` sets by default: its count of
    // nodes, for one.
    assert_true(&[r"
        def v = range(300000)
        yaml.decode(yaml.encode(v)) == v
    "]);
}

#[test]
fn decoding_what_encode_gives_gives_the_value_encoded() {
    for value in REPRESENTABLE {
        assert_true(&[&format!(
            "
            def v = {value}
            yaml.decode(yaml.encode(v)) == v
            "
        )]);
    }
}

#[test]
fn a_nan_round_trips_as_a_nan() {
    // Without its sign: YAML has one NaN.
    assert_true(&[
        "yaml.decode(yaml.encode(math.special_float('NaN'))) == math.special_float('NaN')",
        "yaml.decode(yaml.encode(math.special_float('-NaN'))) == math.special_float('NaN')",
    ]);
}

#[test]
fn every_float_round_trips_exactly() {
    // Bit patterns spread evenly over the whole `f64` range reach every
    // exponent and sign, subnormals included, each with a different mantissa.
    const SAMPLES: u64 = 4099;
    let floats: Vec<String> = (0..SAMPLES)
        .map(|i| f64::from_bits(i * (u64::MAX / SAMPLES)))
        .filter(|f| f.is_finite())
        .map(|f| format!("{f:?}"))
        .collect();
    assert_values(&[(
        &format!(
            "
            def floats = [{}]
            filter(floats, fn f -> yaml.decode(yaml.encode(f)) != f)
            ",
            floats.join(", ")
        ),
        "[]",
    )]);
}

#[test]
fn round_trips_keep_ints_and_floats_apart() {
    for value in ["1", "1.0", "-0.0", "0"] {
        assert_eq!(
            string(&format!("type(yaml.decode(yaml.encode({value})))")),
            string(&format!("type({value})")),
            "{value} keeps its type"
        );
    }
}

#[test]
fn decoding_then_encoding_keeps_the_data_but_not_the_formatting() {
    assert_strings(&[
        (
            r"yaml.encode(yaml.decode('{b: 1,   a: [1e2, 1.50, yes]}  # note'))",
            "a:\n- 100.0\n- 1.5\n- \"yes\"\nb: 1\n",
        ),
        // An alias is written as a copy of what its anchor marks.
        (
            r"yaml.encode(yaml.decode('a: &x [1]\nb: *x'))",
            "a:\n- 1\nb:\n- 1\n",
        ),
    ]);
}

// --- arity and types ---

#[test]
fn encode_takes_one_value() {
    assert_arity("yaml.encode", "1", &[0, 2]);
}

#[test]
fn decode_takes_one_string() {
    assert_arity("yaml.decode", "1", &[0, 2]);
    let cases = type_errors(
        "yaml.decode",
        "String",
        "text",
        &[
            ("x'61'", "Bytes"),
            ("1", "Int"),
            ("null", "Null"),
            ("{}", "Map"),
        ],
    );
    assert_raises(&borrowed(&cases));
}
