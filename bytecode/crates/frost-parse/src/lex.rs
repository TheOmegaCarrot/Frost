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

    // -- Strings --

    // No escape sequences to expand.
    #[regex(r"R'\([^\n]*\)'", slice_raw_str)] // R'(...)'
    #[regex(r#"R"\([^\n]*\)""#, slice_raw_str)] // R"(...)"
    RawStringLiteral(&'src str),

    // Consumer is responsible for expanding escape sequences,
    // and for complaining about invalid sequences.
    #[regex(r"'([^'\\]|\\.)*'", slice_str)] // '...'
    #[regex(r#""([^"\\]|\\.)*""#, slice_str)] // "..."
    SimpleStringLiteral(&'src str),

    // -- Whitespace and comments (skipped) --

    #[regex(r"[ \t\r\f]+", logos::skip)]
    #[regex(r"#[^\n]*", logos::skip)]
    Skip,
}

fn slice_raw_str<'src>(lex: &logos::Lexer<'src, Token<'src>>) -> &'src str {
    let s = lex.slice();
    &s[3..s.len() - 2]
}

fn slice_str<'src>(lex: &logos::Lexer<'src, Token<'src>>) -> &'src str {
    let s = lex.slice();
    &s[1..s.len() - 1]
}
