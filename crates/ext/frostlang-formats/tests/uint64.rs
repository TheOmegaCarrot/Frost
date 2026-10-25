//! `ext.formats.u64`: making and reading unsigned 64-bit integers, which are
//! Ints within the Int range and `Uint64` Opaques beyond it. How each format
//! decodes and encodes them is tested with that format.

use crate::common::{
    assert_arity, assert_raises, assert_true, assert_values, borrowed, type_errors,
};

#[test]
fn from_string_gives_an_int_if_it_fits_one() {
    assert_values(&[
        ("u64.from_string('0')", "0"),
        ("u64.from_string('5')", "5"),
        // A leading `+` is allowed.
        ("u64.from_string('+5')", "5"),
        (
            "u64.from_string('9223372036854775807')",
            "9223372036854775807",
        ),
    ]);
}

#[test]
fn from_string_gives_an_opaque_beyond_the_int_range() {
    assert_values(&[
        (
            "to_string(u64.from_string('9223372036854775808'))",
            "'<Uint64: 9223372036854775808>'",
        ),
        (
            "to_string(u64.from_string('18446744073709551615'))",
            "'<Uint64: 18446744073709551615>'",
        ),
    ]);
    assert_true(&["is_opaque(u64.from_string('18446744073709551615'))"]);
}

#[test]
fn from_string_gives_null_for_text_that_is_no_u64() {
    for text in [
        "",
        "-1",
        "18446744073709551616",
        "1.0",
        " 5",
        "0x10",
        "five",
    ] {
        assert_values(&[(&format!("u64.from_string('{text}')"), "null")]);
    }
}

#[test]
fn uint64s_are_equal_by_value() {
    assert_true(&[
        "u64.from_string('18446744073709551615') == u64.from_string('+18446744073709551615')",
        "u64.from_string('18446744073709551615') != u64.from_string('18446744073709551614')",
    ]);
}

#[test]
fn to_string_and_to_float_read_an_unsigned_integer() {
    assert_values(&[
        (
            "u64.to_string(u64.from_string('18446744073709551615'))",
            "'18446744073709551615'",
        ),
        ("u64.to_string(7)", "'7'"),
        ("u64.to_string(0)", "'0'"),
        // The nearest Float, which need not equal it.
        (
            "u64.to_float(u64.from_string('18446744073709551615'))",
            "18446744073709551616.0",
        ),
        ("u64.to_float(7)", "7.0"),
    ]);
}

#[test]
fn to_string_and_to_float_refuse_anything_but_an_unsigned_integer() {
    // Every other argument, of whatever type, is refused in one message.
    for function in ["to_string", "to_float"] {
        let cases: Vec<(String, String)> = [
            ("-1", "-1"),
            ("test.widget", "<Widget>"),
            ("math.special_float('inf')", "<SpecialFloat: inf>"),
            ("'5'", "String"),
            ("5.0", "Float"),
            ("x'05'", "Bytes"),
            ("[5]", "Array"),
            ("{}", "Map"),
            ("len", "Function"),
            ("null", "Null"),
        ]
        .iter()
        .map(|(argument, got)| {
            (
                format!("u64.{function}({argument})"),
                format!(
                    "Function ext.formats.u64.{function} requires an Int that is not \
                     negative, or a Uint64, as argument 1 (n), got {got}"
                ),
            )
        })
        .collect();
        assert_raises(&borrowed(&cases));
    }
}

// --- arity and types ---

#[test]
fn from_string_takes_one_string() {
    assert_arity("u64.from_string", "1", &[0, 2]);
    // Text, not Bytes.
    let cases = type_errors(
        "u64.from_string",
        "String",
        "text",
        &[("x'35'", "Bytes"), ("5", "Int"), ("null", "Null")],
    );
    assert_raises(&borrowed(&cases));
}

#[test]
fn to_string_and_to_float_take_one_argument() {
    // Their argument's type is checked with its value; see
    // `to_string_and_to_float_refuse_anything_but_an_unsigned_integer`.
    for function in ["to_string", "to_float"] {
        assert_arity(&format!("u64.{function}"), "1", &[0, 2]);
    }
}
