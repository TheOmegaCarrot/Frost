//! `std.encoding`, from Frost source: integers in any base, Bytes as Ints, and
//! the Base64, hex, and URL percent-encodings.
//!
//! Each case runs with only `std.encoding` installed, bound as `enc`.
//! Content to encode or decode may be a String (its UTF-8) or Bytes; a decoder
//! or parser given content that does not decode returns Null.

mod common;

use std::sync::Arc;

use common::Script;
use frost_runtime::{Importer, ImporterBuilder, Stdlib, Value, stdlib};

/// An importer providing only `std.encoding`.
fn importer() -> Arc<Importer> {
    let stdlib = Stdlib::new()
        .with_module(stdlib::encoding())
        .expect("a lone module is accepted");
    ImporterBuilder::new().with_stdlib(stdlib).build()
}

/// `expression`, run with `std.encoding` bound as `enc`.
fn script(expression: &str) -> Script {
    Script::new(&format!("def enc = import('std.encoding')\n{expression}")).importer(importer())
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

/// Assert each `expression` raises exactly `message`.
fn assert_raises(cases: &[(&str, &str)]) {
    for (expression, message) in cases {
        assert_eq!(script(expression).raises(), *message, "{expression:?}");
    }
}

/// Assert `function` (a path under `enc`, such as `b64.encode`) raises its
/// arity error when called with each count in `counts`, where it expects
/// exactly `arity` arguments.
fn assert_arity(function: &str, arity: usize, counts: &[usize]) {
    for &argc in counts {
        let expression = format!("enc.{function}({})", vec!["null"; argc].join(", "));
        assert_raises(&[(
            &expression,
            &format!(
                "Function encoding.{function} expects {arity} arguments, but was called with {argc}"
            ),
        )]);
    }
}

// --- The module ---

#[test]
fn the_module_holds_its_functions() {
    assert_values(&[
        (
            "sorted(keys(enc))",
            "['b64', 'fmt_int', 'from_ints', 'hex', 'parse_int', 'to_ints', 'url']",
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
            // Characters outside the alphabet.
            &format!("enc.b64.{decoder}('Zm9v Yg==')"),
            &format!(r"enc.b64.{decoder}('\u{{e9}}AAA')"),
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
