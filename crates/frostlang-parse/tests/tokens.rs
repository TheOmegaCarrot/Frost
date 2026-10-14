//! `tokens`, the lexer's public face: source to tokens with byte spans.

use std::ops::Range;

use frostlang_parse::ast::SourceSpan;
use frostlang_parse::{LexError, Token, tokens};

/// Each token of `source` with the text its span covers.
fn lexed(source: &str) -> Vec<(Result<Token<'_>, LexError>, &str)> {
    tokens(source)
        .map(|(token, span)| (token, &source[Range::from(span)]))
        .collect()
}

#[test]
fn spans_are_byte_offsets() {
    // `é` is two bytes, so every span after it is offset by bytes, not characters.
    let spans: Vec<SourceSpan> = tokens("'é' x\n~").map(|(_, span)| span).collect();
    assert_eq!(
        spans,
        [
            SourceSpan { start: 0, end: 4 },
            SourceSpan { start: 5, end: 6 },
            SourceSpan { start: 6, end: 7 },
            SourceSpan { start: 7, end: 8 },
        ]
    );
}

#[test]
fn tokens_come_in_order_with_their_spans() {
    let source = "def x = f(1, 'two')";
    assert_eq!(
        lexed(source),
        [
            (Ok(Token::KwDef), "def"),
            (Ok(Token::Identifier("x")), "x"),
            (Ok(Token::Assign), "="),
            (Ok(Token::Identifier("f")), "f"),
            (Ok(Token::OpenParen), "("),
            (Ok(Token::IntLiteral(1)), "1"),
            (Ok(Token::Comma), ","),
            (Ok(Token::SingleQuoteStringLiteral("two")), "'two'"),
            (Ok(Token::CloseParen), ")"),
        ]
    );
}

#[test]
fn spaces_and_comments_make_no_tokens_but_newlines_do() {
    assert_eq!(
        lexed("  a \t# a note (\n  b  "),
        [
            (Ok(Token::Identifier("a")), "a"),
            (Ok(Token::Newline), "\n"),
            (Ok(Token::Identifier("b")), "b"),
        ]
    );
    assert!(lexed("").is_empty());
    assert!(lexed("   # only a comment").is_empty());
}

#[test]
fn a_literals_payload_is_its_text_between_delimiters() {
    assert_eq!(
        lexed(r#"'a\n' $"f ${x}" x'00ff' '''m'''"#),
        [
            (Ok(Token::SingleQuoteStringLiteral(r"a\n")), r"'a\n'"),
            (
                Ok(Token::DoubleQuoteFormatStringLiteral("f ${x}")),
                r#"$"f ${x}""#
            ),
            (Ok(Token::BytesLiteral("00ff")), "x'00ff'"),
            (Ok(Token::MultilineStringLiteral("m")), "'''m'''"),
        ]
    );
}

#[test]
fn an_int_literal_in_another_base_is_its_magnitude() {
    for (source, n) in [
        ("0b101", 5),
        ("0B11", 3),
        ("0o17", 15),
        ("0O17", 15),
        ("0xff", 255),
        ("0XFF", 255),
        ("0xAbC", 0xabc),
        // The least Int's magnitude, which only a negated literal can be.
        ("0x8000000000000000", 1 << 63),
        ("0xffffffffffffffff", u64::MAX),
        // Past `u64::MAX`, the magnitude saturates, as a decimal one does.
        ("0x10000000000000000", u64::MAX),
    ] {
        assert_eq!(
            lexed(source),
            [(Ok(Token::IntLiteral(n)), source)],
            "{source:?}"
        );
    }
}

#[test]
fn a_malformed_based_int_literal_is_one_error() {
    // The literal runs to the end of its word, so a stray character spoils all of
    // it rather than starting a name.
    for source in ["0x", "0b", "0o", "0b2", "0b102", "0o8", "0xfg", "0bar"] {
        assert_eq!(lexed(source), [(Err(LexError), source)], "{source:?}");
    }
    // A word ends at any other character.
    assert_eq!(
        lexed("0xff.x 0b1+0o7"),
        [
            (Ok(Token::IntLiteral(255)), "0xff"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("x")), "x"),
            (Ok(Token::IntLiteral(1)), "0b1"),
            (Ok(Token::OpPlus), "+"),
            (Ok(Token::IntLiteral(7)), "0o7"),
        ]
    );
}

#[test]
fn bytes_that_begin_no_token_are_an_error_and_lexing_resumes() {
    // A character Frost does not use.
    assert_eq!(
        lexed("a ~ b"),
        [
            (Ok(Token::Identifier("a")), "a"),
            (Err(LexError), "~"),
            (Ok(Token::Identifier("b")), "b"),
        ]
    );
    // A point with an exponent and no digits: one malformed number, not the field `e3`
    // of `1`. Other names after an Int's point are fields.
    for number in ["1.e3", "1.E-3", "1.e+3", "12.e3"] {
        assert_eq!(lexed(number), [(Err(LexError), number)], "{number:?}");
    }
    assert_eq!(
        lexed("1.ex 1.e"),
        [
            (Ok(Token::IntLiteral(1)), "1"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("ex")), "ex"),
            (Ok(Token::IntLiteral(1)), "1"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("e")), "e"),
        ]
    );
    // After a name, `.1` is a Float, so `.e3` is a field again.
    assert_eq!(
        lexed("x.1.e3"),
        [
            (Ok(Token::Identifier("x")), "x"),
            (Ok(Token::FloatLiteral(0.1)), ".1"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("e3")), "e3"),
        ]
    );
    // An unclosed String: the error covers its opening quote onward.
    assert_eq!(
        lexed("f('abc"),
        [
            (Ok(Token::Identifier("f")), "f"),
            (Ok(Token::OpenParen), "("),
            (Err(LexError), "'abc"),
        ]
    );
}

// -- Digit separators --
//
// A number's digits may hold `_` separators anywhere after its first digit, as in Rust.
// A separator never changes a value, and never leaves a literal's span.

/// Asserts that `source` lexes to exactly the one `expected` token covering all of it.
#[track_caller]
fn assert_one_token(source: &str, expected: Token<'_>) {
    assert_eq!(
        lexed(source),
        [(Ok(expected), source)],
        "{source:?} should be a single token spanning all of it"
    );
}

/// Asserts that `source` lexes to exactly one error covering all of it.
#[track_caller]
fn assert_one_error(source: &str) {
    assert_eq!(
        lexed(source),
        [(Err(LexError), source)],
        "{source:?} should be one error spanning all of it"
    );
}

#[test]
fn a_decimal_int_ignores_its_separators() {
    for (source, n) in [
        ("1_0", 10),
        ("1_000", 1000),
        ("1_000_000", 1_000_000),
        ("12_34", 1234),
        ("1_2_3_4_5", 12345),
        // Consecutive separators.
        ("1__2", 12),
        ("1___2", 12),
        ("1_0__0", 100),
        // Trailing separators.
        ("1_", 1),
        ("1__", 1),
        ("1___", 1),
        ("42_", 42),
        ("4_2_", 42),
        // Leading zeros, with and without separators.
        ("0_", 0),
        ("0_0", 0),
        ("0__0__", 0),
        ("0_1", 1),
        ("00_7", 7),
        ("0_000_042", 42),
        // Large values.
        ("9_223_372_036_854_775_807", i64::MAX as u64),
        ("9_223_372_036_854_775_808", 1 << 63),
        ("18_446_744_073_709_551_615", u64::MAX),
        ("1_8446_7440_7370_9551_615", u64::MAX),
    ] {
        assert_one_token(source, Token::IntLiteral(n));
    }
}

#[test]
fn a_decimal_int_with_separators_saturates_past_u64_max() {
    for source in [
        "18_446_744_073_709_551_616",
        "18_446_744_073_709_551_616_",
        "99_999_999_999_999_999_999",
        "1_000_000_000_000_000_000_000_000",
        "100000000000000000000_000",
        "0_018_446_744_073_709_551_616",
    ] {
        assert_one_token(source, Token::IntLiteral(u64::MAX));
    }
}

#[test]
fn a_name_may_not_begin_with_a_digit_but_may_begin_with_a_separator() {
    // A leading `_` makes a name, not a number.
    assert_one_token("_1", Token::Identifier("_1"));
    assert_one_token("__1", Token::Identifier("__1"));
    assert_one_token("_1_000", Token::Identifier("_1_000"));
    assert_eq!(
        lexed("_1 2"),
        [
            (Ok(Token::Identifier("_1")), "_1"),
            (Ok(Token::IntLiteral(2)), "2"),
        ]
    );
}

#[test]
fn a_based_int_ignores_its_separators() {
    for (source, n) in [
        // Between digits, in each base.
        ("0b1_0", 0b10),
        ("0b1_0_1_0", 0b1010),
        ("0o1_7", 0o17),
        ("0o7_7_7", 0o777),
        ("0xf_f", 0xff),
        ("0xde_ad_be_ef", 0xdead_beef),
        // Straight after the prefix.
        ("0b_1_0", 2),
        ("0o_17", 15),
        ("0x_ff", 255),
        ("0b__1", 1),
        ("0x__ff", 255),
        // Trailing.
        ("0b1_", 1),
        ("0o7_", 7),
        ("0xff_", 255),
        ("0xF_f_", 255),
        ("0xff__", 255),
        ("0b_1_", 1),
        // Consecutive.
        ("0b1__0", 2),
        ("0o1__7", 15),
        ("0xf___f", 255),
        // Either case of prefix.
        ("0B1_1", 3),
        ("0O1_7", 15),
        ("0X_F_F", 255),
        ("0X_f_F", 255),
        // Mixed-case hex digits.
        ("0xaB_cD", 0xabcd),
        ("0xAb_Cd_Ef", 0xabcdef),
        ("0xA_b_C", 0xabc),
        // Zero.
        ("0x_0", 0),
        ("0b0_0", 0),
        ("0o_0_", 0),
        // The least Int's magnitude and the greatest magnitude.
        ("0x8000_0000_0000_0000", 1 << 63),
        ("0x_8000_0000_0000_0000_", 1 << 63),
        ("0xffff_ffff_ffff_ffff", u64::MAX),
        ("0xFFFF_FFFF_FFFF_FFFF", u64::MAX),
        (
            "0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111",
            u64::MAX,
        ),
        ("0o17_7777_7777_7777_7777_7777", u64::MAX),
        (
            "0b1000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000",
            1 << 63,
        ),
        // Past `u64::MAX`, the magnitude saturates.
        ("0x1_0000_0000_0000_0000", u64::MAX),
        ("0xffff_ffff_ffff_ffff_f", u64::MAX),
        ("0o2_000_000_000_000_000_000_000", u64::MAX),
        (
            "0b1_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000",
            u64::MAX,
        ),
    ] {
        assert_one_token(source, Token::IntLiteral(n));
    }
}

#[test]
fn a_based_int_with_no_digit_is_one_error() {
    // Separators alone are not digits; the whole word is spoiled.
    for source in [
        "0x_", "0x__", "0x___", "0b_", "0b__", "0b___", "0o_", "0o__", "0B_", "0O__", "0X_",
    ] {
        assert_one_error(source);
    }
}

#[test]
fn a_based_int_with_a_bad_digit_among_separators_is_one_error() {
    for source in [
        "0x_g",
        "0x_ff_g",
        "0xf_g_f",
        "0xff_z",
        "0o_8",
        "0o1_8",
        "0o7_9_7",
        "0b_2",
        "0b1_2",
        "0b1_0_2_0",
        "0b1_a",
        "0B_1_x",
        "0O_1_9",
        "0X_G_",
    ] {
        assert_one_error(source);
    }
}

#[test]
fn a_malformed_based_int_stops_at_its_word_end_when_separators_are_involved() {
    // The error covers the word, and lexing resumes at the next non-word character.
    assert_eq!(
        lexed("0x_g+1_0"),
        [
            (Err(LexError), "0x_g"),
            (Ok(Token::OpPlus), "+"),
            (Ok(Token::IntLiteral(10)), "1_0"),
        ]
    );
    assert_eq!(
        lexed("0b_ 0b1_"),
        [(Err(LexError), "0b_"), (Ok(Token::IntLiteral(1)), "0b1_"),]
    );
    assert_eq!(
        lexed("0x_ff.x"),
        [
            (Ok(Token::IntLiteral(255)), "0x_ff"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("x")), "x"),
        ]
    );
}

#[test]
fn a_float_ignores_its_separators() {
    for (source, value) in [
        // The whole part.
        ("1_000.5", "1000.5"),
        ("1_0.5", "10.5"),
        ("1__0.5", "10.5"),
        ("0_0.5", "0.5"),
        // The fraction.
        ("1.5_0", "1.50"),
        ("1.000_1", "1.0001"),
        ("1.0_0_0_1", "1.0001"),
        ("1.5__5", "1.55"),
        // Both.
        ("1_000.000_1", "1000.0001"),
        ("3.141_592_653_589_793", "3.141592653589793"),
        ("12_345.678_9", "12345.6789"),
        // Trailing separators, in either part.
        ("1_.5", "1.5"),
        ("1__.5", "1.5"),
        ("1.5_", "1.5"),
        ("1.5__", "1.5"),
        ("1_.5_", "1.5"),
        // Leading separators in the fraction are not a thing: `1._5` is a field access.
        // Leading-point Floats.
        (".1_0", ".10"),
        (".1_", ".1"),
        (".1__0", ".10"),
        (".0_5", ".05"),
        (".000_1", ".0001"),
        (".5_5_5", ".555"),
    ] {
        let expected: f64 = value.parse().unwrap();
        assert_one_token(source, Token::FloatLiteral(expected));
    }
}

#[test]
fn a_float_exponent_ignores_its_separators() {
    for (source, value) in [
        // No point; the exponent is what makes it a Float.
        ("1e1_0", "1e10"),
        ("1e_5", "1e5"),
        ("1e+_5", "1e5"),
        ("1e-_5", "1e-5"),
        ("1e5_", "1e5"),
        ("1e+5_", "1e5"),
        ("1e-1_0", "1e-10"),
        ("1e1__0", "1e10"),
        ("1e__1", "1e1"),
        ("1_e5", "1e5"),
        ("1__e5", "1e5"),
        ("1_0e1_0", "1e11"),
        ("1_0_e_1_0", "1e11"),
        ("1E5", "1e5"),
        ("1E_5", "1e5"),
        ("1E+_5", "1e5"),
        ("1E-_5_", "1e-5"),
        ("1_0E1_0", "1e11"),
        // With a point.
        ("1.5e1_0", "1.5e10"),
        ("1.5e_2", "1.5e2"),
        ("1.5e+_2", "1.5e2"),
        ("1.5e-_2", "1.5e-2"),
        ("1.5E_2", "1.5e2"),
        ("1.5E-_2_", "1.5e-2"),
        ("1_0.2_5e+1_0", "10.25e10"),
        ("1_.5_e_2", "1.5e2"),
        // Leading point.
        (".5e_2", ".5e2"),
        (".5e+_2", ".5e2"),
        (".5e-_2", ".5e-2"),
        (".5E_2", ".5e2"),
        (".5e1_0", ".5e10"),
        (".5_e2_", ".5e2"),
        (".1_0e-1_0", ".10e-10"),
    ] {
        let expected: f64 = value.parse().unwrap();
        assert_one_token(source, Token::FloatLiteral(expected));
    }
}

#[test]
fn an_exponent_with_no_digit_is_one_error_covering_the_literal() {
    for source in [
        // Separators alone are not digits.
        "1e_", "1e__", "1e+_", "1e-_", "1e+__", "1E_", "1E-_", "1_e_", "1_0e_",
        // With a point.
        "1.5e_", "1.5E-_", "1.5e+__", "1_0.5_e_", // Leading point.
        ".5e_", ".5E_", ".5e+_", ".5e-__", ".1_0e_",
    ] {
        assert_one_error(source);
    }
}

#[test]
fn a_point_then_an_exponent_is_one_error_with_or_without_separators() {
    for source in [
        "1.e3", "1_.e3", "1__.e3", "1.e_3", "1.e_", "1.e+_", "1_.E-3_", "1_0.e1_0",
    ] {
        assert_one_error(source);
    }
}

#[test]
fn lexing_resumes_after_a_malformed_exponent() {
    assert_eq!(
        lexed("1e_ + 2"),
        [
            (Err(LexError), "1e_"),
            (Ok(Token::OpPlus), "+"),
            (Ok(Token::IntLiteral(2)), "2"),
        ]
    );
    assert_eq!(
        lexed("1.5e+__)"),
        [(Err(LexError), "1.5e+__"), (Ok(Token::CloseParen), ")")]
    );
    assert_eq!(
        lexed(".5e_,x"),
        [
            (Err(LexError), ".5e_"),
            (Ok(Token::Comma), ","),
            (Ok(Token::Identifier("x")), "x"),
        ]
    );
    assert_eq!(
        lexed("1_.e3 7"),
        [(Err(LexError), "1_.e3"), (Ok(Token::IntLiteral(7)), "7"),]
    );
}

#[test]
fn an_exponent_marker_without_digits_or_separators_is_not_part_of_a_number() {
    // `e` alone, with nothing numeric after it, begins a name.
    assert_eq!(
        lexed("1e"),
        [
            (Ok(Token::IntLiteral(1)), "1"),
            (Ok(Token::Identifier("e")), "e"),
        ]
    );
    assert_eq!(
        lexed("1_e"),
        [
            (Ok(Token::IntLiteral(1)), "1_"),
            (Ok(Token::Identifier("e")), "e"),
        ]
    );
    assert_eq!(
        lexed("1.e"),
        [
            (Ok(Token::IntLiteral(1)), "1"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("e")), "e"),
        ]
    );
}

#[test]
fn a_separator_after_a_point_is_a_field_access() {
    assert_eq!(
        lexed("1._5"),
        [
            (Ok(Token::IntLiteral(1)), "1"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("_5")), "_5"),
        ]
    );
    assert_eq!(
        lexed("._5"),
        [(Ok(Token::OpDot), "."), (Ok(Token::Identifier("_5")), "_5"),]
    );
    assert_eq!(
        lexed("x._5"),
        [
            (Ok(Token::Identifier("x")), "x"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("_5")), "_5"),
        ]
    );
}

#[test]
fn a_point_after_a_name_then_digits_is_a_float_with_separators() {
    assert_eq!(
        lexed("x.1_0"),
        [
            (Ok(Token::Identifier("x")), "x"),
            (Ok(Token::FloatLiteral(0.10)), ".1_0"),
        ]
    );
    assert_eq!(
        lexed("x.1_0.e3"),
        [
            (Ok(Token::Identifier("x")), "x"),
            (Ok(Token::FloatLiteral(0.10)), ".1_0"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("e3")), "e3"),
        ]
    );
}

#[test]
fn a_point_after_an_int_with_separators_is_a_field_access() {
    assert_eq!(
        lexed("1_.x"),
        [
            (Ok(Token::IntLiteral(1)), "1_"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("x")), "x"),
        ]
    );
    assert_eq!(
        lexed("1_000.len"),
        [
            (Ok(Token::IntLiteral(1000)), "1_000"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("len")), "len"),
        ]
    );
    assert_eq!(
        lexed("1__.ex"),
        [
            (Ok(Token::IntLiteral(1)), "1__"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("ex")), "ex"),
        ]
    );
    assert_eq!(
        lexed("0x_ff.len"),
        [
            (Ok(Token::IntLiteral(255)), "0x_ff"),
            (Ok(Token::OpDot), "."),
            (Ok(Token::Identifier("len")), "len"),
        ]
    );
}

#[test]
fn separators_touching_operators_end_the_number() {
    assert_eq!(
        lexed("1_+2"),
        [
            (Ok(Token::IntLiteral(1)), "1_"),
            (Ok(Token::OpPlus), "+"),
            (Ok(Token::IntLiteral(2)), "2"),
        ]
    );
    assert_eq!(
        lexed("1_0-_2"),
        [
            (Ok(Token::IntLiteral(10)), "1_0"),
            (Ok(Token::OpMinus), "-"),
            (Ok(Token::Identifier("_2")), "_2"),
        ]
    );
    assert_eq!(
        lexed("1_000.5*0b1_1"),
        [
            (Ok(Token::FloatLiteral(1000.5)), "1_000.5"),
            (Ok(Token::OpTimes), "*"),
            (Ok(Token::IntLiteral(3)), "0b1_1"),
        ]
    );
}

#[test]
fn numbers_with_separators_lex_inside_brackets_and_calls() {
    assert_eq!(
        lexed("f(1_000, 2.5_0)"),
        [
            (Ok(Token::Identifier("f")), "f"),
            (Ok(Token::OpenParen), "("),
            (Ok(Token::IntLiteral(1000)), "1_000"),
            (Ok(Token::Comma), ","),
            (Ok(Token::FloatLiteral(2.5)), "2.5_0"),
            (Ok(Token::CloseParen), ")"),
        ]
    );
    assert_eq!(
        lexed("[0xf_f,1e1_0]"),
        [
            (Ok(Token::OpenBracket), "["),
            (Ok(Token::IntLiteral(255)), "0xf_f"),
            (Ok(Token::Comma), ","),
            (Ok(Token::FloatLiteral(1e10)), "1e1_0"),
            (Ok(Token::CloseBracket), "]"),
        ]
    );
}

#[test]
fn a_sequence_of_separated_literals_keeps_each_span_exact() {
    let source = "1_0 0b_1 1_.5 .5_ 0x_ 1e_5 1e_ 2__";
    assert_eq!(
        lexed(source),
        [
            (Ok(Token::IntLiteral(10)), "1_0"),
            (Ok(Token::IntLiteral(1)), "0b_1"),
            (Ok(Token::FloatLiteral(1.5)), "1_.5"),
            (Ok(Token::FloatLiteral(0.5)), ".5_"),
            (Err(LexError), "0x_"),
            (Ok(Token::FloatLiteral(1e5)), "1e_5"),
            (Err(LexError), "1e_"),
            (Ok(Token::IntLiteral(2)), "2__"),
        ]
    );
    let spans: Vec<SourceSpan> = tokens(source).map(|(_, span)| span).collect();
    let expected_starts = [0, 4, 9, 14, 18, 22, 27, 31];
    assert_eq!(
        spans.iter().map(|span| span.start).collect::<Vec<_>>(),
        expected_starts,
        "each literal should start where its text does"
    );
}
