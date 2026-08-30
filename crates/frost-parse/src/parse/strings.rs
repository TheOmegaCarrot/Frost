use std::ops::Range;

use crate::ast::{Expr, Literal, Spanned};
use crate::lex::Token;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

#[derive(Clone, Copy)]
pub(crate) enum QuoteStyle {
    Single,
    Double,
}

fn string_error(span: &Range<usize>, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::at(msg, span.clone().into(), "in this String literal")
}

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_simple_string(&mut self, quote: QuoteStyle) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("String literal")?;
        let span = peek.span.clone();
        let raw = match peek.token {
            Token::SingleQuoteStringLiteral(s) | Token::DoubleQuoteStringLiteral(s) => s.to_owned(),
            _ => return Err(self.unexpected_token(peek, "String literal")),
        };
        self.advance(1);
        let text = expand_escapes(&raw, quote).map_err(|msg| string_error(&span, msg))?;
        Ok(Spanned::new(
            Expr::Literal(Literal::String(text)),
            span.into(),
        ))
    }

    pub(crate) fn parse_raw_string(&mut self) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("raw String literal")?;
        let span = peek.span.clone();
        let Token::RawStringLiteral(raw) = peek.token else {
            return Err(self.unexpected_token(peek, "raw String literal"));
        };
        // A raw string is source text taken verbatim, so it is already valid UTF-8:
        // it has no escapes that could introduce anything else.
        let text = raw.to_owned();
        self.advance(1);
        Ok(Spanned::new(
            Expr::Literal(Literal::String(text)),
            span.into(),
        ))
    }

    pub(crate) fn parse_bytes_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("Bytes literal")?;
        let span = peek.span.clone();
        let Token::BytesLiteral(raw) = peek.token else {
            return Err(self.unexpected_token(peek, "Bytes literal"));
        };
        self.advance(1);
        Ok(Spanned::new(
            Expr::Literal(Literal::Bytes(decode_bytes_literal(raw))),
            span.into(),
        ))
    }

    pub(crate) fn parse_multiline_string(&mut self) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("multiline String literal")?;
        let span = peek.span.clone();
        let raw = match peek.token {
            Token::MultilineStringLiteral(s) => s.to_owned(),
            _ => return Err(self.unexpected_token(peek, "multiline String literal")),
        };
        self.advance(1);
        let trimmed = trim_multiline_indentation(&raw).map_err(|msg| string_error(&span, msg))?;
        let text = expand_multiline_escapes(&trimmed).map_err(|msg| string_error(&span, msg))?;
        Ok(Spanned::new(
            Expr::Literal(Literal::String(text)),
            span.into(),
        ))
    }
}

/// Decodes the hex body of a `\u{...}` escape into its Unicode scalar value.
///
/// Rust's `\u{...}` takes 1 to 6 hex digits; anything else in the body (including the
/// leading `+` that [`u32::from_str_radix`] would tolerate) is rejected here, as is an
/// empty body. [`char::from_u32`] rejects surrogates and values above U+10FFFF.
pub(crate) fn decode_unicode_escape(hex: &str) -> Result<char, String> {
    if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("invalid \\u escape: \\u{{{hex}}}"));
    }
    if hex.len() > 6 {
        return Err(format!(
            "\\u escape has more than 6 hex digits: \\u{{{hex}}}"
        ));
    }
    let code = u32::from_str_radix(hex, 16).expect("body is 1-6 hex digits");
    char::from_u32(code).ok_or_else(|| format!("\\u{{{hex}}} is not a valid Unicode scalar value"))
}

/// Decodes the body of an `x'..'` Bytes literal into its octets. The lexer's regex
/// guarantees an even count of hex digits, so [`u8::from_str_radix`] cannot fail.
fn decode_bytes_literal(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("lexer guarantees hex digit pairs"))
        .collect()
}

fn expand_escapes(raw: &str, quote: QuoteStyle) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        let Some(escape) = chars.next() else {
            return Err("unexpected end of String after backslash".into());
        };

        match escape {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            '\\' => out.push('\\'),
            '0' => out.push('\0'),
            '\'' if matches!(quote, QuoteStyle::Single) => out.push('\''),
            '"' if matches!(quote, QuoteStyle::Double) => out.push('"'),
            'u' => out.push(take_unicode_escape(&mut chars)?),
            _ => return Err(format!("invalid escape sequence: \\{escape}")),
        }
    }

    Ok(out)
}

/// Reads the `{NN..}` body after a `\u` (already consumed) and decodes its scalar.
/// The next character must be `{`; digits run to the closing `}`.
fn take_unicode_escape(chars: &mut std::str::Chars) -> Result<char, String> {
    if chars.next() != Some('{') {
        return Err("\\u escape must be followed by '{'".into());
    }
    let mut hex = String::new();
    loop {
        match chars.next() {
            Some('}') => break,
            Some(c) => hex.push(c),
            None => return Err("unterminated \\u escape".into()),
        }
    }
    decode_unicode_escape(&hex)
}

fn expand_multiline_escapes(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        let Some(escape) = chars.next() else {
            return Err("unexpected end of String after backslash".into());
        };

        match escape {
            '\\' => out.push('\\'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            '\'' => out.push('\''),
            '"' => out.push('"'),
            'u' => out.push(take_unicode_escape(&mut chars)?),
            _ => {
                return Err(format!(
                    "invalid escape sequence in multiline String: \\{escape}"
                ));
            }
        }
    }

    Ok(out)
}

fn trim_multiline_indentation(raw: &str) -> Result<String, String> {
    let Some(content) = raw.strip_prefix('\n') else {
        return Err(
            "multiline String must begin with a newline after the opening delimiter".into(),
        );
    };

    // The content must end with a newline followed by the closing delimiter's
    // indentation (whitespace only). Split on the last newline.
    // Special case: if the content is whitespace-only, the string is empty.
    let Some(last_nl) = content.rfind('\n') else {
        if content.chars().all(|c| c == ' ' || c == '\t') {
            return Ok(String::new());
        }
        return Err("closing delimiter of multiline String must be on its own line".into());
    };

    let body = &content[..last_nl];
    let closing_indent = &content[last_nl + 1..];

    if !closing_indent.chars().all(|c| c == ' ' || c == '\t') {
        return Err("closing delimiter of multiline String must be on its own line".into());
    }

    let indent = closing_indent.len();

    if body.is_empty() {
        return Ok(String::new());
    }

    let mut trimmed = Vec::new();
    for line in body.split('\n') {
        // The prefix is checked as bytes: space and tab are single-byte ASCII, so when all
        // `indent` leading bytes are one of them, `indent` is a char boundary and the slice
        // below is safe. A multibyte character inside the prefix fails the check instead of
        // panicking the slice.
        if line.is_empty() {
            trimmed.push("");
        } else if line.len() >= indent
            && line.as_bytes()[..indent]
                .iter()
                .all(|&b| b == b' ' || b == b'\t')
        {
            trimmed.push(&line[indent..]);
        } else {
            return Err(
                "multiline String content is indented less than the closing delimiter".into(),
            );
        }
    }

    Ok(trimmed.join("\n"))
}
