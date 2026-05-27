use std::ops::Range;

use miette::{LabeledSpan, miette};

use crate::ast::{Expr, ExprKind, Literal};
use crate::lex::Token;
use crate::parse::{ParseError, ParseResult, ctx::ParseCtx};

#[derive(Clone, Copy)]
pub enum QuoteStyle {
    Single,
    Double,
}

fn string_error(ctx: &ParseCtx, span: &Range<usize>, msg: impl Into<String>) -> ParseError {
    miette!(
        labels = vec![LabeledSpan::at(span.clone(), "in this String literal")],
        "{}",
        msg.into()
    )
    .with_source_code(ctx.named_source())
    .into()
}

pub fn parse_simple_string(ctx: &mut ParseCtx, quote: QuoteStyle) -> ParseResult<Expr> {
    let peek = ctx.must_peek("String literal")?;
    let span = peek.span.clone();
    let raw = match peek.token {
        Token::SingleQuoteStringLiteral(s) | Token::DoubleQuoteStringLiteral(s) => s.to_owned(),
        _ => return Err(ctx.unexpected_token(peek, "String literal")),
    };
    ctx.advance(1);
    let bytes = expand_escapes(&raw, quote).map_err(|msg| string_error(ctx, &span, msg))?;
    Ok(Expr {
        span: span.into(),
        kind: ExprKind::Literal(Literal::String(bytes)),
    })
}

pub fn parse_raw_string(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let peek = ctx.must_peek("raw String literal")?;
    let span = peek.span.clone();
    let raw = match peek.token {
        Token::RawStringLiteral(s) => s,
        _ => return Err(ctx.unexpected_token(peek, "raw String literal")),
    };
    let bytes = raw.as_bytes().to_vec();
    ctx.advance(1);
    Ok(Expr {
        span: span.into(),
        kind: ExprKind::Literal(Literal::String(bytes)),
    })
}

pub fn parse_multiline_string(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let peek = ctx.must_peek("multiline String literal")?;
    let span = peek.span.clone();
    let raw = match peek.token {
        Token::MultilineStringLiteral(s) => s.to_owned(),
        _ => return Err(ctx.unexpected_token(peek, "multiline String literal")),
    };
    ctx.advance(1);
    let trimmed = trim_multiline_indentation(&raw).map_err(|msg| string_error(ctx, &span, msg))?;
    let bytes = expand_multiline_escapes(&trimmed).map_err(|msg| string_error(ctx, &span, msg))?;
    Ok(Expr {
        span: span.into(),
        kind: ExprKind::Literal(Literal::String(bytes)),
    })
}

fn expand_escapes(raw: &str, quote: QuoteStyle) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(raw.len());
    let mut chars = raw.bytes().enumerate();

    while let Some((_, b)) = chars.next() {
        if b != b'\\' {
            out.push(b);
            continue;
        }

        let Some((_, escape)) = chars.next() else {
            return Err("unexpected end of String after backslash".into());
        };

        match escape {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'\\' => out.push(b'\\'),
            b'0' => out.push(0),
            b'\'' if matches!(quote, QuoteStyle::Single) => out.push(b'\''),
            b'"' if matches!(quote, QuoteStyle::Double) => out.push(b'"'),
            b'x' => {
                let hi = chars.next().map(|(_, b)| b);
                let lo = chars.next().map(|(_, b)| b);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        let hex = [h, l];
                        let s = std::str::from_utf8(&hex)
                            .map_err(|_| "invalid hex escape".to_owned())?;
                        let val = u8::from_str_radix(s, 16)
                            .map_err(|_| "invalid hex escape".to_owned())?;
                        out.push(val);
                    }
                    _ => return Err("incomplete \\x escape".into()),
                }
            }
            _ => {
                return Err(format!("invalid escape sequence: \\{}", escape as char));
            }
        }
    }

    Ok(out)
}

fn expand_multiline_escapes(raw: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(raw.len());
    let mut chars = raw.bytes().enumerate();

    while let Some((_, b)) = chars.next() {
        if b != b'\\' {
            out.push(b);
            continue;
        }

        let Some((_, escape)) = chars.next() else {
            return Err("unexpected end of String after backslash".into());
        };

        match escape {
            b'\\' => out.push(b'\\'),
            b't' => out.push(b'\t'),
            b'0' => out.push(0),
            b'\'' => out.push(b'\''),
            b'"' => out.push(b'"'),
            _ => {
                return Err(format!(
                    "invalid escape sequence in multiline String: \\{}",
                    escape as char
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
        if line.is_empty() {
            trimmed.push("");
        } else if line.len() >= indent && line[..indent].chars().all(|c| c == ' ' || c == '\t') {
            trimmed.push(&line[indent..]);
        } else {
            return Err(
                "multiline String content is indented less than the closing delimiter".into(),
            );
        }
    }

    Ok(trimmed.join("\n"))
}
