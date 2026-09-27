use std::ops::Range;

use crate::ast::{Expr, FormatSegment, Spanned};
use crate::lex::{Token, skip_interpolation};
use crate::parse::strings::QuoteStyle;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_format_string(&mut self, quote: QuoteStyle) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("format String")?;
        let span = peek.span.clone();
        let raw = match peek.token {
            Token::SingleQuoteFormatStringLiteral(s) | Token::DoubleQuoteFormatStringLiteral(s) => {
                s.to_owned()
            }
            _ => return Err(self.unexpected_token(peek, "format String")),
        };
        self.advance(1);

        let segments = split_format_segments(&raw, quote, self, &span)?;

        Ok(Spanned::new(Expr::FormatString(segments), span.into()))
    }
}

fn format_error(span: &Range<usize>, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::at(msg, span.clone().into(), "in this format String")
}

fn split_format_segments(
    raw: &str,
    quote: QuoteStyle,
    ctx: &mut ParseCtx,
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
                    b'u' => {
                        // \u{NN..}: braces around hex digits naming a Unicode scalar.
                        // Slicing on the ASCII `{` and `}` never splits a character.
                        if bytes.get(i + 2) != Some(&b'{') {
                            return Err(format_error(span, "\\u escape must be followed by '{'"));
                        }
                        let mut j = i + 3;
                        while j < bytes.len() && bytes[j] != b'}' {
                            j += 1;
                        }
                        if j >= bytes.len() {
                            return Err(format_error(span, "unterminated \\u escape"));
                        }
                        let ch = crate::parse::strings::decode_unicode_escape(&raw[i + 3..j])
                            .map_err(|msg| format_error(span, msg))?;
                        let mut utf8 = [0u8; 4];
                        literal_buf.extend_from_slice(ch.encode_utf8(&mut utf8).as_bytes());
                        i = j + 1;
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
                if !literal_buf.is_empty() {
                    segments.push(FormatSegment::Literal(finish_literal(std::mem::take(
                        &mut literal_buf,
                    ))));
                }

                let start = i + 2; // past `${`
                i = skip_interpolation(bytes, start)
                    .ok_or_else(|| format_error(span, "unclosed interpolation in format String"))?;
                // `i` is past the closing `}`.
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

    if !literal_buf.is_empty() {
        segments.push(FormatSegment::Literal(finish_literal(literal_buf)));
    }

    Ok(segments)
}

/// Each literal run between interpolations is UTF-8 by construction: the raw source
/// is valid UTF-8, every escape yields a whole scalar, and a flush only happens at
/// `${` (an ASCII boundary), so no character is ever split.
fn finish_literal(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("format String literal is UTF-8 by construction")
}

fn parse_interpolation(
    src: &str,
    ctx: &mut ParseCtx,
    span: &Range<usize>,
    base_offset: usize,
) -> ParseResult<Spanned<Expr>> {
    // The sub-context lexes `src` with `base_offset`, so its diagnostics already carry whole-source spans.
    // Propagate them as-is, plus an outer "in this format String" label, so their labels render against the real source.
    let context = |d: Diagnostic| d.with_label(span.clone().into(), "in this format String");

    let mut sub_ctx =
        ParseCtx::new_with_offset(ctx.filename(), src, base_offset).map_err(context)?;

    // An interpolation is lexed separately but sits lexically inside any enclosing
    // abbreviated lambda, so the sub-context parses with the outer frames in hand:
    // `$n` is accepted there, and marks land in the enclosing lambda's frame rather
    // than being lost (which would under-report `used_params`).
    sub_ctx.restore_abbrev_frames(ctx.take_abbrev_frames());
    let parsed = sub_ctx.parse_expression();
    ctx.restore_abbrev_frames(sub_ctx.take_abbrev_frames());

    let expr = parsed.map_err(context)?;

    if !sub_ctx.at_end() {
        let leftover = sub_ctx.peek().expect("not at end, so a token remains");
        return Err(context(
            sub_ctx.unexpected_token(leftover, "interpolation expression"),
        ));
    }

    Ok(expr)
}
