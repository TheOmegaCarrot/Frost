use std::ops::Range;

use crate::ast::{Expr, Literal, SourceSpan, Spanned};
use crate::lex::Token;
use crate::parse::hints::code_point;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

#[derive(Clone, Copy)]
pub(crate) enum QuoteStyle {
    Single,
    Double,
}

impl QuoteStyle {
    fn char(self) -> char {
        match self {
            Self::Single => '\'',
            Self::Double => '"',
        }
    }
}

/// A kind of String literal that takes escapes; each kind accepts its own set.
#[derive(Clone, Copy)]
pub(crate) enum EscapingString {
    Quoted(QuoteStyle),
    Format(QuoteStyle),
    Multiline,
}

/// Every escape but `\u{...}`: the character after the `\`, and the character the
/// escape stands for. Which Strings accept each is [`EscapingString::accepts`].
pub(crate) const ESCAPES: [(char, char); 8] = [
    ('n', '\n'),
    ('r', '\r'),
    ('t', '\t'),
    ('\\', '\\'),
    ('0', '\0'),
    ('$', '$'),
    ('\'', '\''),
    ('"', '"'),
];

const UNICODE_ESCAPE_HELP: &str =
    r"a Unicode escape holds 1 to 6 hex digits in braces, like `\u{e9}`";

impl EscapingString {
    /// Whether this kind of String accepts the escape of `c`, an [`ESCAPES`] character.
    fn accepts(self, c: char) -> bool {
        match (self, c) {
            // A multiline String holds real line breaks, and only a format String
            // interpolates, so only it needs `\$`.
            (Self::Multiline, 'n' | 'r') | (Self::Quoted(_) | Self::Multiline, '$') => false,
            // A quoted String escapes only its own quote; a multiline String, either.
            (Self::Quoted(quote) | Self::Format(quote), '\'' | '"') => c == quote.char(),
            _ => true,
        }
    }

    /// The character the escape `\c` stands for here, for every escape but `\u{...}`.
    fn escaped(self, c: char) -> Option<char> {
        ESCAPES
            .iter()
            .find(|&&(escape, _)| escape == c && self.accepts(c))
            .map(|&(_, stands_for)| stands_for)
    }

    /// The escapes this kind of String accepts, as a help lists them.
    fn escapes_help(self) -> String {
        let what = match self {
            Self::Quoted(_) => "this String",
            Self::Format(_) => "this format String",
            Self::Multiline => "this multiline String",
        };
        let escapes: Vec<String> = ESCAPES
            .iter()
            .filter(|&&(escape, _)| self.accepts(escape))
            .map(|&(escape, _)| format!(r"`\{escape}`"))
            .collect();
        format!(
            r"{what}'s escapes are {}, and `\u{{...}}`",
            escapes.join(", ")
        )
    }

    /// What to write instead of `\c`, an escape of another kind of String, where
    /// this kind has a plain way to say it.
    fn instead_of(self, c: char) -> Option<String> {
        match (self, c) {
            (Self::Multiline, 'n') => Some(r"it takes real line breaks instead of `\n`".to_owned()),
            (Self::Quoted(_) | Self::Multiline, '$') => {
                Some("`$` needs no backslash outside a format String".to_owned())
            }
            (Self::Quoted(quote) | Self::Format(quote), '\'' | '"') => Some(format!(
                "`{c}` needs no backslash in a `{}` String",
                quote.char()
            )),
            _ => None,
        }
    }
}

/// The source span of `range`, a range of a literal's text that starts at
/// `text_start` in the whole source.
fn in_source(text_start: usize, range: Range<usize>) -> SourceSpan {
    (text_start + range.start..text_start + range.end).into()
}

fn string_error(span: &Range<usize>, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::at(msg, span.clone().into(), "in this String literal")
}

impl<'src> ParseCtx<'src> {
    pub(crate) fn parse_simple_string(&mut self, quote: QuoteStyle) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("a String literal")?;
        let span = peek.span.clone();
        let (Token::SingleQuoteStringLiteral(raw) | Token::DoubleQuoteStringLiteral(raw)) =
            peek.token
        else {
            return Err(self.expected("a String literal", peek));
        };
        self.advance(1);
        // The text starts past the opening quote.
        let text = expand_escapes(raw, EscapingString::Quoted(quote), span.start + 1)?;
        Ok(Spanned::new(
            Expr::Literal(Literal::String(text)),
            span.into(),
        ))
    }

    pub(crate) fn parse_raw_string(&mut self) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("a raw String literal")?;
        let span = peek.span.clone();
        let Token::RawStringLiteral(raw) = peek.token else {
            return Err(self.expected("a raw String literal", peek));
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
        let peek = self.must_peek("a Bytes literal")?;
        let span = peek.span.clone();
        let Token::BytesLiteral(raw) = peek.token else {
            return Err(self.expected("a Bytes literal", peek));
        };
        self.advance(1);
        Ok(Spanned::new(
            Expr::Literal(Literal::Bytes(decode_bytes_literal(raw))),
            span.into(),
        ))
    }

    pub(crate) fn parse_multiline_string(&mut self) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("a multiline String literal")?;
        let span = peek.span.clone();
        let Token::MultilineStringLiteral(raw) = peek.token else {
            return Err(self.expected("a multiline String literal", peek));
        };
        self.advance(1);
        let lines = multiline_lines(raw).map_err(|msg| string_error(&span, msg))?;
        // The text starts past the opening `'''` or `"""`.
        let text_start = span.start + 3;
        let text = lines
            .into_iter()
            .map(|(at, line)| expand_escapes(line, EscapingString::Multiline, text_start + at))
            .collect::<ParseResult<Vec<_>>>()?
            .join("\n");
        Ok(Spanned::new(
            Expr::Literal(Literal::String(text)),
            span.into(),
        ))
    }
}

/// Decodes the escape that starts with the `\` at index `at` of `text`, the text of
/// a `kind` String, which starts at `text_start` in the whole source.
/// Returns the character the escape stands for and the index just past it.
pub(crate) fn decode_escape(
    text: &str,
    at: usize,
    kind: EscapingString,
    text_start: usize,
) -> ParseResult<(char, usize)> {
    let after_backslash = at + 1;
    let escape = text[after_backslash..].chars().next();
    match escape {
        Some('u') => decode_unicode_escape(text, at, text_start),
        Some(c) if let Some(stands_for) = kind.escaped(c) => {
            Ok((stands_for, after_backslash + c.len_utf8()))
        }
        // A backslash ending its line: only the backslash is labeled, since a label
        // spanning the line break would not render.
        None | Some('\n' | '\r') => {
            let help = match kind {
                EscapingString::Format(quote) => format!(
                    "a format String ends with `{}` on the same line",
                    quote.char()
                ),
                _ => kind.escapes_help(),
            };
            Err(Diagnostic::at(
                r"invalid escape sequence: `\` at the end of a line",
                in_source(text_start, at..after_backslash),
                "this escape",
            )
            .with_help(Some(help)))
        }
        Some(c) => {
            // An escape of another kind of String is one Frost knows, just not here.
            let known = ESCAPES.iter().any(|&(escape, _)| escape == c);
            let help = match kind.instead_of(c) {
                Some(instead) => format!("{}; {instead}", kind.escapes_help()),
                None => kind.escapes_help(),
            };
            // A space or control character in backticks would print as itself.
            let message = if c.is_whitespace() || c.is_control() {
                format!(
                    r"invalid escape sequence: `\` followed by {}",
                    code_point(c)
                )
            } else {
                format!(r"invalid escape sequence `\{c}`")
            };
            Err(Diagnostic::at(
                message,
                in_source(text_start, at..after_backslash + c.len_utf8()),
                if known { "this escape" } else { "unrecognized" },
            )
            .with_help(Some(help)))
        }
    }
}

/// Decodes the `\u{...}` escape at index `at` of `text`, as [`decode_escape`] does:
/// braces around the hex digits of a Unicode scalar.
///
/// Rust's `\u{...}` takes 1 to 6 hex digits; anything else in the body (including the
/// leading `+` that [`u32::from_str_radix`] would tolerate) is rejected here, as is an
/// empty body. [`char::from_u32`] rejects surrogates and values above U+10FFFF.
fn decode_unicode_escape(text: &str, at: usize, text_start: usize) -> ParseResult<(char, usize)> {
    let error = |message: String, escape: Range<usize>| {
        Diagnostic::at(message, in_source(text_start, escape), "this escape")
    };
    let malformed = |message: String, escape: Range<usize>| {
        error(message, escape).with_help(Some(UNICODE_ESCAPE_HELP.to_owned()))
    };

    // Slicing on the ASCII `\`, `u`, `{`, and `}` never splits a character.
    let open = at + 2;
    if !text[open..].starts_with('{') {
        return Err(malformed(
            r"`\u` escape must be followed by `{`".to_owned(),
            at..open,
        ));
    }
    let Some(close) = text[open..].find('}').map(|close| open + close) else {
        return Err(malformed(
            r"unterminated `\u` escape".to_owned(),
            at..text.len(),
        ));
    };
    let escape = at..close + 1;
    let hex = &text[open + 1..close];
    if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(malformed(
            format!(r"invalid `\u` escape `\u{{{hex}}}`"),
            escape,
        ));
    }
    if hex.len() > 6 {
        return Err(malformed(
            format!(r"`\u` escape `\u{{{hex}}}` has more than 6 hex digits"),
            escape,
        ));
    }
    let code = u32::from_str_radix(hex, 16).expect("body is 1-6 hex digits");
    match char::from_u32(code) {
        Some(c) => Ok((c, escape.end)),
        None => Err(error(
            format!(r"`\u{{{hex}}}` is not a valid Unicode scalar value"),
            escape,
        )),
    }
}

/// Decodes the body of an `x'..'` Bytes literal into its octets. The lexer's regex
/// guarantees an even count of hex digits, so [`u8::from_str_radix`] cannot fail.
fn decode_bytes_literal(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("lexer guarantees hex digit pairs"))
        .collect()
}

/// `text`, the text of a `kind` String, with its escapes decoded.
/// `text` starts at `text_start` in the whole source.
fn expand_escapes(text: &str, kind: EscapingString, text_start: usize) -> ParseResult<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest_start = 0;
    while let Some(backslash) = text[rest_start..].find('\\').map(|i| rest_start + i) {
        out.push_str(&text[rest_start..backslash]);
        let (c, next) = decode_escape(text, backslash, kind, text_start)?;
        out.push(c);
        rest_start = next;
    }
    out.push_str(&text[rest_start..]);
    Ok(out)
}

/// The lines of a multiline String's `raw` text, without the indentation its closing
/// delimiter sets, each with the index in `raw` where its text starts.
/// A CRLF line break is a line break like LF.
fn multiline_lines(raw: &str) -> Result<Vec<(usize, &str)>, String> {
    let Some(content) = raw.strip_prefix("\r\n").or_else(|| raw.strip_prefix('\n')) else {
        return Err(
            "multiline String must begin with a newline after the opening delimiter".into(),
        );
    };
    let content_start = raw.len() - content.len();

    // The content must end with a newline followed by the closing delimiter's
    // indentation (whitespace only). Split on the last newline.
    // Special case: if the content is whitespace-only, the string is empty.
    let Some(last_nl) = content.rfind('\n') else {
        if content.chars().all(|c| c == ' ' || c == '\t') {
            return Ok(Vec::new());
        }
        return Err("closing delimiter of multiline String must be on its own line".into());
    };

    let body = &content[..last_nl];
    let closing_indent = &content[last_nl + 1..];

    if !closing_indent.chars().all(|c| c == ' ' || c == '\t') {
        return Err("closing delimiter of multiline String must be on its own line".into());
    }

    let indent = closing_indent.len();

    body.split('\n')
        .scan(content_start, |line_start, line| {
            let at = *line_start;
            *line_start += line.len() + 1;
            Some((at, line.strip_suffix('\r').unwrap_or(line)))
        })
        .map(|(at, line)| {
            // The prefix is checked as bytes: space and tab are single-byte ASCII, so when all
            // `indent` leading bytes are one of them, `indent` is a char boundary and the slice
            // below is safe. A multibyte character inside the prefix fails the check instead of
            // panicking the slice.
            if line.is_empty() {
                Ok((at, line))
            } else if line.len() >= indent
                && line.as_bytes()[..indent]
                    .iter()
                    .all(|&b| b == b' ' || b == b'\t')
            {
                Ok((at + indent, &line[indent..]))
            } else {
                Err("multiline String content is indented less than the closing delimiter".into())
            }
        })
        .collect()
}
