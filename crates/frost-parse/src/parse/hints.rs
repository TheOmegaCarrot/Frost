//! Help for errors that look like habits carried over from other languages.
//!
//! Hints only decorate an error the parser already raises; they never change what parses.
//! Each rule matches the tokens around the error, so the parser's own code stays free of
//! other languages' syntax.

use crate::lex::Token;
use crate::parse::ctx::{ParseCtx, SrcToken};
use crate::parse::match_expr::TYPE_CONSTRAINTS;

/// Words other languages use to declare a variable.
const DECLARATION_WORDS: &[&str] = &["let", "var", "const", "local", "val", "mut"];

/// Words other languages use to define a function.
const FUNCTION_WORDS: &[&str] = &["function", "func", "fun"];

/// Words other languages use to start a loop.
const LOOP_WORDS: &[&str] = &["for", "while"];

const LAMBDA_HELP: &str = "a lambda is written `fn x -> ...`";

const GUARD_HELP: &str = "a guard is written `if:` before its condition, like `n if: n > 0 => ...`";

const ARM_HELP: &str = "a `match` arm is written `pattern => result`";

const AS_HELP: &str = "`as` binds a whole Map pattern, like `{name} as person`";

const ANNOTATION_HELP: &str = "Frost parameters have no type annotations";

impl<'src, 'f> ParseCtx<'src, 'f> {
    /// Help for the error at the token at `found`, an index into this context's tokens
    /// (its length at the end of input), when `expected` was expected, if one token was.
    pub(crate) fn habit_help(&self, found: usize, expected: Option<&Token>) -> Option<String> {
        let site = Site { ctx: self, found };
        if let Some(help) = expected.and_then(|expected| site.expected_token_help(expected)) {
            return Some(help);
        }
        // `1.`, which may end the input; `1..5` has help of its own.
        if site.at(-1).map(|t| &t.token) == Some(&Token::OpDot)
            && matches!(site.at(-2).map(|t| &t.token), Some(Token::IntLiteral(_)))
            && site.at(-2)?.span.end == site.at(-1)?.span.start
            && !matches!(
                site.at(0).map(|t| &t.token),
                Some(Token::Identifier(_) | Token::FloatLiteral(_))
            )
        {
            return Some("a Float needs digits after its point, like `1.0`".to_owned());
        }
        let token = &site.at(0)?.token;

        if let Some(word) = site.statement_word() {
            if DECLARATION_WORDS.contains(&word)
                && let Token::Identifier(name) = token
            {
                return Some(format!(
                    "Frost has no `{word}`; bind a name with `def`: `def {name} = ...`"
                ));
            }
            if FUNCTION_WORDS.contains(&word)
                && let Token::Identifier(name) = token
            {
                return Some(format!(
                    "Frost has no `{word}`; define a function with `defn {name}(...) -> ...`"
                ));
            }
            if LOOP_WORDS.contains(&word) {
                return Some(format!(
                    "Frost has no `{word}` loop; use `map`, `filter`, `reduce`, or `foreach` \
                     with a function, like `foreach xs with fn x -> ...`"
                ));
            }
            match word {
                "return" => {
                    return Some(
                        "Frost has no `return`; a function's value is its last expression"
                            .to_owned(),
                    );
                }
                "lambda" => return Some(LAMBDA_HELP.to_owned()),
                _ => {}
            }
        }

        if let Some(help) = site
            .touching_help()
            .or_else(|| site.keyword_as_name_help())
            .or_else(|| site.map_brace_help())
        {
            return Some(help);
        }

        match token {
            Token::Assign => {
                if let Some(op) = site.abutting_operator() {
                    return Some(format!(
                        "Frost has no `{op}=`; bind the result to a new name with `def`"
                    ));
                }
                // An assignment's target ends just before its `=`.
                if !ends_an_expression(&site.at(-1)?.token) {
                    return None;
                }
                match site.enclosing() {
                    Enclosing::Statements => match site.statement_word() {
                        Some(name) => Some(format!(
                            "Frost has no assignment; bind a new name with `def`: `def {name} = ...`"
                        )),
                        None => Some(
                            "Frost values are immutable; bind an updated value to a new name \
                             with `def`"
                                .to_owned(),
                        ),
                    },
                    Enclosing::Map => Some("a Map entry is written `key: value`".to_owned()),
                    Enclosing::Call => {
                        Some("Frost has no named arguments; pass arguments in order".to_owned())
                    }
                    Enclosing::Parameters => {
                        Some("Frost parameters have no default values".to_owned())
                    }
                    Enclosing::Other => None,
                }
            }
            Token::FatArrow | Token::SlimArrow => Some(LAMBDA_HELP.to_owned()),
            Token::Pipe => match site.abutting_next() {
                Some(Token::Pipe) => Some("Frost's \"or\" is `or`".to_owned()),
                Some(Token::OpGt) => {
                    Some("Frost threads a value into a call with `@`: `x @ f()`".to_owned())
                }
                // `|x| ...` or `|x, y| ...`
                Some(Token::Identifier(_))
                    if matches!(
                        site.at(2).map(|t| &t.token),
                        Some(Token::Pipe | Token::Comma)
                    ) =>
                {
                    Some(LAMBDA_HELP.to_owned())
                }
                _ => None,
            },
            Token::KwIf => Some("Frost's conditional is written `if a: b else: c`".to_owned()),
            Token::SingleQuoteStringLiteral(_) | Token::DoubleQuoteStringLiteral(_)
                if site.abutting_previous() == Some(&Token::Identifier("f")) =>
            {
                Some("a format String is written `$'...${x}...'`".to_owned())
            }
            Token::Colon if matches!(site.enclosing(), Enclosing::Parameters) => {
                Some(ANNOTATION_HELP.to_owned())
            }
            Token::KwAs if matches!(site.enclosing(), Enclosing::Map) => {
                Some("rename a Map entry with `key: name`, like `{a: b}`".to_owned())
            }
            Token::KwDef | Token::KwDefn => match site.before_line_breaks() {
                Some(Token::SlimArrow) => {
                    Some("a function body with statements goes in braces: `-> { ... }`".to_owned())
                }
                _ => Some(
                    "a definition is a statement; for statements inside an expression, \
                     use a block: `do { ... }`"
                        .to_owned(),
                ),
            },
            Token::KwExport => Some("`export` is only allowed at the top level".to_owned()),
            Token::DollarIdentifier(_) => {
                Some("`$` placeholders work only inside `$( ... )`, like `$($ * 2)`".to_owned())
            }
            Token::Comma if site.at(-2).map(|t| &t.token) == Some(&Token::DotDotDot) => {
                Some("a `...rest` binding comes last, with nothing after it".to_owned())
            }
            Token::OpDot if site.abutting_next() == Some(&Token::OpDot) => {
                Some("a rest binding is written `...name`".to_owned())
            }
            Token::DotDotDot => Some(match site.enclosing() {
                Enclosing::Call => "Frost has no spread; to pass an Array's elements as \
                                    arguments, use `call(f, args)`"
                    .to_owned(),
                _ => "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`"
                    .to_owned(),
            }),
            Token::Identifier(name) if site.at(-1).map(|t| &t.token) == Some(&Token::KwIs) => {
                Some(type_name_help(name))
            }
            _ => None,
        }
    }
}

/// Help for `name` written where a type name belongs, after `is`.
fn type_name_help(name: &str) -> String {
    match TYPE_CONSTRAINTS
        .iter()
        .find(|(type_name, _)| type_name.eq_ignore_ascii_case(name))
    {
        Some((type_name, _)) => format!("did you mean `{type_name}`?"),
        None => {
            let names: Vec<String> = TYPE_CONSTRAINTS
                .iter()
                .map(|(type_name, _)| format!("`{type_name}`"))
                .collect();
            format!("the types are {}", names.join(", "))
        }
    }
}

impl Site<'_, '_, '_> {
    /// Help for tokens touching with no space between, the shape of a token Frost lacks:
    /// `**`, `===`, `//`, `0xFF`, `t.0`, and the like.
    fn touching_help(&self) -> Option<String> {
        let found = &self.at(0)?.token;
        let help = match (self.abutting_previous()?, found) {
            (Token::OpEq, Token::Assign) => "Frost's equality is `==`",
            (Token::OpNeq, Token::Assign) => "Frost's inequality is `!=`",
            (Token::OpTimes, Token::OpTimes) => {
                "Frost has no `**`; for powers, the `std.math` module has `pow`"
            }
            (Token::OpDiv, Token::OpDiv | Token::OpTimes) => "a comment starts with `#`",
            (Token::OpLt, Token::OpLt) | (Token::OpGt, Token::OpGt) => {
                "Frost has no shift operators"
            }
            (Token::DollarIdentifier(_), Token::IntLiteral(_)) => {
                "the placeholders are `$`, `$1` to `$9`, and `$$`"
            }
            (Token::IntLiteral(_), Token::Identifier(word)) => {
                if word.starts_with('_') {
                    "Frost number literals have no `_` separators"
                } else if self.ctx.source_text(self.at(-1)?.span.clone().into()) == "0"
                    && word.starts_with(['x', 'b', 'o'])
                {
                    "Frost number literals are decimal only"
                } else {
                    return None;
                }
            }
            (Token::OpDot, Token::FloatLiteral(_)) => {
                "Frost has no range operator; for a range of Ints, use `range(start, stop)`"
            }
            (_, Token::FloatLiteral(_))
                if self
                    .ctx
                    .source_text(self.at(0)?.span.clone().into())
                    .starts_with('.') =>
            {
                "index an Array with brackets, like `xs[0]`"
            }
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Help for a keyword written where a name belongs.
    fn keyword_as_name_help(&self) -> Option<String> {
        let found = &self.at(0)?.token;
        if !is_word_keyword(found) {
            return None;
        }
        let previous = self.at(-1).map(|t| &t.token);
        let next = self.at(1).map(|t| &t.token);
        if previous == Some(&Token::OpDot) {
            return Some(if is_iterative_keyword(found) {
                format!(
                    "`{found}` is a keyword, not a method; write `{found} xs with f`, \
                     or `xs @ {}(f)`",
                    function_form(found)
                )
            } else {
                format!("`{found}` is a keyword; index with brackets instead: `x[\"{found}\"]`")
            });
        }
        if next == Some(&Token::Colon) && matches!(self.enclosing(), Enclosing::Map) {
            return Some(format!(
                "`{found}` is a keyword; write the key in brackets: `[\"{found}\"]: ...`"
            ));
        }
        let names_here = matches!(
            previous,
            Some(Token::KwDef | Token::KwDefn | Token::KwFn | Token::DotDotDot | Token::KwAs)
        ) || (matches!(previous, Some(Token::OpenParen | Token::Comma))
            && matches!(self.enclosing(), Enclosing::Parameters));
        names_here.then(|| format!("`{found}` is a keyword, so it cannot be a name"))
    }

    /// Help for a `{` read as a Map where a block or a quoted key was meant.
    fn map_brace_help(&self) -> Option<String> {
        let found = self.at(0)?;
        let literal_key = |token: &SrcToken| match token.token {
            Token::SingleQuoteStringLiteral(_)
            | Token::DoubleQuoteStringLiteral(_)
            | Token::IntLiteral(_)
            | Token::FloatLiteral(_) => Some(self.ctx.source_text(token.span.clone().into())),
            _ => None,
        };
        let key_help = |key: &str| {
            format!("a Map key that is not a name goes in brackets: `{{[{key}]: ...}}`")
        };

        // `{"a": 1}`
        if let Some(key) = literal_key(found)
            && self.at(1).map(|t| &t.token) == Some(&Token::Colon)
            && matches!(self.enclosing(), Enclosing::Map)
        {
            return Some(key_help(key));
        }
        // `fn -> { "a": 1 }`, where the `{` was read as a block
        if found.token == Token::Colon
            && let Some(key) = self.at(-1).and_then(literal_key)
            && self.at(-2).map(|t| &t.token) == Some(&Token::OpenBrace)
        {
            return Some(key_help(key));
        }

        // `if c: { def y = 1; y }` or `{ x + 1 }`, meant as blocks
        if !matches!(self.enclosing(), Enclosing::Map) {
            return None;
        }
        let first_in_braces = match self.at(-1).map(|t| &t.token) {
            // `{...m}` is a spread, which has help of its own.
            Some(Token::OpenBrace) => found.token != Token::DotDotDot,
            Some(Token::Identifier(_)) => {
                self.at(-2).map(|t| &t.token) == Some(&Token::OpenBrace)
                    && infix_or_postfix(&found.token)
            }
            _ => false,
        };
        first_in_braces.then(|| {
            "`{` starts a Map here; for a block of statements, use `do { ... }`".to_owned()
        })
    }

    /// The nearest token before the error that is not a line break.
    fn before_line_breaks(&self) -> Option<&Token<'_>> {
        (0..self.found)
            .rev()
            .filter_map(|index| self.ctx.get(index))
            .map(|t| &t.token)
            .find(|token| **token != Token::Newline)
    }
}

/// Whether `token` is a keyword spelled as a word, which looks like a name.
fn is_word_keyword(token: &Token) -> bool {
    matches!(
        token,
        Token::KwAs
            | Token::KwDef
            | Token::KwDefn
            | Token::KwDo
            | Token::KwElif
            | Token::KwElse
            | Token::KwExport
            | Token::KwFalse
            | Token::KwFilter
            | Token::KwFn
            | Token::KwForeach
            | Token::KwIf
            | Token::KwInit
            | Token::KwIs
            | Token::KwMap
            | Token::KwMatch
            | Token::KwNull
            | Token::KwReduce
            | Token::KwTrue
            | Token::KwWith
            | Token::OpAnd
            | Token::OpOr
            | Token::OpNot
    )
}

/// Whether `token` can end an expression: a name, a literal, or a closing bracket.
fn ends_an_expression(token: &Token) -> bool {
    matches!(
        token,
        Token::Identifier(_)
            | Token::IntLiteral(_)
            | Token::FloatLiteral(_)
            | Token::SingleQuoteStringLiteral(_)
            | Token::DoubleQuoteStringLiteral(_)
            | Token::KwTrue
            | Token::KwFalse
            | Token::KwNull
            | Token::CloseParen
            | Token::CloseBracket
            | Token::CloseBrace
    )
}

/// Whether `token` continues an expression after a name: an operator or a postfix.
fn infix_or_postfix(token: &Token) -> bool {
    matches!(
        token,
        Token::OpPlus
            | Token::OpMinus
            | Token::OpTimes
            | Token::OpDiv
            | Token::OpMod
            | Token::OpEq
            | Token::OpNeq
            | Token::OpLt
            | Token::OpLte
            | Token::OpGt
            | Token::OpGte
            | Token::OpAnd
            | Token::OpOr
            | Token::OpDot
            | Token::OpThread
            | Token::OpenParen
            | Token::OpenBracket
    )
}

impl Site<'_, '_, '_> {
    /// Help for the error, given the one token that was expected there.
    fn expected_token_help(&self, expected: &Token) -> Option<String> {
        let found = self.at(0).map(|t| &t.token);
        let previous = self.at(-1).map(|t| &t.token);
        let help = match expected {
            Token::Colon => match previous {
                Some(Token::KwIf) => GUARD_HELP,
                Some(Token::KwElse) if found == Some(&Token::KwIf) => {
                    "use `elif` for another condition"
                }
                Some(Token::KwElse) => "`else` is followed by a colon: `else: ...`",
                _ => match self.nearest_on_line(|t| matches!(t, Token::KwIf | Token::KwElif))? {
                    Token::KwElif => "`elif` takes a colon after its condition: `elif x: ...`",
                    _ => "`if` takes a colon after its condition: `if x: ...`",
                },
            },
            Token::FatArrow => match found {
                Some(Token::SlimArrow | Token::Colon) => ARM_HELP,
                Some(Token::Identifier("when")) => GUARD_HELP,
                Some(Token::KwAs) => AS_HELP,
                _ if previous == Some(&Token::Identifier("case")) => {
                    "Frost's `match` has no `case`; an arm is written `pattern => result`"
                }
                _ => return None,
            },
            Token::SlimArrow => match found? {
                Token::OpenBrace => "add `->` before the body: `-> { ... }`",
                Token::FatArrow => "a lambda's arrow is `->`: `fn x -> ...`",
                Token::Colon => ANNOTATION_HELP,
                _ => return None,
            },
            Token::Assign if found == Some(&Token::KwAs) => AS_HELP,
            Token::KwDef if previous == Some(&Token::KwExport) => {
                "`export` takes a definition: `export def x = ...` or `export defn f(...) -> ...`"
            }
            Token::KwWith => {
                let keyword = self.nearest_on_line(is_iterative_keyword)?;
                return Some(iterative_help(keyword));
            }
            // `map(xs, f)`: the parenthesized collection is cut off at the comma.
            Token::CloseParen if found == Some(&Token::Comma) => {
                let opener = self.innermost_open()?;
                let keyword = &self.ctx.get(opener.checked_sub(1)?)?.token;
                if !is_iterative_keyword(keyword) {
                    return None;
                }
                return Some(iterative_help(keyword));
            }
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// The nearest token before the error and on its line that `wanted` accepts.
    /// A `;` ends the search as a line break does.
    fn nearest_on_line(&self, wanted: impl Fn(&Token) -> bool) -> Option<&Token<'_>> {
        // When the error starts a line, or is the end of input, the line meant is the
        // last one before it with a token on it.
        let mut end = self.found;
        while end > 0
            && matches!(
                self.ctx.get(end - 1).map(|t| &t.token),
                Some(Token::Newline)
            )
        {
            end -= 1;
        }
        (0..end)
            .rev()
            .map_while(|index| {
                self.ctx
                    .get(index)
                    .filter(|t| !matches!(t.token, Token::Newline | Token::Semicolon))
            })
            .map(|t| &t.token)
            .find(|token| wanted(token))
    }
}

fn is_iterative_keyword(token: &Token) -> bool {
    matches!(
        token,
        Token::KwMap | Token::KwFilter | Token::KwReduce | Token::KwForeach
    )
}

/// Help for `map`, `filter`, `reduce`, or `foreach` written as a function call.
fn iterative_help(keyword: &Token) -> String {
    format!(
        "`{keyword}` is an expression, written `{keyword} xs with f`; \
         its function form is `{}(xs, f)`",
        function_form(keyword)
    )
}

/// The function that does what the iterative expression `keyword` does.
fn function_form(keyword: &Token) -> &'static str {
    match keyword {
        Token::KwMap => "transform",
        Token::KwFilter => "select",
        Token::KwReduce => "fold",
        _ => "each",
    }
}

/// What holds an error, by the innermost bracket still open there.
enum Enclosing {
    /// No bracket, or a block's `{`: a place for statements.
    Statements,
    /// A Map literal's `{`.
    Map,
    /// A call's `(`.
    Call,
    /// A lambda's or `defn`'s parameter list.
    Parameters,
    /// Any other bracket.
    Other,
}

/// The tokens around an error, at index `found`.
struct Site<'c, 'src, 'f> {
    ctx: &'c ParseCtx<'src, 'f>,
    found: usize,
}

impl<'src> Site<'_, 'src, '_> {
    /// The token `offset` places from the error's.
    fn at(&self, offset: isize) -> Option<&SrcToken<'src>> {
        self.ctx.get(self.found.checked_add_signed(offset)?)
    }

    /// Whether the token at `offset` begins a statement: it starts the input, a line,
    /// or follows a `;` or the `{` of a block.
    fn starts_statement(&self, offset: isize) -> bool {
        match self.found.checked_add_signed(offset - 1) {
            None => true,
            Some(before) => matches!(
                self.ctx.get(before).map(|t| &t.token),
                None | Some(Token::Newline | Token::Semicolon | Token::OpenBrace)
            ),
        }
    }

    /// The word before the error, when it alone was read as a whole statement.
    fn statement_word(&self) -> Option<&'src str> {
        match self.at(-1)?.token {
            Token::Identifier(word) if self.starts_statement(-1) => Some(word),
            _ => None,
        }
    }

    /// The index of the innermost bracket still open at the error.
    fn innermost_open(&self) -> Option<usize> {
        let mut open: Vec<usize> = Vec::new();
        for index in 0..self.found {
            match self.ctx.get(index).map(|t| &t.token) {
                Some(
                    Token::OpenParen | Token::DollarParen | Token::OpenBracket | Token::OpenBrace,
                ) => {
                    open.push(index);
                }
                Some(Token::CloseParen | Token::CloseBracket | Token::CloseBrace) => {
                    open.pop();
                }
                _ => {}
            }
        }
        open.pop()
    }

    /// What the innermost bracket still open at the error holds.
    fn enclosing(&self) -> Enclosing {
        let Some(opener) = self.innermost_open() else {
            return Enclosing::Statements;
        };
        let token = |index: Option<usize>| index.and_then(|i| self.ctx.get(i)).map(|t| &t.token);
        let before = |steps: usize| token(opener.checked_sub(steps));
        match (token(Some(opener)), before(1)) {
            (Some(Token::OpenBrace), Some(Token::KwDo | Token::SlimArrow)) => Enclosing::Statements,
            (Some(Token::OpenBrace), _) => Enclosing::Map,
            (Some(Token::OpenParen), Some(Token::KwFn)) => Enclosing::Parameters,
            (Some(Token::OpenParen), Some(Token::Identifier(_)))
                if matches!(before(2), Some(Token::KwFn | Token::KwDefn)) =>
            {
                Enclosing::Parameters
            }
            (
                Some(Token::OpenParen),
                Some(Token::Identifier(_) | Token::CloseParen | Token::CloseBracket),
            ) => Enclosing::Call,
            _ => Enclosing::Other,
        }
    }

    /// The previous token, when it touches the error's with no space between.
    fn abutting_previous(&self) -> Option<&Token<'src>> {
        let (previous, found) = (self.at(-1)?, self.at(0)?);
        (previous.span.end == found.span.start).then_some(&previous.token)
    }

    /// The next token, when it touches the error's with no space between.
    fn abutting_next(&self) -> Option<&Token<'src>> {
        let (found, next) = (self.at(0)?, self.at(1)?);
        (found.span.end == next.span.start).then_some(&next.token)
    }

    /// The arithmetic operator touching the error's token from before, as in `+=`.
    fn abutting_operator(&self) -> Option<&'static str> {
        match self.abutting_previous()? {
            Token::OpPlus => Some("+"),
            Token::OpMinus => Some("-"),
            Token::OpTimes => Some("*"),
            Token::OpDiv => Some("/"),
            Token::OpMod => Some("%"),
            _ => None,
        }
    }
}

/// The message and any help for source the lexer could not read, `text` being the
/// source from the unreadable point to the end of its line.
pub(crate) fn unreadable(text: &str) -> (String, Option<String>) {
    let mut chars = text.chars();
    let first = chars.next().unwrap_or(' ');
    let second = chars.next();

    let unclosed = |what: &str, closer: &str| {
        (
            format!("unclosed {what}"),
            Some(format!("a {what} ends with `{closer}` on the same line")),
        )
    };
    match (first, second) {
        ('\'', _) => return unclosed("String", "'"),
        ('"', _) => return unclosed("String", "\""),
        ('$', Some('\'')) => return unclosed("format String", "'"),
        ('$', Some('"')) => return unclosed("format String", "\""),
        ('R', Some('\'')) => return unclosed("raw String", ")'"),
        ('R', Some('"')) => return unclosed("raw String", ")\""),
        ('x', Some('\'' | '"')) => {
            return (
                "invalid Bytes literal".to_owned(),
                Some("a Bytes literal holds pairs of hex digits, like `x'00ff'`".to_owned()),
            );
        }
        _ => {}
    }

    let help = match (first, second) {
        ('&', Some('&')) => Some("Frost's \"and\" is `and`"),
        ('!', _) => Some("Frost's \"not\" is `not`"),
        ('?', Some('?')) => Some("for a fallback when a value is null, use `or`: `a or b`"),
        ('?', _) => Some("Frost's conditional is written `if a: b else: c`"),
        ('`', _) => Some("a format String is written `$'...${x}...'`"),
        ('\\', None) => Some(
            "a line continues only when the next line starts with `.` or `@`; \
             otherwise, wrap the expression in parentheses",
        ),
        ('\\', _) => Some(LAMBDA_HELP),
        ('\u{201c}' | '\u{201d}' | '\u{2018}' | '\u{2019}', _) => {
            Some("Frost Strings use straight quotes, `'` or `\"`")
        }
        _ => None,
    };
    (
        format!("unexpected character {}", describe_char(first)),
        help.map(str::to_owned),
    )
}

/// A character as an error names it: in backticks, with its code point when it is
/// not plain visible ASCII.
fn describe_char(c: char) -> String {
    if c == '`' {
        "`` ` ``".to_owned()
    } else if c.is_ascii_graphic() {
        format!("`{c}`")
    } else if c.is_whitespace() || c.is_control() || c == '\u{feff}' {
        format!("U+{:04X}", u32::from(c))
    } else {
        format!("`{c}` (U+{:04X})", u32::from(c))
    }
}
