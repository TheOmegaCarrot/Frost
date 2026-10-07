use crate::ast::{Expr, FormatSegment, SourceSpan, Spanned};
use crate::lex::{Token, skip_interpolation};
use crate::parse::strings::{EscapingString, QuoteStyle, decode_escape};
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

impl<'src> ParseCtx<'src> {
    pub(crate) fn parse_format_string(&mut self, quote: QuoteStyle) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("a format String")?;
        let span = peek.span.clone();
        let (Token::SingleQuoteFormatStringLiteral(raw)
        | Token::DoubleQuoteFormatStringLiteral(raw)) = peek.token
        else {
            return Err(self.expected("a format String", peek));
        };
        self.advance(1);

        // The text starts past the opening `$'` or `$"`.
        let segments = split_format_segments(raw, quote, self, span.start + 2)?;

        Ok(Spanned::new(Expr::FormatString(segments), span.into()))
    }
}

/// Splits `raw`, the text of a format String, into its segments.
/// `raw` starts at `text_start` in the whole source.
fn split_format_segments(
    raw: &str,
    quote: QuoteStyle,
    ctx: &mut ParseCtx,
    text_start: usize,
) -> ParseResult<Vec<FormatSegment>> {
    let bytes = raw.as_bytes();
    let mut segments = Vec::new();
    let mut literal_buf = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                let (c, next) = decode_escape(raw, i, EscapingString::Format(quote), text_start)?;
                literal_buf.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                i = next;
            }

            b'$' if i + 1 < bytes.len() && bytes[i + 1] == b'{' => {
                if !literal_buf.is_empty() {
                    segments.push(FormatSegment::Literal(finish_literal(std::mem::take(
                        &mut literal_buf,
                    ))));
                }

                let open = text_start + i;
                let start = i + 2; // past `${`
                i = skip_interpolation(bytes, start)
                    .expect("IMPOSSIBLE: the lexer closes every interpolation");
                // `i` is past the closing `}`.
                let interp_src = &raw[start..i - 1];

                // Re-lex and parse the interpolation content as an expression.
                let expr = parse_interpolation(interp_src, ctx, (open..open + 2).into())?;
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

/// Parses `src`, the text of the interpolation that `open`, its `${`, starts.
fn parse_interpolation(
    src: &str,
    ctx: &mut ParseCtx,
    open: SourceSpan,
) -> ParseResult<Spanned<Expr>> {
    // The sub-context lexes `src` with its offset, so its diagnostics already carry
    // whole-source spans. They gain a label on the `${` rather than the whole String,
    // since a label containing the error's own labels on one line does not render
    // reliably.
    let context = |d: Diagnostic| d.in_interpolation(open);

    let mut sub_ctx = ParseCtx::new_interpolation(src, open.end).map_err(context)?;

    // An interpolation is lexed separately but sits lexically inside any enclosing
    // abbreviated lambda, so the sub-context parses with the outer frames in hand:
    // `$n` is accepted there, and marks land in the enclosing lambda's frame rather
    // than being lost (which would under-report `used_params`).
    // A leftover token's error is made while the frames are still lent, so its help sees them.
    sub_ctx.restore_abbrev_frames(ctx.take_abbrev_frames());
    let parsed = sub_ctx
        .parse_expression()
        .and_then(|expr| match sub_ctx.peek() {
            Some(leftover) => Err(sub_ctx.expected("`}`", leftover)),
            None => Ok(expr),
        });
    ctx.restore_abbrev_frames(sub_ctx.take_abbrev_frames());

    parsed.map_err(context)
}
