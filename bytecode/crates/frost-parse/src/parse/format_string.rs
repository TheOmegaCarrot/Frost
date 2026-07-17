use std::ops::Range;

use crate::ast::{Expr, FormatSegment, Spanned};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::strings::QuoteStyle;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

pub fn parse_format_string(ctx: &mut ParseCtx, quote: QuoteStyle) -> ParseResult<Spanned<Expr>> {
    let peek = ctx.must_peek("format String")?;
    let span = peek.span.clone();
    let raw = match peek.token {
        Token::SingleQuoteFormatStringLiteral(s) | Token::DoubleQuoteFormatStringLiteral(s) => {
            s.to_owned()
        }
        _ => return Err(ctx.unexpected_token(peek, "format String")),
    };
    ctx.advance(1);

    let segments = split_format_segments(&raw, quote, ctx, &span)?;

    Ok(Spanned::new(Expr::FormatString(segments), span.into()))
}

fn format_error(span: &Range<usize>, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::at(msg, span.clone().into(), "in this format String")
}

fn split_format_segments(
    raw: &str,
    quote: QuoteStyle,
    ctx: &ParseCtx,
    span: &Range<usize>,
) -> ParseResult<Vec<FormatSegment>> {
    let bytes = raw.as_bytes();
    let mut segments = Vec::new();
    let mut literal_buf = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if i + 1 >= bytes.len() {
                    return Err(format_error(
                        span,
                        "unexpected end of String after backslash",
                    ));
                }
                let escape = bytes[i + 1];
                match escape {
                    b'n' => {
                        literal_buf.push(b'\n');
                        i += 2;
                    }
                    b't' => {
                        literal_buf.push(b'\t');
                        i += 2;
                    }
                    b'r' => {
                        literal_buf.push(b'\r');
                        i += 2;
                    }
                    b'\\' => {
                        literal_buf.push(b'\\');
                        i += 2;
                    }
                    b'0' => {
                        literal_buf.push(0);
                        i += 2;
                    }
                    b'$' => {
                        literal_buf.push(b'$');
                        i += 2;
                    }
                    b'\'' if matches!(quote, QuoteStyle::Single) => {
                        literal_buf.push(b'\'');
                        i += 2;
                    }
                    b'"' if matches!(quote, QuoteStyle::Double) => {
                        literal_buf.push(b'"');
                        i += 2;
                    }
                    b'x' => {
                        if i + 3 >= bytes.len() {
                            return Err(format_error(span, "incomplete \\x escape"));
                        }
                        let hex = &raw[i + 2..i + 4];
                        let val = u8::from_str_radix(hex, 16)
                            .map_err(|_| format_error(span, "invalid hex escape"))?;
                        literal_buf.push(val);
                        i += 4;
                    }
                    _ => {
                        return Err(format_error(
                            span,
                            format!("invalid escape sequence: \\{}", escape as char),
                        ));
                    }
                }
            }

            b'$' if i + 1 < bytes.len() && bytes[i + 1] == b'{' => {
                // Flush literal buffer
                if !literal_buf.is_empty() {
                    segments.push(FormatSegment::Literal(std::mem::take(&mut literal_buf)));
                }

                // Extract interpolation content (brace-depth balanced)
                i += 2; // skip ${
                let start = i;
                let mut depth = 1u32;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        q @ (b'\'' | b'"') => {
                            i += 1;
                            while i < bytes.len() {
                                match bytes[i] {
                                    b'\\' if i + 1 < bytes.len() => i += 2,
                                    c if c == q => {
                                        i += 1;
                                        break;
                                    }
                                    _ => i += 1,
                                }
                            }
                            continue;
                        }
                        _ => {}
                    }
                    i += 1;
                }

                if depth != 0 {
                    return Err(format_error(
                        span,
                        "unclosed interpolation in format String",
                    ));
                }

                // The content is between start and i-1 (i is past the closing })
                let interp_src = &raw[start..i - 1];

                // Re-lex and parse the interpolation content as an expression.
                // Offset: token start + 2 ($' prefix) + start (position of content after ${)
                let interp_offset = span.start + 2 + start;
                let expr = parse_interpolation(interp_src, ctx, span, interp_offset)?;
                segments.push(FormatSegment::Interpolation(expr));
            }

            b'$' => {
                // A bare $ not followed by { is a literal
                literal_buf.push(b'$');
                i += 1;
            }

            _ => {
                literal_buf.push(bytes[i]);
                i += 1;
            }
        }
    }

    // Flush remaining literal
    if !literal_buf.is_empty() {
        segments.push(FormatSegment::Literal(literal_buf));
    }

    Ok(segments)
}

fn parse_interpolation(
    src: &str,
    ctx: &ParseCtx,
    span: &Range<usize>,
    base_offset: usize,
) -> ParseResult<Spanned<Expr>> {
    // The sub-context lexes `src` with `base_offset`, so every diagnostic it
    // produces already carries whole-source spans. We propagate those inner
    // diagnostics directly (adding an outer "in this format String" label for
    // context) rather than flattening them to text, so their labels survive
    // and render against the real source.
    let context = |d: Diagnostic| d.with_label(span.clone().into(), "in this format String");

    let mut sub_ctx =
        ParseCtx::new_with_offset(ctx.filename(), src, base_offset).map_err(context)?;

    let expr = parse_expression(&mut sub_ctx).map_err(context)?;

    if !sub_ctx.at_end() {
        let leftover = sub_ctx.peek().expect("not at end, so a token remains");
        return Err(context(
            sub_ctx.unexpected_token(leftover, "interpolation expression"),
        ));
    }

    Ok(expr)
}
