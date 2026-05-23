use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
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

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    // -- Whitespace and comments (skipped) --

    #[regex(r"[ \t\r\f]+", logos::skip)]
    #[regex(r"#[^\n]*", logos::skip)]
    Skip,
}
