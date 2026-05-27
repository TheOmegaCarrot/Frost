use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
// Re: lifetime: Logos understands this annotation and fills in the rest in its generated code,
//               so that the lifetime of the Token is tied to the lifetime of the input string.
pub enum Token<'src> {
    // -- Keywords --
    #[token("as")]
    KwAs,
    #[token("def")]
    KwDef,
    #[token("defn")]
    KwDefn,
    #[token("do")]
    KwDo,
    #[token("elif")]
    KwElif,
    #[token("else")]
    KwElse,
    #[token("export")]
    KwExport,
    #[token("false")]
    KwFalse,
    #[token("filter")]
    KwFilter,
    #[token("fn")]
    KwFn,
    #[token("foreach")]
    KwForeach,
    #[token("if")]
    KwIf,
    #[token("init")]
    KwInit,
    #[token("is")]
    KwIs,
    #[token("map")]
    KwMap,
    #[token("match")]
    KwMatch,
    #[token("null")]
    KwNull,
    #[token("reduce")]
    KwReduce,
    #[token("true")]
    KwTrue,
    #[token("with")]
    KwWith,

    // -- Punctiation (excludes operators) --
    #[token(":")]
    Colon,

    #[token(";")]
    Semicolon,

    #[token(",")]
    Comma,

    #[token("->")]
    SlimArrow,

    #[token("=>")]
    FatArrow,

    #[token("$(")]
    DollarParen,

    #[token("(")]
    OpenParen,

    #[token(")")]
    CloseParen,

    #[token("[")]
    OpenBracket,

    #[token("]")]
    CloseBracket,

    #[token("{")]
    OpenBrace,

    #[token("}")]
    CloseBrace,

    #[token("...")]
    DotDotDot,

    #[token("=")]
    Assign,

    #[token("\n")]
    Newline,

    // part of punctuation because it's used to separate match alternative patterns,
    // rather than as an operator
    #[token("|")]
    Pipe,

    // -- Operators (including keyword operators) --
    #[token("and")]
    OpAnd,

    #[token("or")]
    OpOr,

    #[token("not")]
    OpNot,

    #[token("+")]
    OpPlus,

    #[token("-")]
    OpMinus,

    #[token("*")]
    OpTimes,

    #[token("/")]
    OpDiv,

    #[token("%")]
    OpMod,

    #[token(".")]
    OpDot,

    #[token("@")]
    OpThread,

    #[token("==")]
    OpEq,

    #[token("!=")]
    OpNeq,

    #[token("<")]
    OpLt,

    #[token("<=")]
    OpLte,

    #[token(">")]
    OpGt,

    #[token(">=")]
    OpGte,

    // -- Literals --
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLiteral(i64),

    #[regex(r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    #[regex(r"\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    FloatLiteral(f64),

    // -- Identifiers --
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice())]
    Identifier(&'src str),

    // As appear in abbreviated lambdas.
    #[regex(r"\$[1-9$]?")]
    DollarIdentifier(&'src str),

    // -- Strings --

    // No escape sequences to expand. Uses #[token] + callback because
    // logos' DFA can't disambiguate the R' prefix from identifier + simple string.
    #[token("R'(", |lex| lex_raw(lex, ")'"))]
    #[token(r#"R"("#, |lex| lex_raw(lex, r#")""#))]
    RawStringLiteral(&'src str),

    // Consumer is responsible for expanding escape sequences,
    // and for complaining about invalid sequences.
    #[regex(r"'([^'\\]|\\.)*'", slice_str)] // '...'
    SingleQuoteStringLiteral(&'src str),

    #[regex(r#""([^"\\]|\\.)*""#, slice_str)] // "..."
    DoubleQuoteStringLiteral(&'src str),

    // Consumer is responsible for trimming indentation and expanding
    // the restricted set of escape sequences.
    #[token("\"\"\"", lex_multiline_double)]
    #[token("'''", lex_multiline_single)]
    MultilineStringLiteral(&'src str),

    // Consumer is responsible for splitting into segments, expanding
    // escape sequences, and re-lexing/parsing interpolation expressions.
    #[token("$'", |lex| lex_format_str(lex, b'\''))]
    SingleQuoteFormatStringLiteral(&'src str),
    #[token("$\"", |lex| lex_format_str(lex, b'"'))]
    DoubleQuoteFormatStringLiteral(&'src str),

    // -- Whitespace and comments (skipped) --
    #[regex(r"[ \t\r\f]+", logos::skip)]
    #[regex(r"#[^\n]*", logos::skip)]
    Skip,
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
    let bytes = lex.remainder().as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            // Escaped character — skip both the backslash and the next byte
            b'\\' if i + 1 < bytes.len() => i += 2,

            // Interpolation — scan forward with brace depth counting
            b'$' if i + 1 < bytes.len() && bytes[i + 1] == b'{' => {
                i += 2; // skip past ${
                let mut depth = 1u32;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        // String literal inside interpolation — skip its contents
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
                    return None; // unclosed interpolation
                }
            }

            // Closing quote — done
            c if c == quote => {
                let content = &lex.remainder()[..i];
                lex.bump(i + 1); // consume content + closing quote
                return Some(content);
            }

            // Newline — format strings are single-line
            b'\n' => return None,

            // Any other byte
            _ => i += 1,
        }
    }

    None // unclosed string
}

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
            Token::Identifier(s) => write!(f, "{s}"),
            Token::DollarIdentifier(s) => write!(f, "{s}"),
            Token::RawStringLiteral(s) => write!(f, "R'({s})'"),
            Token::SingleQuoteStringLiteral(s) => write!(f, "'{s}'"),
            Token::DoubleQuoteStringLiteral(s) => write!(f, "\"{s}\""),
            Token::MultilineStringLiteral(_) => write!(f, "multiline string"),
            Token::SingleQuoteFormatStringLiteral(s) => write!(f, "$'{s}'"),
            Token::DoubleQuoteFormatStringLiteral(s) => write!(f, "$\"{s}\""),
            Token::Skip => write!(f, ""),
        }
    }
}
