//! TOML: encoding, decoding, datetimes, the values TOML has no form for, and
//! the options.

use crate::common::{
    FormatModule, assert_arity, assert_raises, assert_strings, assert_true, assert_values,
    nested_arrays, nested_maps,
};

const TOML: FormatModule = FormatModule {
    name: "toml",
    format: "TOML",
};

/// A Frost String literal of the TOML text of [`nested_maps`]: `depth`
/// tables, each the only entry of the one enclosing it, under `a`, with the
/// document itself the outermost. The document and the innermost `{}` are two
/// of the `depth` levels.
fn nested_tables_text(depth: usize) -> String {
    format!(
        "'a = {}{{}}{}'",
        "{a = ".repeat(depth - 2),
        "}".repeat(depth - 2)
    )
}

/// TOML text of each kind of date and time.
pub(crate) const DATETIMES: &[&str] = &[
    // Offset datetimes.
    "1979-05-27T07:32:00Z",
    "1979-05-27T00:32:00.999999-07:00",
    // A local datetime.
    "1979-05-27T07:32:00",
    // A local date.
    "1979-05-27",
    // Local times.
    "07:32:00",
    "00:32:00.5",
];

/// Text that spells no TOML date, time, or datetime.
pub(crate) const NOT_DATETIMES: &[&str] = &[
    "",
    "nope",
    "1979-13-01",
    "25:00:00",
    "1979-05-27T",
    "1979-05-27 07:32:00 Z",
];

// --- encode ---

#[test]
fn encode_lays_out_a_document_as_people_write_toml() {
    // A table's plain values first, then its tables and arrays of tables,
    // in key order among themselves, each table named by its dotted path.
    assert_strings(&[(
        r"toml.encode({
            package: {name: 'frost', version: '0.0.1'},
            dependencies: {serde: '1', toml: {version: '1', features: ['a', 'b']}},
            bin: [{name: 'x', path: 'src/x.rs'}, {name: 'y'}],
            zeta: 1,
        })",
        r#"zeta = 1

[[bin]]
name = "x"
path = "src/x.rs"

[[bin]]
name = "y"

[dependencies]
serde = "1"

[dependencies.toml]
features = ["a", "b"]
version = "1"

[package]
name = "frost"
version = "0.0.1"
"#,
    )]);
}

#[test]
fn encode_writes_each_scalar() {
    assert_strings(&[
        (
            "toml.encode({a: true, b: false})",
            r"a = true
b = false
",
        ),
        (
            "toml.encode({a: 0, b: -42})",
            r"a = 0
b = -42
",
        ),
        (
            "toml.encode({a: 9223372036854775807, b: -9223372036854775807 - 1})",
            r"a = 9223372036854775807
b = -9223372036854775808
",
        ),
        (
            "toml.encode({a: 1.0, b: -0.0, c: 0.5})",
            r"a = 1.0
b = -0.0
c = 0.5
",
        ),
        (
            "toml.encode({a: 'hello'})",
            r#"a = "hello"
"#,
        ),
    ]);
}

#[test]
fn encode_writes_special_floats() {
    assert_strings(&[(
        r"toml.encode({
            a: math.special_float('NaN'),
            b: math.special_float('inf'),
            c: math.special_float('-inf'),
        })",
        r"a = nan
b = inf
c = -inf
",
    )]);
}

#[test]
fn encode_quotes_a_key_that_needs_it() {
    assert_strings(&[(
        r"toml.encode({['']: 1, ['my key']: 2, ['a.b']: 3, plain_key: 4})",
        r#""" = 1
"a.b" = 3
"my key" = 2
plain_key = 4
"#,
    )]);
}

#[test]
fn encode_writes_maps_within_other_arrays_inline() {
    assert_strings(&[(
        "toml.encode({a: [1, 'x', {k: 1}], b: [[{k: 2}]]})",
        r#"a = [1, "x", { k = 1 }]
b = [[{ k = 2 }]]
"#,
    )]);
}

#[test]
fn encode_writes_an_empty_document_and_empty_containers() {
    assert_strings(&[
        ("toml.encode({})", ""),
        (
            "toml.encode({a: [], b: {}})",
            r"a = []

[b]
",
        ),
    ]);
}

#[test]
fn encode_refuses_a_document_that_is_not_a_map() {
    assert_raises(&[
        ("toml.encode(1)", &TOML.cannot_encode_as_document("Int")),
        (
            "toml.encode('text')",
            &TOML.cannot_encode_as_document("String"),
        ),
        ("toml.encode([1])", &TOML.cannot_encode_as_document("Array")),
        ("toml.encode(true)", &TOML.cannot_encode_as_document("Bool")),
        (
            "toml.encode(toml.datetime('1979-05-27'))",
            &TOML.cannot_encode_as_document("<TOML Datetime: 1979-05-27>"),
        ),
        (
            "toml.encode(math.special_float('inf'))",
            &TOML.cannot_encode_as_document("<SpecialFloat: inf>"),
        ),
    ]);
}

#[test]
fn encode_refuses_what_toml_has_no_form_for() {
    assert_raises(&[
        // Even as the whole document, a value with no TOML form at all is
        // refused as such.
        ("toml.encode(null)", &TOML.cannot_encode("Null")),
        ("toml.encode(x'00')", &TOML.cannot_encode("Bytes")),
        ("toml.encode(fn -> 1)", &TOML.cannot_encode("Function")),
        ("toml.encode(test.widget)", &TOML.cannot_encode("<Widget>")),
        (
            "toml.encode({a: null})",
            &TOML.cannot_encode_at("Null", "a"),
        ),
        (
            "toml.encode({a: [1, x'00']})",
            &TOML.cannot_encode_at("Bytes", "a[1]"),
        ),
        (
            "toml.encode({a: {b: len}})",
            &TOML.cannot_encode_at("Function", "a.b"),
        ),
        (
            "toml.encode({a: test.widget})",
            &TOML.cannot_encode_at("<Widget>", "a"),
        ),
        (
            "toml.encode({[1]: 'a'})",
            &TOML.cannot_encode("the Int key 1"),
        ),
        (
            "toml.encode({a: [{[true]: 1}]})",
            &TOML.cannot_encode_at("the Bool key true", "a[0]"),
        ),
    ]);
}

#[test]
fn encode_refuses_the_first_unrepresentable_value_in_key_order() {
    // Though plain values are written before tables, the value refused is
    // the first in key order.
    assert_raises(&[
        (
            "toml.encode({a: {x: null}, b: null})",
            &TOML.cannot_encode_at("Null", "a.x"),
        ),
        (
            "toml.encode({a: null, b: {x: null}})",
            &TOML.cannot_encode_at("Null", "a"),
        ),
    ]);
}

#[test]
fn encode_nests_as_deep_as_decode_reads() {
    // The document is the outermost Map, so 81 Maps in all.
    assert_true(&[
        &format!(
            "
            def v = {}
            toml.decode(toml.encode(v)) == v
            ",
            nested_maps(81)
        ),
        &format!(
            "
            def v = {{a: {}}}
            toml.decode(toml.encode(v)) == v
            ",
            nested_arrays(80)
        ),
    ]);
    let too_deep = TOML.too_deep(81);
    assert_raises(&[
        (&format!("toml.encode({})", nested_maps(82)), &too_deep),
        (
            &format!("toml.encode({{a: {}}})", nested_arrays(81)),
            &too_deep,
        ),
        (
            &format!("toml.encode({{a: {}}})", nested_arrays(100_000)),
            &too_deep,
        ),
    ]);
}

// --- decode ---

#[test]
fn decode_gives_a_map_of_each_kind_of_value() {
    assert_values(&[
        ("toml.decode('')", "{}"),
        (
            r"toml.decode('''
                a = true
                b = 42
                c = 1.5
                d = 'x'
                e = [1, 2]
                ''')",
            "{a: true, b: 42, c: 1.5, d: 'x', e: [1, 2]}",
        ),
        (
            r"toml.decode('''
                [t]
                k = 1
                [t.u]
                v = 2
                ''')",
            "{t: {k: 1, u: {v: 2}}}",
        ),
        ("toml.decode('a.b.c = 1')", "{a: {b: {c: 1}}}"),
        ("toml.decode('t = { k = 1, l = [] }')", "{t: {k: 1, l: []}}"),
        (
            r"toml.decode('''
                [[t]]
                k = 1
                [[t]]
                k = 2
                ''')",
            "{t: [{k: 1}, {k: 2}]}",
        ),
    ]);
}

#[test]
fn decode_reads_every_integer_spelling() {
    assert_values(&[(
        r"toml.decode('''
            a = 0x1F
            b = 0o17
            c = 0b101
            d = 1_000
            e = +5
            f = -0
            ''')",
        "{a: 31, b: 15, c: 5, d: 1000, e: 5, f: 0}",
    )]);
}

#[test]
fn decode_reads_floats_and_special_floats() {
    assert_values(&[
        (
            r"toml.decode('''
                a = 1e3
                b = -0.0
                c = 6.626e-34
                ''')",
            "{a: 1000.0, b: -0.0, c: 6.626e-34}",
        ),
        (
            r"toml.decode('''
                a = inf
                b = -inf
                c = +inf
                d = nan
                e = -nan
                ''')",
            "{
                a: math.special_float('inf'),
                b: math.special_float('-inf'),
                c: math.special_float('inf'),
                d: math.special_float('NaN'),
                e: math.special_float('-NaN'),
            }",
        ),
    ]);
}

#[test]
fn decode_reads_every_string_spelling() {
    // Frost's own escape `\\` spells the backslash of each TOML escape. A
    // Frost `'''` String holds TOML's `"""` strings, and a Frost `"""` String
    // its `'''` strings.
    assert_values(&[
        (
            r#"toml.decode('''
                a = "esc\\tape"
                b = 'lit\\eral'
                c = """
                multi
                line"""
                ''')"#,
            r"{a: 'esc\tape', b: 'lit\\eral', c: 'multi\nline'}",
        ),
        (
            r#"toml.decode("""
                d = '''
                raw
                lines'''
                """)"#,
            r"{d: 'raw\nlines'}",
        ),
    ]);
}

#[test]
fn decode_refuses_an_integer_beyond_the_int_range() {
    // TOML integers are 64-bit signed; the specification requires an error
    // for any other.
    TOML.assert_cannot_decode_because(&[
        (
            "toml.decode('a = 9223372036854775808')",
            "integer 9223372036854775808 is beyond the Int range",
        ),
        (
            "toml.decode('a = -9223372036854775809')",
            "integer -9223372036854775809 is beyond the Int range",
        ),
        (
            "toml.decode('a = 99999999999999999999')",
            "integer 99999999999999999999 is beyond the Int range",
        ),
    ]);
}

#[test]
fn decode_refuses_malformed_input() {
    TOML.assert_cannot_decode(&[
        "toml.decode('a')",
        "toml.decode('a = ')",
        r"toml.decode('''
            a = 1
            a = 2
            ''')",
        r"toml.decode('''
            [t]
            [t]
            ''')",
        "toml.decode('a = [1,,]')",
        "toml.decode('a = 1e400')",
        "toml.decode('a = 1979-13-01')",
        "toml.decode('= 1')",
    ]);
}

#[test]
fn decode_errors_say_where_the_problem_is_on_one_line() {
    let reason = TOML.decode_error_reason(
        r"toml.decode('''
            x = 1
            y = 2
            z = [1,,]
            ''')",
    );
    assert!(
        reason.contains("line 3") && !reason.contains('\n'),
        "{reason:?} gives the line of the problem, on one line"
    );
}

#[test]
fn decode_errors_count_columns_as_an_editor_shows_them() {
    // Characters, not bytes; and neither a byte order mark nor the `\r` of a
    // line ending takes a column. Escapes write the characters these tests
    // are about.
    for (expression, position) in [
        ("toml.decode('\u{e9}\u{e9} = [1,,]')", "line 1 column 9"),
        (r"toml.decode('\u{feff}x = = 1')", "line 1 column 5"),
        (
            r"toml.decode('x = 1\r\ny = \'abc\r\nz = 1')",
            "line 2 column 9",
        ),
    ] {
        let reason = TOML.decode_error_reason(expression);
        assert!(
            reason.ends_with(&format!("at {position}")),
            "{reason:?} is at {position}"
        );
    }
}

#[test]
fn decode_skips_a_leading_byte_order_mark() {
    assert_values(&[(r"toml.decode('\u{feff}a = 1')", "{a: 1}")]);
    // Only one, and only at the start.
    TOML.assert_cannot_decode(&[
        r"toml.decode('\u{feff}\u{feff}a = 1')",
        r"toml.decode('a = 1\n\u{feff}b = 2')",
    ]);
}

#[test]
fn decode_reads_as_deep_as_encode_writes() {
    assert_true(&[&format!(
        "toml.decode({}) == {}",
        nested_tables_text(81),
        nested_maps(81)
    )]);
    TOML.assert_cannot_decode(&[&format!("toml.decode({})", nested_tables_text(82))]);
}

#[test]
fn decode_refuses_nesting_past_the_limit_however_it_is_written() {
    // Dotted keys within inline tables nest far deeper than the parser's own
    // limit on inline tables, here 20 tables of 79 levels each.
    let key = vec!["a"; 79].join(".");
    let text = (0..20).fold("z = 1".to_string(), |inner, _| {
        format!("x = {{{key} = {{{inner}}}}}")
    });
    TOML.assert_cannot_decode_because(&[(
        &format!("toml.decode('{text}')"),
        "Arrays and Maps nest more than 81 deep",
    )]);
}

// --- datetimes ---

#[test]
fn every_kind_of_datetime_decodes_to_an_opaque_rendering_its_text() {
    for text in DATETIMES {
        assert_true(&[&format!("is_opaque(toml.decode('a = {text}').a)")]);
        assert_values(&[(
            &format!("to_string(toml.decode('a = {text}').a)"),
            &format!("'<TOML Datetime: {text}>'"),
        )]);
    }
}

#[test]
fn every_kind_of_datetime_encodes_back_as_itself() {
    for text in DATETIMES {
        let encoded = format!("a = {text}\n");
        assert_strings(&[
            (&format!("toml.encode(toml.decode('a = {text}'))"), &encoded),
            (
                &format!("toml.encode({{a: toml.datetime('{text}')}})"),
                &encoded,
            ),
        ]);
    }
}

#[test]
fn datetimes_are_equal_by_their_text() {
    assert_true(&[
        "toml.datetime('1979-05-27') == toml.datetime('1979-05-27')",
        "toml.decode('a = 1979-05-27').a == toml.datetime('1979-05-27')",
        "toml.datetime('1979-05-27') != toml.datetime('1979-05-28')",
        "toml.datetime('07:32:00') != toml.datetime('1979-05-27T07:32:00')",
        "toml.datetime('1979-05-27T07:32:00Z') != toml.datetime('1979-05-27T07:32:00')",
    ]);
}

#[test]
fn datetime_reads_any_kind_of_date_or_time() {
    for text in DATETIMES {
        assert_values(&[(
            &format!("to_string(toml.datetime('{text}'))"),
            &format!("'<TOML Datetime: {text}>'"),
        )]);
    }
    // A space may separate the date and time, and is written back as `T`.
    assert_values(&[(
        "to_string(toml.datetime('1979-05-27 07:32:00Z'))",
        "'<TOML Datetime: 1979-05-27T07:32:00Z>'",
    )]);
}

#[test]
fn datetime_refuses_text_that_is_no_date_or_time() {
    let cases: Vec<(String, String)> = NOT_DATETIMES
        .iter()
        .map(|text| {
            (
                format!("toml.datetime('{text}')"),
                format!(
                    "Function ext.formats.toml.datetime requires a TOML date, time, or \
                     datetime, got \"{text}\""
                ),
            )
        })
        .collect();
    assert_raises(&borrowed(&cases));
}

// --- round trips ---

/// Documents covering every type TOML has a form for, and their edges.
const REPRESENTABLE: &[&str] = &[
    "{}",
    "{a: true, b: false}",
    "{a: 0, b: -1, c: 9223372036854775807, d: -9223372036854775807 - 1}",
    "{a: 0.0, b: -0.0, c: 1.0, d: 0.1, e: 1e300, f: 5e-324, g: -1.7976931348623157e308}",
    "{a: math.special_float('inf'), b: math.special_float('-inf')}",
    r"{a: '', b: 'hello', c: '\'\n\t\\\u{0}\u{1f}\u{7f}', d: 'caf\u{e9} \u{1f600}'}",
    "{a: [], b: {}, c: [[], [[]]], d: [1, 'x', 1.0, true]}",
    "{a: [{b: 1}, {b: 2, c: [{d: 3}]}]}",
    "{a: [[{b: 1}], {c: {d: [1]}}]}",
    "{['']: 1, ['my key']: 2, ['a.b']: 3, ['caf\u{e9}']: 4, ['if']: 5}",
    "{a: {b: {c: {d: 1}}, e: 2}, f: 3}",
    "{a: toml.datetime('1979-05-27T07:32:00Z'), b: [toml.datetime('07:32:00')]}",
];

#[test]
fn decoding_what_encode_gives_gives_the_value_encoded() {
    for document in REPRESENTABLE {
        for options in ["", ", {pretty: true}"] {
            assert_true(&[&format!(
                "
                def v = {document}
                toml.decode(toml.encode(v{options})) == v
                "
            )]);
        }
    }
}

#[test]
fn a_nan_round_trips_as_a_nan_without_its_sign() {
    assert_true(&[r"
        def nans = [math.special_float('NaN'), math.special_float('-NaN')]
        all(nans, fn nan -> toml.decode(toml.encode({a: nan})).a == math.special_float('NaN'))
    "]);
}

#[test]
fn every_kind_of_datetime_round_trips() {
    for text in DATETIMES {
        assert_true(&[&format!(
            "
            def v = {{a: toml.datetime('{text}'), b: [toml.datetime('{text}')]}}
            toml.decode(toml.encode(v)) == v
            "
        )]);
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
    assert_values(&[(
        &format!(
            "
            def floats = [{}]
            filter(floats, fn f -> toml.decode(toml.encode({{a: f}})).a != f)
            ",
            floats.join(", ")
        ),
        "[]",
    )]);
}

#[test]
fn round_trips_keep_ints_and_floats_apart() {
    assert_true(&[
        "is_int(toml.decode(toml.encode({a: 1})).a)",
        "is_float(toml.decode(toml.encode({a: 1.0})).a)",
    ]);
}

#[test]
fn decoding_then_encoding_keeps_the_data_but_not_the_formatting() {
    assert_strings(&[(
        // Frost's own escape `\\` spells the backslash of a TOML escape.
        r#"toml.encode(toml.decode('''
            # A comment.
            b = 0x10
            a.x = 'lit'
            c = { k = 1_000 }
            d = "\\u0041"
            '''))"#,
        r#"b = 16
d = "A"

[a]
x = "lit"

[c]
k = 1000
"#,
    )]);
}

// --- options ---

#[test]
fn pretty_puts_each_array_element_on_its_own_line() {
    assert_strings(&[
        (
            "toml.encode({a: [1, 2], b: [3], c: []}, {pretty: true})",
            r"a = [
    1,
    2,
]
b = [3]
c = []
",
        ),
        (
            "toml.encode({a: [1, 2]}, {pretty: false})",
            r"a = [1, 2]
",
        ),
        (
            "toml.encode({a: [1, 2]}, {})",
            r"a = [1, 2]
",
        ),
    ]);
}

#[test]
fn options_are_checked() {
    assert_raises(&[
        (
            "toml.encode({}, 'pretty')",
            "Function ext.formats.toml.encode requires Map as argument 2 (options), got String",
        ),
        (
            "toml.encode({}, null)",
            "Function ext.formats.toml.encode requires Map as argument 2 (options), got Null",
        ),
        (
            "toml.encode({}, {pretty: 1})",
            "Function ext.formats.toml.encode requires argument 2 (options) to be valid: \
             expected Bool, got Int (at `pretty`)",
        ),
        (
            "toml.encode({}, {indent: 2})",
            "Function ext.formats.toml.encode requires argument 2 (options) to be valid: \
             unknown field `indent`, expected `pretty` (at a key)",
        ),
    ]);
}

// --- arity and types ---

/// The cases of `cases`, borrowed as [`assert_raises`] takes them.
fn borrowed(cases: &[(String, String)]) -> Vec<(&str, &str)> {
    cases
        .iter()
        .map(|(expression, message)| (expression.as_str(), message.as_str()))
        .collect()
}

/// The type errors `function` raises for each argument in `arguments`, of
/// the type paired with it, as its only argument, `text`.
fn string_type_errors(function: &str, arguments: &[(&str, &str)]) -> Vec<(String, String)> {
    arguments
        .iter()
        .map(|(argument, type_name)| {
            (
                format!("{function}({argument})"),
                format!(
                    "Function ext.formats.{function} requires String as argument 1 (text), \
                     got {type_name}"
                ),
            )
        })
        .collect()
}

#[test]
fn encode_takes_a_document_and_optional_options() {
    assert_arity("toml.encode", "between 1 and 2", &[0, 3, 4]);
}

#[test]
fn decode_takes_one_string() {
    assert_arity("toml.decode", "1", &[0, 2, 3]);
    let cases = string_type_errors(
        "toml.decode",
        &[
            ("x'31'", "Bytes"),
            ("1", "Int"),
            ("null", "Null"),
            ("{}", "Map"),
        ],
    );
    assert_raises(&borrowed(&cases));
}

#[test]
fn datetime_takes_one_string() {
    assert_arity("toml.datetime", "1", &[0, 2]);
    let cases = string_type_errors(
        "toml.datetime",
        &[("x'31'", "Bytes"), ("1", "Int"), ("null", "Null")],
    );
    assert_raises(&borrowed(&cases));
}
