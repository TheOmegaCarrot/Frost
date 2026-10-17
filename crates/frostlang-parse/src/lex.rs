//! The lexer: source text to [`Token`]s.

use std::borrow::Cow;
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
    // A number's digits may hold `_` separators anywhere after its first digit, as
    // Rust's do. An exponent's may also start with one, but must hold a digit.
    #[regex(r"[0-9][0-9_]*", |lex| {
        without_separators(lex.slice()).parse::<u64>().unwrap_or(u64::MAX)
    })]
    // A literal in another base runs to the end of its word, so a stray digit or
    // letter, as in `0b12` or `0xfg`, makes the whole literal an error.
    #[regex(r"0[xXoObB][0-9a-zA-Z_]*", based_int)]
    IntLiteral(u64),

    /// A Float literal.
    // An exponent of separators alone, as in `1e_`, is an error, not the Int `1`
    // touching the name `e_`.
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9_]+)?", float)]
    #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9_]+", float)]
    #[regex(r"\.[0-9][0-9_]*([eE][+-]?[0-9_]+)?", float)]
    // `1.e3` is an error, not the field `e3` of the Int `1`.
    #[regex(r"[0-9][0-9_]*\.[eE][+-]?[0-9_]+", |_| None::<f64>)]
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

/// A base other than decimal that an Int literal can be written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntBase {
    Binary,
    Octal,
    Hexadecimal,
}

impl IntBase {
    /// The base whose prefix, such as `0x`, starts `literal`.
    pub(crate) fn of(literal: &str) -> Option<Self> {
        match literal.as_bytes() {
            [b'0', b'b' | b'B', ..] => Some(Self::Binary),
            [b'0', b'o' | b'O', ..] => Some(Self::Octal),
            [b'0', b'x' | b'X', ..] => Some(Self::Hexadecimal),
            _ => None,
        }
    }

    pub(crate) fn radix(self) -> u32 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Hexadecimal => 16,
        }
    }
}

/// The magnitude of a `0b`, `0o`, or `0x` Int literal, saturating as a decimal
/// one does; `None` if it has no digits, or one its base lacks.
fn based_int<'src>(lex: &logos::Lexer<'src, Token<'src>>) -> Option<u64> {
    let literal = lex.slice();
    let radix = IntBase::of(literal)?.radix();
    let digits = without_separators(&literal[2..]);
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    Some(u64::from_str_radix(&digits, radix).unwrap_or(u64::MAX))
}

/// A Float literal's value; `None` if its exponent has no digits.
fn float<'src>(lex: &logos::Lexer<'src, Token<'src>>) -> Option<f64> {
    let literal = lex.slice();
    if let Some(e) = literal.find(['e', 'E'])
        && !literal[e..].contains(|c: char| c.is_ascii_digit())
    {
        return None;
    }
    without_separators(literal).parse().ok()
}

/// A number literal's text without its `_` separators.
fn without_separators(text: &str) -> Cow<'_, str> {
    if text.contains('_') {
        Cow::Owned(text.replace('_', ""))
    } else {
        Cow::Borrowed(text)
    }
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

/// The text of a multiline String up to its closing `delimiter`: the first not broken
/// up by an escape, as `\'''` is.
///
/// A closer after text on a line past the opener's is an error here, not in parsing,
/// so it is reported before anything the lexer would misread after it.
/// Text on the opener's own line is left for parsing to reject.
fn lex_multiline<'src>(
    lex: &mut logos::Lexer<'src, Token<'src>>,
    delimiter: &str,
) -> Option<&'src str> {
    let rest = lex.remainder();
    let mut chars = rest.char_indices();
    let close = loop {
        let (at, c) = chars.next()?;
        if c == '\\' {
            chars.next();
        } else if rest[at..].starts_with(delimiter) {
            break at;
        }
    };
    lex.bump(close + delimiter.len());
    let text = &rest[..close];
    let is_blank = |line: &str| line.trim_start_matches([' ', '\t']).is_empty();
    let closer_follows_text = text.split_once('\n').is_some_and(|(opener_line, _)| {
        let opener_line = opener_line.strip_suffix('\r').unwrap_or(opener_line);
        let (_, closer_line) = text.rsplit_once('\n').unwrap_or_default();
        is_blank(opener_line) && !is_blank(closer_line)
    });
    (!closer_follows_text).then_some(text)
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
    let mut at = start;
    loop {
        match scan_format_text(text, at, quote) {
            FormatText::Closed(end) => return FormatStringEnd::Closed(end),
            FormatText::Unclosed => return FormatStringEnd::Unclosed,
            FormatText::Interpolation(open) => match skip_interpolation(text, open + 2) {
                Ok(end) => at = end,
                Err(why) => return FormatStringEnd::UnclosedInterpolation { open, why },
            },
        }
    }
}

/// Where the text of a format String, scanned up to its next interpolation, ends.
/// Each index is into the text scanned.
enum FormatText {
    /// At its closing quote, at this index.
    Closed(usize),
    /// At an interpolation, whose `${` is at this index.
    Interpolation(usize),
    /// At a line break or the end of input: format Strings are single-line.
    Unclosed,
}

/// Scans the text of the format String in `text` from `at` to its next interpolation,
/// `quote` being its quote character.
fn scan_format_text(text: &str, at: usize, quote: u8) -> FormatText {
    let bytes = text.as_bytes();
    let mut i = at;
    while i < bytes.len() {
        match bytes[i] {
            // Escaped character: skip both the backslash and the next byte
            b'\\' if i + 1 < bytes.len() => i += 2,
            b'$' if bytes.get(i + 1) == Some(&b'{') => return FormatText::Interpolation(i),
            c if c == quote => return FormatText::Closed(i),
            b'\n' => return FormatText::Unclosed,
            _ => i += 1,
        }
    }
    FormatText::Unclosed
}

/// Scans the interpolation in `text` whose expression starts at `start`, just past
/// its `${`, returning the index just past its closing `}`.
///
/// The expression is lexed as Frost, so a brace in a String, or in a format String
/// nested in it, does not count. Like its format String, an interpolation is
/// single-line.
///
/// A format String nested in it, and the interpolations nested in that, are scanned
/// on a stack rather than by recursing, so nesting costs no native stack. A nested
/// format String that does not close on its line, for whatever reason, is a String
/// in this interpolation that does not.
pub(crate) fn skip_interpolation(text: &str, start: usize) -> Result<usize, UnclosedInterpolation> {
    let mut stack = vec![Nested::Interpolation(InterpolationScan::new(start))];
    loop {
        let top = stack
            .last_mut()
            .expect("the interpolation scanned is on the stack");
        match top {
            Nested::Interpolation(scan) => match scan.scan(text) {
                InterpolationEnd::Closed(end) => {
                    stack.pop();
                    match stack.last_mut() {
                        None => return Ok(end),
                        Some(Nested::FormatString { at, .. }) => *at = end,
                        Some(Nested::Interpolation(_)) => {
                            unreachable!("an interpolation is nested only in a format String")
                        }
                    }
                }
                InterpolationEnd::FormatString { opener, quote } => {
                    stack.push(Nested::FormatString {
                        opener,
                        quote,
                        at: opener + 2,
                    });
                }
                InterpolationEnd::Unclosed(why) if stack.len() == 1 => return Err(why),
                InterpolationEnd::Unclosed(_) => return Err(unclosed_nested(&stack)),
            },
            Nested::FormatString { opener, quote, at } => match scan_format_text(text, *at, *quote)
            {
                FormatText::Closed(end) => {
                    let string = *opener..end + 1;
                    stack.pop();
                    match stack.last_mut() {
                        Some(Nested::Interpolation(scan)) => scan.resume_after_string(string),
                        _ => unreachable!("a format String is nested only in an interpolation"),
                    }
                }
                FormatText::Interpolation(open) => {
                    stack.push(Nested::Interpolation(InterpolationScan::new(open + 2)));
                }
                FormatText::Unclosed => return Err(unclosed_nested(&stack)),
            },
        }
    }
}

/// What [`skip_interpolation`] has open, outermost first: the interpolation it scans,
/// then alternately a format String nested in the one before and an interpolation in that.
enum Nested {
    Interpolation(InterpolationScan),
    FormatString {
        /// The index of its `$`.
        opener: usize,
        quote: u8,
        /// The index its text is scanned from.
        at: usize,
    },
}

/// Why the scan of the interpolation at the bottom of `stack` fails, given that
/// something nested in it does not close on its line: the format String nested in it
/// that holds the failure does not.
fn unclosed_nested(stack: &[Nested]) -> UnclosedInterpolation {
    match stack.get(1) {
        Some(Nested::FormatString { opener, .. }) => {
            UnclosedInterpolation::String(*opener..opener + 2)
        }
        _ => unreachable!("a failure above the interpolation scanned is in a format String in it"),
    }
}

/// The scan of one interpolation's expression, which pauses at each format String
/// nested in it.
struct InterpolationScan {
    /// The index the expression is lexed from.
    at: usize,
    /// The index of each `{` in it still open, innermost last.
    open_braces: Vec<usize>,
    /// The index just past its last token.
    last_end: usize,
    /// The String that is its last token, if one is.
    last_string: Option<Range<usize>>,
}

/// Where the scan of an interpolation pauses or ends.
enum InterpolationEnd {
    /// At its closing `}`: this is the index just past it.
    Closed(usize),
    /// At a format String nested in it, whose `$` is at `opener`.
    FormatString { opener: usize, quote: u8 },
    /// At its line's end, or at something in it that does not close on its line.
    Unclosed(UnclosedInterpolation),
}

impl InterpolationScan {
    fn new(start: usize) -> Self {
        Self {
            at: start,
            open_braces: Vec::new(),
            last_end: start,
            last_string: None,
        }
    }

    /// Continue past `string`, a format String nested in the interpolation, which is now
    /// its last token.
    fn resume_after_string(&mut self, string: Range<usize>) {
        self.at = string.end;
        self.last_end = string.end;
        self.last_string = Some(string);
    }

    /// Lex the interpolation in `text` from where the scan is, to where it pauses or ends.
    fn scan(&mut self, text: &str) -> InterpolationEnd {
        let start = self.at;
        let mut lexer = Token::lexer(&text[start..]);
        loop {
            // A format String is left to `skip_interpolation`, rather than lexed here,
            // which would scan it by recursing. The lexer skips spaces before a token.
            let rest = lexer.remainder();
            let token = rest.trim_start_matches([' ', '\t', '\r', '\x0C']);
            if let [b'$', quote @ (b'\'' | b'"'), ..] = token.as_bytes() {
                let opener = text.len() - token.len();
                return InterpolationEnd::FormatString {
                    opener,
                    quote: *quote,
                };
            }

            let Some(token) = lexer.next() else { break };
            let span = start + lexer.span().start..start + lexer.span().end;
            let mut is_string = false;
            match token {
                Ok(Token::OpenBrace) => self.open_braces.push(span.start),
                Ok(Token::CloseBrace) => {
                    if self.open_braces.pop().is_none() {
                        return InterpolationEnd::Closed(span.end);
                    }
                }
                Ok(Token::Newline) => {
                    return InterpolationEnd::Unclosed(self.at_line_end(text, span.start));
                }
                Ok(Token::MultilineStringLiteral(_)) if lexer.slice().contains('\n') => {
                    return InterpolationEnd::Unclosed(UnclosedInterpolation::MultilineString(
                        span.start,
                    ));
                }
                Ok(
                    Token::SingleQuoteStringLiteral(_)
                    | Token::DoubleQuoteStringLiteral(_)
                    | Token::RawStringLiteral(_)
                    | Token::MultilineStringLiteral(_)
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
                            None => {
                                return InterpolationEnd::Unclosed(UnclosedInterpolation::String(
                                    span,
                                ));
                            }
                        }
                    }
                    unlexable => {
                        if let Some(opener) = STRING_OPENERS
                            .iter()
                            .find(|opener| unlexable.starts_with(opener.as_bytes()))
                        {
                            let opener = span.start..span.start + opener.len();
                            return InterpolationEnd::Unclosed(UnclosedInterpolation::String(
                                opener,
                            ));
                        }
                    }
                },
            }
            self.last_end = start + lexer.span().end;
            self.last_string = is_string.then_some(span.start..self.last_end);
        }
        InterpolationEnd::Unclosed(self.at_line_end(text, text.len()))
    }

    /// Why the interpolation ends at the line end at `line_end` in `text`. The lexer
    /// skips only spaces and comments, so a `#` after the last token starts a comment.
    fn at_line_end(&self, text: &str, line_end: usize) -> UnclosedInterpolation {
        match text[self.last_end..line_end].find('#') {
            Some(comment) => UnclosedInterpolation::Comment(self.last_end + comment),
            None => UnclosedInterpolation::LineEnd {
                open_brace: self.open_braces.last().copied(),
                last_string: self.last_string.clone(),
            },
        }
    }
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
