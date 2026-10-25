//! JSON: encoding, decoding, the values JSON has no form for, and the options.

use crate::common::{
    FormatModule, assert_arity, assert_raises, assert_strings, assert_true, assert_values,
    nested_arrays, nested_maps, raises, string,
};

const JSON: FormatModule = FormatModule {
    name: "json",
    format: "JSON",
};

/// A Frost String literal of the JSON text of [`nested_arrays`]: `depth`
/// Arrays, each the only element of the one enclosing it.
fn nested_arrays_text(depth: usize) -> String {
    format!("'{}{}'", "[".repeat(depth), "]".repeat(depth))
}

// --- encode ---

#[test]
fn encode_writes_each_scalar() {
    assert_strings(&[
        ("json.encode(null)", "null"),
        ("json.encode(true)", "true"),
        ("json.encode(false)", "false"),
        ("json.encode(0)", "0"),
        ("json.encode(-42)", "-42"),
        ("json.encode(9223372036854775807)", "9223372036854775807"),
        (
            "json.encode(-9223372036854775807 - 1)",
            "-9223372036854775808",
        ),
        ("json.encode('')", r#""""#),
        ("json.encode('hello')", r#""hello""#),
    ]);
}

#[test]
fn encode_writes_every_float_with_a_fraction_or_exponent() {
    // A Float that is a whole number keeps its `.0`, which is what makes it
    // decode as a Float rather than an Int.
    assert_strings(&[
        ("json.encode(1.0)", "1.0"),
        ("json.encode(-0.0)", "-0.0"),
        ("json.encode(0.0)", "0.0"),
        ("json.encode(0.1)", "0.1"),
        ("json.encode(-2.5)", "-2.5"),
        ("json.encode(1e300)", "1e+300"),
        ("json.encode(1.5e-7)", "1.5e-7"),
    ]);
}

#[test]
fn encode_escapes_what_a_json_string_must() {
    assert_strings(&[
        (r"json.encode('a\'b')", r#""a'b""#),
        (r#"json.encode('say "hi"')"#, r#""say \"hi\"""#),
        (r"json.encode('back\\slash')", r#""back\\slash""#),
        (r"json.encode('line\nbreak\ttab')", r#""line\nbreak\ttab""#),
        (r"json.encode('\u{1}')", r#""\u0001""#),
        // Anything else, non-ASCII included, is written as is.
        (
            "json.encode('caf\u{e9} \u{1f600}')",
            "\"caf\u{e9} \u{1f600}\"",
        ),
    ]);
}

#[test]
fn encode_writes_arrays_and_maps_compactly() {
    assert_strings(&[
        ("json.encode([])", "[]"),
        ("json.encode({})", "{}"),
        ("json.encode([1, 'two', null])", r#"[1,"two",null]"#),
        ("json.encode({a: 1, b: [true]})", r#"{"a":1,"b":[true]}"#),
        (
            "json.encode({['my-key']: {inner: []}})",
            r#"{"my-key":{"inner":[]}}"#,
        ),
    ]);
}

#[test]
fn encode_writes_map_entries_in_key_order() {
    assert_strings(&[("json.encode({b: 1, c: 2, a: 3})", r#"{"a":3,"b":1,"c":2}"#)]);
}

#[test]
fn encode_refuses_bytes() {
    assert_raises(&[
        ("json.encode(x'00')", &JSON.cannot_encode("Bytes")),
        ("json.encode(x'')", &JSON.cannot_encode("Bytes")),
    ]);
}

#[test]
fn encode_refuses_a_key_of_any_type_but_string() {
    assert_raises(&[
        (
            "json.encode({[1]: 'a'})",
            &JSON.cannot_encode("the Int key 1"),
        ),
        (
            "json.encode({[1.5]: 'a'})",
            &JSON.cannot_encode("the Float key 1.5"),
        ),
        (
            "json.encode({[true]: 'a'})",
            &JSON.cannot_encode("the Bool key true"),
        ),
        (
            "json.encode({[x'ff']: 'a'})",
            &JSON.cannot_encode("the Bytes key x'ff'"),
        ),
    ]);
}

#[test]
fn encode_refuses_special_floats() {
    assert_raises(&[
        (
            "json.encode(math.special_float('NaN'))",
            &JSON.cannot_encode("<SpecialFloat: NaN>"),
        ),
        (
            "json.encode(math.special_float('inf'))",
            &JSON.cannot_encode("<SpecialFloat: inf>"),
        ),
        (
            "json.encode(math.special_float('-inf'))",
            &JSON.cannot_encode("<SpecialFloat: -inf>"),
        ),
    ]);
}

#[test]
fn encode_refuses_functions_and_opaques() {
    assert_raises(&[
        ("json.encode(fn -> 1)", &JSON.cannot_encode("Function")),
        ("json.encode(len)", &JSON.cannot_encode("Function")),
        ("json.encode(test.widget)", &JSON.cannot_encode("<Widget>")),
        // Another format's Opaques too.
        (
            "json.encode(u64.from_string('18446744073709551615'))",
            &JSON.cannot_encode("<Uint64: 18446744073709551615>"),
        ),
        (
            "json.encode(toml.datetime('1979-05-27'))",
            &JSON.cannot_encode("<TOML Datetime: 1979-05-27>"),
        ),
    ]);
}

#[test]
fn encode_says_where_a_refused_value_is() {
    assert_raises(&[
        (
            "json.encode([1, x'00'])",
            &JSON.cannot_encode_at("Bytes", "[1]"),
        ),
        (
            "json.encode({a: x'00'})",
            &JSON.cannot_encode_at("Bytes", "a"),
        ),
        (
            "json.encode({a: {b: [null, [x'00']]}})",
            &JSON.cannot_encode_at("Bytes", "a.b[1][0]"),
        ),
        (
            "json.encode({['my-key']: x'00'})",
            &JSON.cannot_encode_at("Bytes", r#"["my-key"]"#),
        ),
        (
            "json.encode({['if']: [x'00']})",
            &JSON.cannot_encode_at("Bytes", r#"["if"][0]"#),
        ),
        (
            "json.encode([{a: {[2]: null}}])",
            &JSON.cannot_encode_at("the Int key 2", "[0].a"),
        ),
        (
            "json.encode({a: [math.special_float('NaN')]})",
            &JSON.cannot_encode_at("<SpecialFloat: NaN>", "a[0]"),
        ),
    ]);
}

#[test]
fn encode_refuses_the_first_unrepresentable_value_in_order() {
    // Entries are written in key order, so `a` is refused before `b`.
    assert_raises(&[(
        "json.encode({b: x'00', a: fn -> 1})",
        &JSON.cannot_encode_at("Function", "a"),
    )]);
}

#[test]
fn encode_nests_as_deep_as_decode_reads() {
    for deepest in [nested_arrays(127), nested_maps(127)] {
        assert_true(&[&format!(
            "
            def v = {deepest}
            json.decode(json.encode(v)) == v
            "
        )]);
    }
    for too_deep_value in [nested_arrays(128), nested_maps(128), nested_arrays(100_000)] {
        assert_raises(&[(
            &format!("json.encode({too_deep_value})"),
            &JSON.too_deep(127),
        )]);
    }
}

// --- decode ---

#[test]
fn decode_reads_each_scalar() {
    assert_values(&[
        ("json.decode('null')", "null"),
        ("json.decode('true')", "true"),
        ("json.decode('false')", "false"),
        ("json.decode('0')", "0"),
        ("json.decode('-42')", "-42"),
        ("json.decode('9223372036854775807')", "9223372036854775807"),
        (
            "json.decode('-9223372036854775808')",
            "-9223372036854775807 - 1",
        ),
        (r#"json.decode('"hello"')"#, "'hello'"),
        (r#"json.decode('""')"#, "''"),
    ]);
}

#[test]
fn decode_reads_minus_zero_as_a_zero() {
    // The Float `-0.0`, or with `serde_json`'s `arbitrary_precision`, the Int
    // `0`: these tests run both ways.
    assert_true(&[r"
        def zero = json.decode('-0')
        zero == -0.0 or zero == 0
    "]);
}

#[test]
fn decode_reads_a_number_with_a_fraction_or_exponent_as_a_float() {
    assert_values(&[
        ("json.decode('1.0')", "1.0"),
        ("json.decode('-0.0')", "-0.0"),
        ("json.decode('0.5')", "0.5"),
        ("json.decode('1e3')", "1000.0"),
        ("json.decode('1E3')", "1000.0"),
        ("json.decode('2.5e-1')", "0.25"),
    ]);
    assert_true(&[
        "is_float(json.decode('1.0'))",
        "is_float(json.decode('1e0'))",
        "is_int(json.decode('1'))",
    ]);
}

#[test]
fn decode_reads_an_integer_beyond_the_int_range_as_the_nearest_float() {
    assert_values(&[
        // Just past each end of the Int range.
        (
            "json.decode('9223372036854775808')",
            "9223372036854775808.0",
        ),
        (
            "json.decode('-9223372036854775809')",
            "-9223372036854775808.0",
        ),
        // Past the end of the `u64` range, where `serde_json` reads it.
        (
            "json.decode('18446744073709551616')",
            "18446744073709551616.0",
        ),
        ("json.decode('100000000000000000000000000000')", "1e29"),
    ]);
    assert_true(&["is_float(json.decode('18446744073709551615'))"]);
}

#[test]
fn decode_reads_a_fraction_finer_than_a_float_as_the_nearest_float() {
    assert_values(&[("json.decode('0.10000000000000000000000001')", "0.1")]);
}

#[test]
fn decode_refuses_a_number_beyond_every_float() {
    JSON.assert_cannot_decode(&[
        "json.decode('1e400')",
        "json.decode('-1e400')",
        "json.decode('[1.5e999999]')",
    ]);
}

#[test]
fn decode_reads_string_escapes() {
    assert_values(&[
        (r#"json.decode(R'("a\"b")')"#, r#"'a"b'"#),
        (
            r#"json.decode(R'("\\ \/ \b \f \n \r \t")')"#,
            r"'\\ / \u{8} \u{c} \n \r \t'",
        ),
        // Frost's own escape `\\` spells the backslash of each `\u` escape.
        (
            r#"json.decode('"\\u00e9 \\u00E9 \\u0041"')"#,
            "'\u{e9} \u{e9} A'",
        ),
        // A surrogate pair, escaped, is one character.
        (r#"json.decode('"\\ud83d\\ude00"')"#, "'\u{1f600}'"),
    ]);
}

#[test]
fn decode_refuses_a_lone_surrogate() {
    // A String holds only Unicode scalar values, which a lone surrogate is not.
    JSON.assert_cannot_decode(&[
        r#"json.decode(R'("\ud800")')"#,
        r#"json.decode(R'("\udc00")')"#,
        r#"json.decode(R'("\ud800x")')"#,
    ]);
}

#[test]
fn decode_reads_arrays_and_objects() {
    assert_values(&[
        ("json.decode('[]')", "[]"),
        ("json.decode('{}')", "{}"),
        (
            r#"json.decode('[1, "two", null, [false]]')"#,
            "[1, 'two', null, [false]]",
        ),
        (
            r#"json.decode('{"a": 1, "my-key": {"b": []}}')"#,
            "{a: 1, ['my-key']: {b: []}}",
        ),
    ]);
}

#[test]
fn decode_ignores_whitespace_between_tokens() {
    assert_values(&[(
        r#"json.decode(' \n\t{ "a" : [ 1 , 2 ] }\r\n ')"#,
        "{a: [1, 2]}",
    )]);
}

#[test]
fn decode_refuses_a_key_given_twice() {
    JSON.assert_cannot_decode_because(&[
        (r#"json.decode('{"a": 1, "a": 2}')"#, r#"duplicate key "a""#),
        (
            r#"json.decode('[{"b": {"x": 1, "x": 1}}]')"#,
            r#"duplicate key "x""#,
        ),
        // Keys are compared as decoded, not as spelled.
        (
            r#"json.decode('{"a": 1, "\\u0061": 2}')"#,
            r#"duplicate key "a""#,
        ),
    ]);
}

#[test]
fn decode_errors_say_where_the_problem_is() {
    let reason = JSON.decode_error_reason(
        r#"json.decode('''
            {
              "a": 1,
              "b": ?
            }
            ''')"#,
    );
    assert!(
        reason.contains("line 3"),
        "{reason:?} gives the line of the problem"
    );
}

#[test]
fn decode_refuses_malformed_input() {
    JSON.assert_cannot_decode(&[
        "json.decode('')",
        "json.decode('{')",
        "json.decode('[1,]')",
        "json.decode('nul')",
        "json.decode('{a: 1}')",
        "json.decode('NaN')",
        "json.decode('Infinity')",
        "json.decode('01')",
        r"json.decode('\'a\'')",
        // A String is valid UTF-8, so only the raw control characters remain
        // to refuse inside a string.
        r#"json.decode('"\u{1}"')"#,
    ]);
}

#[test]
fn decode_refuses_anything_after_the_value() {
    JSON.assert_cannot_decode(&[
        "json.decode('1 2')",
        "json.decode('[] x')",
        "json.decode('{}}')",
    ]);
}

#[test]
fn decode_skips_a_leading_byte_order_mark() {
    assert_values(&[
        (r"json.decode('\u{feff}[1]')", "[1]"),
        (r#"json.decode('\u{feff}"x"')"#, "'x'"),
    ]);
    // Only one, and only before the value.
    JSON.assert_cannot_decode(&[
        r"json.decode('\u{feff}\u{feff}1')",
        r"json.decode('[\u{feff}1]')",
        r"json.decode('1\u{feff}')",
    ]);
}

#[test]
fn decode_reads_as_deep_as_encode_writes() {
    assert_true(&[&format!(
        "json.decode({}) == {}",
        nested_arrays_text(127),
        nested_arrays(127)
    )]);
    JSON.assert_cannot_decode(&[&format!("json.decode({})", nested_arrays_text(128))]);
}

// --- round trips ---

/// Values covering every type JSON has a form for, and their edges.
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
    "123456789.123456789",
    "''",
    "'hello'",
    r"'\'\n\t\\\u{0}\u{1f}\u{7f}'",
    "'caf\u{e9} \u{1f600} \u{10ffff}'",
    "[]",
    "{}",
    "[[], {}, [[]]]",
    "[1, 1.0, '1', null, true]",
    "{a: 1, b: 1.0, c: '1', d: null, e: false}",
    "{['']: 1, ['my-key']: 2, ['\u{e9}']: 3, ['if']: 4}",
    "{a: {b: [{c: [1, 2.5, 'x']}]}}",
];

#[test]
fn decoding_what_encode_gives_gives_the_value_encoded() {
    for value in REPRESENTABLE {
        for options in ["", ", {pretty: true}", ", {pretty: true, indent: 0}"] {
            assert_true(&[&format!(
                "
                def v = {value}
                json.decode(json.encode(v{options})) == v
                "
            )]);
        }
    }
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
    assert!(floats.len() > 4000, "most samples are finite");
    assert_values(&[(
        &format!(
            "
            def floats = [{}]
            filter(floats, fn f -> json.decode(json.encode(f)) != f)
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
            string(&format!("type(json.decode(json.encode({value})))")),
            string(&format!("type({value})")),
            "{value} keeps its type"
        );
    }
}

#[test]
fn decoding_then_encoding_keeps_the_data_but_not_the_formatting() {
    assert_strings(&[
        (
            r#"json.encode(json.decode(' { "b" : 1 , "a" : [ 1e2 , 1.50 ] } '))"#,
            r#"{"a":[100.0,1.5],"b":1}"#,
        ),
        (r#"json.encode(json.decode('"A"'))"#, r#""A""#),
    ]);
}

// --- options ---

#[test]
fn pretty_puts_each_element_on_its_own_line() {
    assert_strings(&[
        ("json.encode(1, {pretty: true})", "1"),
        ("json.encode([], {pretty: true})", "[]"),
        ("json.encode({}, {pretty: true})", "{}"),
        (
            "json.encode({a: [1, {}], b: 'x'}, {pretty: true})",
            "{\n  \"a\": [\n    1,\n    {}\n  ],\n  \"b\": \"x\"\n}",
        ),
    ]);
}

#[test]
fn indent_sets_the_spaces_per_level() {
    assert_strings(&[
        (
            "json.encode({a: [1]}, {pretty: true, indent: 4})",
            "{\n    \"a\": [\n        1\n    ]\n}",
        ),
        (
            "json.encode({a: [1]}, {pretty: true, indent: 0})",
            "{\n\"a\": [\n1\n]\n}",
        ),
        (
            "json.encode([1], {pretty: true, indent: 16})",
            &format!("[\n{}1\n]", " ".repeat(16)),
        ),
        ("json.encode([1], {pretty: true, indent: 2})", "[\n  1\n]"),
    ]);
}

#[test]
fn options_left_out_or_off_give_compact_text() {
    assert_strings(&[
        ("json.encode({a: [1]}, {})", r#"{"a":[1]}"#),
        ("json.encode({a: [1]}, {pretty: false})", r#"{"a":[1]}"#),
    ]);
}

#[test]
fn indent_is_taken_only_with_pretty() {
    let message = "Function ext.formats.json.encode takes option `indent` only with `pretty: true`";
    assert_raises(&[
        ("json.encode(1, {indent: 2})", message),
        ("json.encode(1, {pretty: false, indent: 2})", message),
    ]);
}

#[test]
fn indent_is_from_0_to_16() {
    assert_raises(&[
        (
            "json.encode(1, {pretty: true, indent: 17})",
            "Function ext.formats.json.encode requires option `indent` from 0 to 16, got 17",
        ),
        (
            "json.encode(1, {pretty: true, indent: -1})",
            "Function ext.formats.json.encode requires option `indent` from 0 to 16, got -1",
        ),
    ]);
}

#[test]
fn options_are_checked() {
    assert_raises(&[
        (
            "json.encode(1, 'pretty')",
            "Function ext.formats.json.encode requires Map as argument 2 (options), got String",
        ),
        // Null is an argument, unlike leaving the options out.
        (
            "json.encode(1, null)",
            "Function ext.formats.json.encode requires Map as argument 2 (options), got Null",
        ),
        (
            "json.encode(1, {pretty: 1})",
            "Function ext.formats.json.encode requires argument 2 (options) to be valid: \
             expected Bool, got Int (at `pretty`)",
        ),
        (
            "json.encode(1, {pretty: true, indent: 2.0})",
            "Function ext.formats.json.encode requires argument 2 (options) to be valid: \
             expected Int, got Float (at `indent`)",
        ),
        (
            "json.encode(1, {color: true})",
            "Function ext.formats.json.encode requires argument 2 (options) to be valid: \
             unknown field `color`, expected `pretty` or `indent` (at a key)",
        ),
    ]);
}

// --- arity and types ---

#[test]
fn encode_takes_a_value_and_optional_options() {
    assert_arity("json.encode", "between 1 and 2", &[0, 3, 4]);
}

#[test]
fn decode_takes_one_string() {
    assert_arity("json.decode", "1", &[0, 2, 3]);
    for (argument, type_name) in [
        ("x'31'", "Bytes"),
        ("1", "Int"),
        ("null", "Null"),
        ("['1']", "Array"),
    ] {
        assert_eq!(
            raises(&format!("json.decode({argument})")),
            format!(
                "Function ext.formats.json.decode requires String as argument 1 (text), \
                 got {type_name}"
            ),
            "{argument}"
        );
    }
}
