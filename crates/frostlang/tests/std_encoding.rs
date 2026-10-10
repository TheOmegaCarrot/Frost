//! `std.encoding`, from Frost source: integers in any base, numbers to fixed
//! decimal places, Bytes as Ints, and the Base64, hex, and URL percent-encodings.
//!
//! Each case runs with only `std.encoding` installed, bound as `enc`.
//! Content to encode or decode may be a String (its UTF-8) or Bytes; a decoder
//! or parser given content that does not decode returns Null.

use crate::script;

use frostlang::{FrostFloat, Value, stdlib};
use script::Script;
use script::assertions::{Library, library_assertions};

library_assertions!(Library::module(stdlib::encoding, "enc"));

/// Assert each `expression` runs to Null.
fn assert_nulls(expressions: &[&str]) {
    for expression in expressions {
        assert_eq!(
            script(expression).run(),
            Value::Null,
            "{expression:?} is Null"
        );
    }
}

// --- The module ---

#[test]
fn the_module_holds_its_functions() {
    assert_values(&[
        (
            "sorted(keys(enc))",
            "['b64', 'fmt_fixed', 'fmt_int', 'from_ints', 'hex', 'parse_int', 'to_ints', 'url']",
        ),
        (
            "sorted(keys(enc.b64))",
            "['decode', 'encode', 'urldecode', 'urlencode']",
        ),
        ("sorted(keys(enc.hex))", "['decode', 'encode']"),
        ("sorted(keys(enc.url))", "['decode', 'encode']"),
        (
            "enc.hex.encode('Hi')",
            "import('std.encoding.hex').encode('Hi')",
        ),
    ]);
}

#[test]
fn the_module_is_absent_unless_installed() {
    let raised = Script::new("import('std.encoding')").raises();
    assert_eq!(raised, "Could not resolve import 'std.encoding'");
}

// --- fmt_int, parse_int ---

#[test]
fn fmt_int_writes_an_int_in_a_base() {
    assert_values(&[
        ("enc.fmt_int(255, 16)", "'ff'"),
        ("enc.fmt_int(10, 2)", "'1010'"),
        ("enc.fmt_int(35, 36)", "'z'"),
        ("enc.fmt_int(36, 36)", "'10'"),
        ("enc.fmt_int(0, 2)", "'0'"),
        ("enc.fmt_int(-255, 16)", "'-ff'"),
        ("enc.fmt_int(7, 10)", "'7'"),
        ("enc.fmt_int(9223372036854775807, 2)", "tile('1', 63)"),
        (
            "enc.fmt_int(-9223372036854775807 - 1, 16)",
            "'-8000000000000000'",
        ),
        (
            "enc.fmt_int(-9223372036854775807 - 1, 10)",
            "'-9223372036854775808'",
        ),
    ]);
}

#[test]
fn parse_int_reads_an_int_in_a_base() {
    assert_values(&[
        ("enc.parse_int('ff', 16)", "255"),
        ("enc.parse_int('FF', 16)", "255"),
        ("enc.parse_int('1010', 2)", "10"),
        ("enc.parse_int('z', 36)", "35"),
        ("enc.parse_int('0', 2)", "0"),
        ("enc.parse_int('-ff', 16)", "-255"),
        ("enc.parse_int('+7', 10)", "7"),
        (
            "enc.parse_int('-9223372036854775808', 10)",
            "-9223372036854775807 - 1",
        ),
    ]);
}

#[test]
fn parse_int_returns_null_for_text_that_is_not_an_int() {
    assert_nulls(&[
        "enc.parse_int('', 10)",
        "enc.parse_int('-', 10)",
        "enc.parse_int('12', 2)",
        "enc.parse_int('ff', 10)",
        "enc.parse_int(' 1', 10)",
        "enc.parse_int('1 ', 10)",
        "enc.parse_int('1_000', 10)",
        "enc.parse_int('0x10', 16)",
        // Past the Int range.
        "enc.parse_int('9223372036854775808', 10)",
        // Only ASCII digits count: an Arabic-Indic three.
        r"enc.parse_int('\u{663}', 10)",
    ]);
}

#[test]
fn fmt_int_and_parse_int_round_trip_in_every_base() {
    assert_values(&[(
        r"
        all(range(2, 37), fn base -> {
            def numbers = [0, 1, -1, 35, 255, 9223372036854775807, -9223372036854775807 - 1]
            all(numbers, fn n -> enc.parse_int(enc.fmt_int(n, base), base) == n)
        })
        ",
        "true",
    )]);
}

#[test]
fn fmt_int_and_parse_int_require_a_base_from_2_to_36() {
    for function in ["fmt_int", "parse_int"] {
        let number = if function == "fmt_int" { "1" } else { "'1'" };
        for base in ["0", "1", "37", "-2"] {
            assert_raises(&[(
                &format!("enc.{function}({number}, {base})"),
                &format!("Function encoding.{function} requires a base from 2 to 36, got {base}"),
            )]);
        }
    }
}

#[test]
fn fmt_int_and_parse_int_check_their_arguments() {
    assert_raises(&[
        (
            "enc.fmt_int(1.0, 10)",
            "Function encoding.fmt_int requires Int as argument 1 (number), got Float",
        ),
        (
            "enc.fmt_int(1, '10')",
            "Function encoding.fmt_int requires Int as argument 2 (base), got String",
        ),
        (
            "enc.parse_int(x'31', 10)",
            "Function encoding.parse_int requires String as argument 1 (text), got Bytes",
        ),
        (
            "enc.parse_int('1', null)",
            "Function encoding.parse_int requires Int as argument 2 (base), got Null",
        ),
    ]);
    assert_arity("fmt_int", 2, &[0, 1, 3]);
    assert_arity("parse_int", 2, &[0, 1, 3]);
}

// --- fmt_fixed ---

#[test]
fn fmt_fixed_writes_a_float_to_fixed_places() {
    assert_values(&[
        ("enc.fmt_fixed(1.5, 2)", "'1.50'"),
        ("enc.fmt_fixed(3.14159, 2)", "'3.14'"),
        ("enc.fmt_fixed(3.14159, 4)", "'3.1416'"),
        ("enc.fmt_fixed(-3.14159, 3)", "'-3.142'"),
        ("enc.fmt_fixed(0.0, 2)", "'0.00'"),
        ("enc.fmt_fixed(9.99, 1)", "'10.0'"),
        // No places, no point.
        ("enc.fmt_fixed(2.0, 0)", "'2'"),
        ("enc.fmt_fixed(2.7, 0)", "'3'"),
        ("enc.fmt_fixed(-2.7, 0)", "'-3'"),
    ]);
}

#[test]
fn fmt_fixed_rounds_a_float_from_its_exact_value() {
    assert_values(&[
        // Stored just below the written tie, so they round down.
        ("enc.fmt_fixed(2.675, 2)", "'2.67'"),
        ("enc.fmt_fixed(1.005, 2)", "'1.00'"),
        // Exact ties go to the even digit.
        ("enc.fmt_fixed(0.125, 2)", "'0.12'"),
        ("enc.fmt_fixed(0.375, 2)", "'0.38'"),
        ("enc.fmt_fixed(-1.25, 1)", "'-1.2'"),
        ("enc.fmt_fixed(0.5, 0)", "'0'"),
        ("enc.fmt_fixed(1.5, 0)", "'2'"),
        ("enc.fmt_fixed(2.5, 0)", "'2'"),
        ("enc.fmt_fixed(-2.5, 0)", "'-2'"),
    ]);
}

#[test]
fn fmt_fixed_writes_zero_without_a_sign() {
    assert_values(&[
        ("enc.fmt_fixed(-0.0, 0)", "'0'"),
        ("enc.fmt_fixed(-0.0, 3)", "'0.000'"),
        ("enc.fmt_fixed(-0.001, 2)", "'0.00'"),
        ("enc.fmt_fixed(-0.4, 0)", "'0'"),
        ("enc.fmt_fixed(-5e-324, 3)", "'0.000'"),
        // A negative Float that does not round to zero keeps its sign.
        ("enc.fmt_fixed(-0.006, 2)", "'-0.01'"),
        ("enc.fmt_fixed(-0.6, 0)", "'-1'"),
    ]);
}

#[test]
fn fmt_fixed_writes_an_int_exactly() {
    assert_values(&[
        ("enc.fmt_fixed(5, 2)", "'5.00'"),
        ("enc.fmt_fixed(5, 0)", "'5'"),
        ("enc.fmt_fixed(-5, 1)", "'-5.0'"),
        ("enc.fmt_fixed(0, 3)", "'0.000'"),
        // Past 2^53, where a Float would round to 9007199254740992.
        ("enc.fmt_fixed(9007199254740993, 1)", "'9007199254740993.0'"),
        (
            "enc.fmt_fixed(9223372036854775807, 0)",
            "'9223372036854775807'",
        ),
        (
            "enc.fmt_fixed(-9223372036854775807 - 1, 2)",
            "'-9223372036854775808.00'",
        ),
    ]);
}

#[test]
fn fmt_fixed_never_uses_scientific_notation() {
    assert_values(&[
        ("enc.fmt_fixed(1e21, 1)", "'1000000000000000000000.0'"),
        ("enc.fmt_fixed(1e-7, 8)", "'0.00000010'"),
        // The largest Float has 309 digits before the point.
        ("len(enc.fmt_fixed(1.7976931348623157e308, 0))", "309"),
        // The smallest positive Float is exact at 1074 places, ending in 5.
        ("enc.fmt_fixed(5e-324, 3)", "'0.000'"),
        ("len(enc.fmt_fixed(5e-324, 1074))", "1076"),
        ("ends_with(enc.fmt_fixed(5e-324, 1074), '5')", "true"),
        ("ends_with(enc.fmt_fixed(5e-324, 1075), '50')", "true"),
    ]);
}

#[test]
fn fmt_fixed_takes_places_from_0_to_65535() {
    assert_values(&[
        ("len(enc.fmt_fixed(1, 65535))", "65537"),
        ("len(enc.fmt_fixed(0.5, 65535))", "65537"),
    ]);
    for places in ["-1", "65536", "-9223372036854775807 - 1"] {
        let got = script(places).run().to_frost_string();
        for number in ["1", "1.0"] {
            assert_raises(&[(
                &format!("enc.fmt_fixed({number}, {places})"),
                &format!("Function encoding.fmt_fixed requires places from 0 to 65535, got {got}"),
            )]);
        }
    }
}

#[test]
fn fmt_fixed_formats_runtime_values() {
    let fixed = script("[enc.fmt_fixed(f, p), enc.fmt_fixed(i, p)]")
        .capture("f", Value::Float(FrostFloat::new(2.345).expect("finite")))
        .capture("i", Value::Int(7))
        .capture("p", Value::Int(1))
        .run();
    assert_eq!(fixed, Value::array(["2.3", "7.0"]));
}

#[test]
fn fmt_fixed_checks_its_arguments() {
    for (call, position, requires, got) in [
        (
            "enc.fmt_fixed('1.5', 2)",
            "argument 1 (number)",
            "Numeric",
            "String",
        ),
        (
            "enc.fmt_fixed(null, 2)",
            "argument 1 (number)",
            "Numeric",
            "Null",
        ),
        (
            "enc.fmt_fixed(true, 2)",
            "argument 1 (number)",
            "Numeric",
            "Bool",
        ),
        (
            "enc.fmt_fixed([1.5], 2)",
            "argument 1 (number)",
            "Numeric",
            "Array",
        ),
        (
            "enc.fmt_fixed(1.5, 2.0)",
            "argument 2 (places)",
            "Int",
            "Float",
        ),
        (
            "enc.fmt_fixed(1.5, '2')",
            "argument 2 (places)",
            "Int",
            "String",
        ),
        (
            "enc.fmt_fixed(1.5, null)",
            "argument 2 (places)",
            "Int",
            "Null",
        ),
    ] {
        assert_raises(&[(
            call,
            &format!("Function encoding.fmt_fixed requires {requires} as {position}, got {got}"),
        )]);
    }
    assert_arity("fmt_fixed", 2, &[0, 1, 3]);
}

// --- to_ints, from_ints ---

#[test]
fn to_ints_lists_each_byte() {
    assert_values(&[
        ("enc.to_ints('ABC')", "[65, 66, 67]"),
        ("enc.to_ints('')", "[]"),
        (r"enc.to_ints('\u{e9}')", "[195, 169]"),
        ("enc.to_ints(x'00ff')", "[0, 255]"),
    ]);
}

#[test]
fn from_ints_makes_bytes() {
    assert_values(&[
        ("enc.from_ints([72, 105])", "x'4869'"),
        ("enc.from_ints([])", "x''"),
        ("enc.from_ints([0, 255])", "x'00ff'"),
        ("enc.from_ints(enc.to_ints('Hi'))", "to_bytes('Hi')"),
    ]);
    assert_nulls(&["enc.from_ints([256])", "enc.from_ints([1, -1])"]);
}

#[test]
fn from_ints_requires_ints_before_it_checks_their_range() {
    assert_raises(&[
        (
            "enc.from_ints([1, 'a'])",
            "Function encoding.from_ints requires an Array of Ints, but element 1 is String",
        ),
        (
            "enc.from_ints([1.0])",
            "Function encoding.from_ints requires an Array of Ints, but element 0 is Float",
        ),
        // An Int out of range before it does not make this Null.
        (
            "enc.from_ints([256, null])",
            "Function encoding.from_ints requires an Array of Ints, but element 1 is Null",
        ),
    ]);
}

#[test]
fn to_ints_and_from_ints_check_their_argument() {
    assert_raises(&[
        (
            "enc.to_ints([1])",
            "Function encoding.to_ints requires String or Bytes as argument 1, got Array",
        ),
        (
            "enc.from_ints('ab')",
            "Function encoding.from_ints requires Array as argument 1, got String",
        ),
    ]);
    assert_arity("to_ints", 1, &[0, 2]);
    assert_arity("from_ints", 1, &[0, 2]);
}

// --- b64 ---

#[test]
fn b64_encode_matches_the_rfc_4648_vectors() {
    assert_values(&[
        ("enc.b64.encode('')", "''"),
        ("enc.b64.encode('f')", "'Zg=='"),
        ("enc.b64.encode('fo')", "'Zm8='"),
        ("enc.b64.encode('foo')", "'Zm9v'"),
        ("enc.b64.encode('foob')", "'Zm9vYg=='"),
        ("enc.b64.encode('fooba')", "'Zm9vYmE='"),
        ("enc.b64.encode('foobar')", "'Zm9vYmFy'"),
        ("enc.b64.encode('Hello')", "'SGVsbG8='"),
    ]);
}

#[test]
fn b64_decode_inverts_the_rfc_4648_vectors() {
    assert_values(&[
        ("enc.b64.decode('')", "x''"),
        ("enc.b64.decode('Zg==')", "to_bytes('f')"),
        ("enc.b64.decode('Zm8=')", "to_bytes('fo')"),
        ("enc.b64.decode('Zm9v')", "to_bytes('foo')"),
        ("enc.b64.decode('Zm9vYmFy')", "to_bytes('foobar')"),
        // Encoded text may arrive as Bytes.
        ("enc.b64.decode(x'5a673d3d')", "to_bytes('f')"),
    ]);
}

#[test]
fn b64_alphabets_differ_in_their_last_two_characters() {
    assert_values(&[
        ("enc.b64.encode(x'fbff')", "'+/8='"),
        ("enc.b64.urlencode(x'fbff')", "'-_8='"),
        ("enc.b64.decode('+/8=')", "x'fbff'"),
        ("enc.b64.urldecode('-_8=')", "x'fbff'"),
        ("enc.b64.urlencode('foobar')", "'Zm9vYmFy'"),
    ]);
    assert_nulls(&["enc.b64.decode('-_8=')", "enc.b64.urldecode('+/8=')"]);
}

#[test]
fn b64_decode_returns_null_for_text_that_is_not_canonical_base64() {
    for decoder in ["decode", "urldecode"] {
        assert_nulls(&[
            // Not a multiple of four characters: unpadded or truncated.
            &format!("enc.b64.{decoder}('Zg')"),
            &format!("enc.b64.{decoder}('Zg=')"),
            // Too much padding, or padding before the end.
            &format!("enc.b64.{decoder}('Z===')"),
            &format!("enc.b64.{decoder}('====')"),
            &format!("enc.b64.{decoder}('Zg==Zg==')"),
            &format!("enc.b64.{decoder}('Zg=A')"),
            // Bits past the last byte that are not zero.
            &format!("enc.b64.{decoder}('Zh==')"),
            &format!("enc.b64.{decoder}('Zm9=')"),
            // Characters outside the alphabet, in a length that is a multiple of four.
            &format!("enc.b64.{decoder}('Zm 9')"),
            &format!("enc.b64.{decoder}('Zm9v Yg=')"),
            &format!(r"enc.b64.{decoder}('\u{{e9}}AA')"),
        ]);
    }
}

#[test]
fn b64_round_trips() {
    for (encoder, decoder) in [("encode", "decode"), ("urlencode", "urldecode")] {
        assert_values(&[
            (
                &format!("enc.b64.{decoder}(enc.b64.{encoder}(enc.from_ints(range(256))))"),
                "enc.from_ints(range(256))",
            ),
            (
                &format!(r"from_utf8(enc.b64.{decoder}(enc.b64.{encoder}('h\u{{e9}}llo')))"),
                r"'h\u{e9}llo'",
            ),
        ]);
    }
}

#[test]
fn b64_functions_check_their_argument() {
    for function in ["encode", "decode", "urlencode", "urldecode"] {
        assert_raises(&[(
            &format!("enc.b64.{function}(1)"),
            &format!(
                "Function encoding.b64.{function} requires String or Bytes as argument 1, got Int"
            ),
        )]);
        assert_arity(&format!("b64.{function}"), 1, &[0, 2]);
    }
}

// --- hex ---

#[test]
fn hex_encodes_and_decodes() {
    assert_values(&[
        ("enc.hex.encode('Hi')", "'4869'"),
        ("enc.hex.encode(x'00ff10')", "'00ff10'"),
        ("enc.hex.encode('')", "''"),
        ("enc.hex.decode('4869')", "x'4869'"),
        // Either case decodes.
        ("enc.hex.decode('4A6b')", "x'4a6b'"),
        ("enc.hex.decode('')", "x''"),
        ("enc.hex.decode(x'3438')", "x'48'"),
        (
            "enc.hex.decode(enc.hex.encode(enc.from_ints(range(256))))",
            "enc.from_ints(range(256))",
        ),
    ]);
    assert_nulls(&[
        "enc.hex.decode('486')",
        "enc.hex.decode('zz')",
        "enc.hex.decode('4g')",
        "enc.hex.decode(' 48')",
        "enc.hex.decode('0x48')",
    ]);
}

#[test]
fn hex_functions_check_their_argument() {
    for function in ["encode", "decode"] {
        assert_raises(&[(
            &format!("enc.hex.{function}([1])"),
            &format!(
                "Function encoding.hex.{function} requires String or Bytes as argument 1, got Array"
            ),
        )]);
        assert_arity(&format!("hex.{function}"), 1, &[0, 2]);
    }
}

// --- url ---

#[test]
fn url_encode_escapes_all_but_unreserved_characters() {
    assert_values(&[
        ("enc.url.encode('hello world')", "'hello%20world'"),
        ("enc.url.encode('AZaz09-._~')", "'AZaz09-._~'"),
        ("enc.url.encode('a/b?c=d&e')", "'a%2Fb%3Fc%3Dd%26e'"),
        ("enc.url.encode('%+')", "'%25%2B'"),
        (r"enc.url.encode('\u{e9}')", "'%C3%A9'"),
        ("enc.url.encode(x'ff00')", "'%FF%00'"),
        ("enc.url.encode('')", "''"),
    ]);
}

#[test]
fn url_decode_unescapes_to_text() {
    assert_values(&[
        ("enc.url.decode('hello%20world')", "'hello world'"),
        ("enc.url.decode('%2f%2F')", "'//'"),
        // `+` is not a space in percent-encoding.
        ("enc.url.decode('a+b')", "'a+b'"),
        ("enc.url.decode('%C3%A9')", r"'\u{e9}'"),
        (r"enc.url.decode('\u{e9}')", r"'\u{e9}'"),
        ("enc.url.decode('')", "''"),
        // Encoded text may arrive as Bytes: `%41`.
        ("enc.url.decode(x'253431')", "'A'"),
        (
            r"enc.url.decode(enc.url.encode('a b/\u{e9}?&=%'))",
            r"'a b/\u{e9}?&=%'",
        ),
    ]);
}

#[test]
fn url_decode_returns_null_for_text_that_does_not_decode() {
    assert_nulls(&[
        "enc.url.decode('%GG')",
        "enc.url.decode('%2')",
        "enc.url.decode('%')",
        "enc.url.decode('abc%')",
        "enc.url.decode('%41%')",
        // Decodes, but not to UTF-8.
        "enc.url.decode('%FF')",
        "enc.url.decode('%C3')",
    ]);
}

#[test]
fn url_functions_check_their_argument() {
    for function in ["encode", "decode"] {
        assert_raises(&[(
            &format!("enc.url.{function}(null)"),
            &format!(
                "Function encoding.url.{function} requires String or Bytes as argument 1, got Null"
            ),
        )]);
        assert_arity(&format!("url.{function}"), 1, &[0, 2]);
    }
}
