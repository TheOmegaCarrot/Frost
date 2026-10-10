//! The lexer: source text to [`Token`]s.

use std::ops::Range;

use logos::Logos;

use crate::ast::SourceSpan;

/// The tokens of `source`, each with its byte span, in order.
///
/// Whitespace other than newlines, and comments, produce no tokens. An `Err`
/// covers bytes that begin no token, such as a string missing its closing
/// quote, a number written `1.e3`, or a character Frost does not use; tokens
/// resume after it.
pub fn tokens(source: &str) -> impl Iterator<Item = (Result<Token<'_>, LexError>, SourceSpan)> {
    Token::lexer(source)
        .spanned()
        .map(|(token, span)| (token.map_err(|()| LexError), span.into()))
}

/// Bytes of source that begin no [`Token`]; see [`tokens`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexError;

/// One token of Frost source. See [`tokens`].
///
/// A literal's payload is its text as written, between its delimiters: escape
/// sequences are not yet expanded, nor indentation trimmed.
#[derive(Logos, Debug, PartialEq)]
// Re: lifetime: Logos understands this annotation and fills in the rest in its generated code,
//               so that the lifetime of the Token is tied to the lifetime of the input string.
#[logos(skip r"[ \t\r\f]+")]
#[logos(skip r"#[^\n]*")]
pub enum Token<'src> {
    // -- Keywords --
    /// `as`
    #[token("as")]
    KwAs,
    /// `def`
    #[token("def")]
    KwDef,
    /// `defn`
    #[token("defn")]
    KwDefn,
    /// `do`
    #[token("do")]
    KwDo,
    /// `elif`
    #[token("elif")]
    KwElif,
    /// `else`
    #[token("else")]
    KwElse,
    /// `export`
    #[token("export")]
    KwExport,
    /// `false`
    #[token("false")]
    KwFalse,
    /// `filter`
    #[token("filter")]
    KwFilter,
    /// `fn`
    #[token("fn")]
    KwFn,
    /// `foreach`
    #[token("foreach")]
    KwForeach,
    /// `if`
    #[token("if")]
    KwIf,
    /// `init`
    #[token("init")]
    KwInit,
    /// `is`
    #[token("is")]
    KwIs,
    /// `map`
    #[token("map")]
    KwMap,
    /// `match`
    #[token("match")]
    KwMatch,
    /// `null`
    #[token("null")]
    KwNull,
    /// `reduce`
    #[token("reduce")]
    KwReduce,
    /// `true`
    #[token("true")]
    KwTrue,
    /// `with`
    #[token("with")]
    KwWith,

    // -- Punctuation (excludes operators) --
    /// `:`
    #[token(":")]
    Colon,

    /// `;`
    #[token(";")]
    Semicolon,

    /// `,`
    #[token(",")]
    Comma,

    /// `->`
    #[token("->")]
    SlimArrow,

    /// `=>`
    #[token("=>")]
    FatArrow,

    /// `$(`, opening an abbreviated lambda.
    #[token("$(")]
    DollarParen,

    /// `(`
    #[token("(")]
    OpenParen,

    /// `)`
    #[token(")")]
    CloseParen,

    /// `[`
    #[token("[")]
    OpenBracket,

    /// `]`
    #[token("]")]
    CloseBracket,

    /// `{`
    #[token("{")]
    OpenBrace,

    /// `}`
    #[token("}")]
    CloseBrace,

    /// `...`
    #[token("...")]
    DotDotDot,

    /// `=`
    #[token("=")]
    Assign,

    /// A line break.
    #[token("\n")]
    Newline,

    // part of punctuation because it's used to separate match alternative patterns,
    // rather than as an operator
    /// `|`
    #[token("|")]
    Pipe,

    // -- Operators (including keyword operators) --
    /// `and`
    #[token("and")]
    OpAnd,

    /// `or`
    #[token("or")]
    OpOr,

    /// `not`
    #[token("not")]
    OpNot,

    /// `+`
    #[token("+")]
    OpPlus,

    /// `-`
    #[token("-")]
    OpMinus,

    /// `*`
    #[token("*")]
    OpTimes,

    /// `/`
    #[token("/")]
    OpDiv,

    /// `%`
    #[token("%")]
    OpMod,

    /// `.`
    #[token(".")]
    OpDot,

    /// `@`
    #[token("@")]
    OpThread,

    /// `==`
    #[token("==")]
    OpEq,

    /// `!=`
    #[token("!=")]
    OpNeq,

    /// `<`
    #[token("<")]
    OpLt,

    /// `<=`
    #[token("<=")]
    OpLte,

    /// `>`
    #[token(">")]
    OpGt,

    /// `>=`
    #[token(">=")]
    OpGte,

    // -- Literals --
    /// An Int literal's magnitude, which may be out of the Int range: the parser
    /// checks it, since the largest negative Int's magnitude is not a positive
    /// Int. Past `u64::MAX` it saturates, being out of range all the same.
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<u64>().unwrap_or(u64::MAX))]
    IntLiteral(u64),

    /// A Float literal.
    #[regex(r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    #[regex(r"\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    // `1.e3` is an error, not the field `e3` of the Int `1`.
    #[regex(r"[0-9]+\.[eE][+-]?[0-9]+", |_| None::<f64>)]
    FloatLiteral(f64),

    // -- Identifiers --
    /// A name.
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice())]
    Identifier(&'src str),

    /// An abbreviated lambda's parameter: `$`, `$1` to `$9`, or `$$`.
    #[regex(r"\$[1-9$]?")]
    DollarIdentifier(&'src str),

    // -- Strings --

    // No escape sequences to expand. Uses #[token] + callback because
    // logos' DFA can't disambiguate the R' prefix from identifier + simple string.
    /// A raw String literal: `R'(...)'` or `R"(...)"`.
    #[token("R'(", |lex| lex_raw(lex, ")'"))]
    #[token(r#"R"("#, |lex| lex_raw(lex, r#")""#))]
    RawStringLiteral(&'src str),

    // Consumer is responsible for expanding escape sequences,
    // and for complaining about invalid sequences.
    /// A String literal in single quotes.
    // A quoted String is single-line: a line break cannot appear in it,
    // raw or after a backslash (`.` excludes `\n`).
    #[regex(r"'([^'\\\n]|\\.)*'", slice_str)] // '...'
    SingleQuoteStringLiteral(&'src str),

    /// A String literal in double quotes.
    #[regex(r#""([^"\\\n]|\\.)*""#, slice_str)] // "..."
    DoubleQuoteStringLiteral(&'src str),

    // Consumer is responsible for trimming indentation and expanding
    // the restricted set of escape sequences.
    /// A String literal in triple quotes: `'''...'''` or `"""..."""`.
    #[token("\"\"\"", lex_multiline_double)]
    #[token("'''", lex_multiline_single)]
    MultilineStringLiteral(&'src str),

    // Consumer is responsible for splitting into segments, expanding
    // escape sequences, and re-lexing/parsing interpolation expressions.
    /// A format String literal in single quotes: `$'...'`.
    #[token("$'", |lex| lex_format_str(lex, b'\''))]
    SingleQuoteFormatStringLiteral(&'src str),
    /// A format String literal in double quotes: `$"..."`.
    #[token("$\"", |lex| lex_format_str(lex, b'"'))]
    DoubleQuoteFormatStringLiteral(&'src str),

    // A Bytes literal: hex-digit pairs inside x'...' or x"...". The content is a
    // regular language, so the regex pins it exactly (even count, hex only).
    /// A Bytes literal: `x'...'` or `x"..."`, holding hex digit pairs.
    #[regex(r"x'([0-9a-fA-F]{2})*'", bytes_slice)]
    #[regex(r#"x"([0-9a-fA-F]{2})*""#, bytes_slice)]
    BytesLiteral(&'src str),
}

fn lex_raw<'src>(lex: &mut logos::Lexer<'src, Token<'src>>, closer: &str) -> Option<&'src str> {
    let rest = lex.remainder();
    let close = rest.find(closer)?;
    if rest[..close].contains('\n') {
        return None;
    }
    lex.bump(close + closer.len());
    Some(&rest[..close])
}

/// The hex body of a Bytes literal: the slice minus the `x'`/`x"` prefix and the
/// closing quote.
fn bytes_slice<'src>(lex: &logos::Lexer<'src, Token<'src>>) -> &'src str {
    let s = lex.slice();
    &s[2..s.len() - 1]
}

fn slice_str<'src>(lex: &logos::Lexer<'src, Token<'src>>) -> &'src str {
    let s = lex.slice();
    &s[1..s.len() - 1]
}

fn lex_multiline<'src>(
    lex: &mut logos::Lexer<'src, Token<'src>>,
    delimiter: &str,
) -> Option<&'src str> {
    let rest = lex.remainder();
    let close = rest.find(delimiter)?;
    lex.bump(close + delimiter.len());
    Some(&rest[..close])
}

fn lex_multiline_double<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> Option<&'src str> {
    lex_multiline(lex, "\"\"\"")
}

fn lex_multiline_single<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> Option<&'src str> {
    lex_multiline(lex, "'''")
}

fn lex_format_str<'src>(lex: &mut logos::Lexer<'src, Token<'src>>, quote: u8) -> Option<&'src str> {
    let text = lex.remainder();
    let FormatStringEnd::Closed(end) = scan_format_string(text, 0, quote) else {
        return None;
    };
    lex.bump(end + 1); // consume content + closing quote
    Some(&text[..end])
}

/// Where a format String's scan ends. Each index is into the text scanned.
pub(crate) enum FormatStringEnd {
    /// At its closing quote, at this index.
    Closed(usize),
    /// At an interpolation, whose `${` is at `open`, that does not close on its line.
    UnclosedInterpolation {
        open: usize,
        why: UnclosedInterpolation,
    },
    /// At a line break or the end of input: format Strings are single-line.
    Unclosed,
}

/// Why an interpolation does not close on its line. Each index is into the text
/// scanned.
#[derive(Debug)]
pub(crate) enum UnclosedInterpolation {
    /// A String in it, whose opener (such as `'` or `R"(`) is at this range, does not
    /// close on its line.
    String(Range<usize>),
    /// A multiline String in it, starting at this index, spans lines.
    MultilineString(usize),
    /// A comment in it, starting at this index, runs to the end of the line.
    Comment(usize),
    /// The line ends first.
    LineEnd {
        /// The innermost `{` in it still open, if any.
        open_brace: Option<usize>,
        /// The String that ends the line, if one does.
        last_string: Option<Range<usize>>,
    },
}

/// Scans the format String in `text` whose text starts at `start`, just past its
/// opening `$'` or `$"`, `quote` being its quote character.
pub(crate) fn scan_format_string(text: &str, start: usize, quote: u8) -> FormatStringEnd {
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            // Escaped character: skip both the backslash and the next byte
            b'\\' if i + 1 < bytes.len() => i += 2,

            b'$' if bytes.get(i + 1) == Some(&b'{') => match skip_interpolation(text, i + 2) {
                Ok(end) => i = end,
                Err(why) => return FormatStringEnd::UnclosedInterpolation { open: i, why },
            },

            c if c == quote => return FormatStringEnd::Closed(i),

            b'\n' => return FormatStringEnd::Unclosed,

            _ => i += 1,
        }
    }
    FormatStringEnd::Unclosed
}

/// Scans the interpolation in `text` whose expression starts at `start`, just past
/// its `${`, returning the index just past its closing `}`.
///
/// The expression is lexed as Frost, so a brace in a String, or in a format String
/// nested in it, does not count. Like its format String, an interpolation is
/// single-line.
pub(crate) fn skip_interpolation(text: &str, start: usize) -> Result<usize, UnclosedInterpolation> {
    let mut open_braces = Vec::new();
    let mut last_end = start;
    let mut last_string = None;
    // Why the interpolation ends at the line end at `line_end`. The lexer skips only
    // spaces and comments, so a `#` after the last token starts a comment.
    let at_line_end =
        |line_end: usize, last_end: usize, open_braces: &[usize], last_string| match text
            [last_end..line_end]
            .find('#')
        {
            Some(comment) => UnclosedInterpolation::Comment(last_end + comment),
            None => UnclosedInterpolation::LineEnd {
                open_brace: open_braces.last().copied(),
                last_string,
            },
        };
    let mut lexer = Token::lexer(&text[start..]);
    while let Some(token) = lexer.next() {
        let span = start + lexer.span().start..start + lexer.span().end;
        let mut is_string = false;
        match token {
            Ok(Token::OpenBrace) => open_braces.push(span.start),
            Ok(Token::CloseBrace) => {
                if open_braces.pop().is_none() {
                    return Ok(span.end);
                }
            }
            Ok(Token::Newline) => {
                return Err(at_line_end(span.start, last_end, &open_braces, last_string));
            }
            Ok(Token::MultilineStringLiteral(_)) if lexer.slice().contains('\n') => {
                return Err(UnclosedInterpolation::MultilineString(span.start));
            }
            Ok(
                Token::SingleQuoteStringLiteral(_)
                | Token::DoubleQuoteStringLiteral(_)
                | Token::RawStringLiteral(_)
                | Token::MultilineStringLiteral(_)
                | Token::SingleQuoteFormatStringLiteral(_)
                | Token::DoubleQuoteFormatStringLiteral(_)
                | Token::BytesLiteral(_),
            ) => is_string = true,
            Ok(_) => {}
            Err(()) => match lexer.slice().as_bytes() {
                // A Bytes literal that closes on its line is left to the parser,
                // which reports what is wrong inside it.
                [b'x', quote @ (b'\'' | b'"')] => {
                    let line = lexer.remainder().split('\n').next().unwrap_or_default();
                    match line.find(char::from(*quote)) {
                        Some(close) => {
                            lexer.bump(close + 1);
                            is_string = true;
                        }
                        None => return Err(UnclosedInterpolation::String(span)),
                    }
                }
                unlexable => {
                    if let Some(opener) = STRING_OPENERS
                        .iter()
                        .find(|opener| unlexable.starts_with(opener.as_bytes()))
                    {
                        let opener = span.start..span.start + opener.len();
                        return Err(UnclosedInterpolation::String(opener));
                    }
                }
            },
        }
        last_end = start + lexer.span().end;
        last_string = is_string.then_some(span.start..last_end);
    }
    Err(at_line_end(text.len(), last_end, &open_braces, last_string))
}

/// The openers of every String but a Bytes literal, longest first. Source the lexer
/// cannot read that starts with one is a String that does not close on its line.
const STRING_OPENERS: [&str; 8] = ["'''", r#"""""#, "R'(", r#"R"("#, "$'", r#"$""#, "'", r#"""#];

impl<'src> std::fmt::Display for Token<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::KwAs => write!(f, "as"),
            Token::KwDef => write!(f, "def"),
            Token::KwDefn => write!(f, "defn"),
            Token::KwDo => write!(f, "do"),
            Token::KwElif => write!(f, "elif"),
            Token::KwElse => write!(f, "else"),
            Token::KwExport => write!(f, "export"),
            Token::KwFalse => write!(f, "false"),
            Token::KwFilter => write!(f, "filter"),
            Token::KwFn => write!(f, "fn"),
            Token::KwForeach => write!(f, "foreach"),
            Token::KwIf => write!(f, "if"),
            Token::KwInit => write!(f, "init"),
            Token::KwIs => write!(f, "is"),
            Token::KwMap => write!(f, "map"),
            Token::KwMatch => write!(f, "match"),
            Token::KwNull => write!(f, "null"),
            Token::KwReduce => write!(f, "reduce"),
            Token::KwTrue => write!(f, "true"),
            Token::KwWith => write!(f, "with"),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::SlimArrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::DollarParen => write!(f, "$("),
            Token::OpenParen => write!(f, "("),
            Token::CloseParen => write!(f, ")"),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::OpenBrace => write!(f, "{{"),
            Token::CloseBrace => write!(f, "}}"),
            Token::DotDotDot => write!(f, "..."),
            Token::Assign => write!(f, "="),
            Token::Newline => write!(f, "newline"),
            Token::Pipe => write!(f, "|"),
            Token::OpAnd => write!(f, "and"),
            Token::OpOr => write!(f, "or"),
            Token::OpNot => write!(f, "not"),
            Token::OpPlus => write!(f, "+"),
            Token::OpMinus => write!(f, "-"),
            Token::OpTimes => write!(f, "*"),
            Token::OpDiv => write!(f, "/"),
            Token::OpMod => write!(f, "%"),
            Token::OpDot => write!(f, "."),
            Token::OpThread => write!(f, "@"),
            Token::OpEq => write!(f, "=="),
            Token::OpNeq => write!(f, "!="),
            Token::OpLt => write!(f, "<"),
            Token::OpLte => write!(f, "<="),
            Token::OpGt => write!(f, ">"),
            Token::OpGte => write!(f, ">="),
            Token::IntLiteral(n) => write!(f, "{n}"),
            Token::FloatLiteral(n) => write!(f, "{n}"),
            Token::Identifier(s) | Token::DollarIdentifier(s) => write!(f, "{s}"),
            Token::RawStringLiteral(s) => write!(f, "R'({s})'"),
            Token::SingleQuoteStringLiteral(s) => write!(f, "'{s}'"),
            Token::DoubleQuoteStringLiteral(s) => write!(f, "\"{s}\""),
            Token::MultilineStringLiteral(_) => write!(f, "multiline string"),
            Token::SingleQuoteFormatStringLiteral(s) => write!(f, "$'{s}'"),
            Token::DoubleQuoteFormatStringLiteral(s) => write!(f, "$\"{s}\""),
            Token::BytesLiteral(s) => write!(f, "x'{s}'"),
        }
    }
}

#[cfg(test)]
mod tests;
