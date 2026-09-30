//! `tokens`, the lexer's public face: source to tokens with byte spans.

use frost_parse::{LexError, Token, tokens};

/// Each token of `source` with the text its span covers.
fn lexed(source: &str) -> Vec<(Result<Token<'_>, LexError>, &str)> {
    tokens(source)
        .map(|(token, span)| (token, &source[span]))
        .collect()
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
