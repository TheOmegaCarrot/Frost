//! Help for errors that look like habits carried over from other languages.
//!
//! Hints only decorate an error the parser already raises; they never change what parses.
//! Each rule matches the tokens around the error and the brackets the parser has open
//! there, so the parser's own code stays free of other languages' syntax.
//!
//! The rules live by family: [`expected`] for the one token the parser expected,
//! [`words`] for words and keywords, [`symbols`] for operators and punctuation, and
//! [`brackets`] for what brackets hold. [`site`] holds the queries they share, and
//! [`unreadable`] the errors for source the lexer cannot read. This root holds what
//! spans families: the order the rules run in, the dispatch on the found token, and
//! the help several families give.

mod brackets;
mod expected;
mod site;
mod symbols;
mod unreadable;
mod words;

pub(crate) use unreadable::unreadable;

use crate::lex::Token;
use crate::parse::ctx::{Bracket, ParseCtx};

// -- Help shared by several rules --

const LAMBDA_HELP: &str = "a lambda is written `fn x -> ...`";

const REST_HELP: &str = "a rest binding is written `...name`";

const FORMAT_STRING_HELP: &str = "a format String is written `$'...${x}...'`";

const CONDITIONAL_HELP: &str = "Frost's conditional is written `if a: b else: c`";

const ELIF_HELP: &str = "use `elif` for another condition";

const CATCH_ALL_HELP: &str = "a catch-all arm is `_ => ...`";

const ANNOTATION_HELP: &str = "Frost parameters have no type annotations";

const RETURN_TYPE_HELP: &str = "Frost has no return types; a function body follows `->`";

const DEFAULTS_HELP: &str = "Frost parameters have no default values";

const NAMED_ARGUMENTS_HELP: &str = "Frost has no named arguments; pass arguments in order";

const EQUALITY_HELP: &str = "Frost's equality is `==`";

const INEQUALITY_HELP: &str = "Frost's inequality is `!=`";

const MAP_ENTRY_HELP: &str = "a Map entry is written `key: value`";

const BLOCK_HELP: &str = "`{` starts a Map here; for a block of statements, use `do { ... }`";

const SPREAD_HELP: &str = "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`";

const ALTERNATIVES_HELP: &str = "pattern alternatives are separated by `|`, like `1 | 2 => ...`";

const SLICE_HELP: &str = "Frost has no slice syntax; use `slice(xs, start, end)`";

const CALL_HELP: &str = "call a function with parentheses: `f(x)`";

const POWER_HELP: &str = "for powers, the `std.math` module has `pow`";

const BITWISE_HELP: &str = "Frost has no bitwise operators";

const LINE_CONTINUATION_HELP: &str = "a line continues only when the next line starts with `.` \
                                      or `@`; otherwise, wrap the expression in parentheses";

/// The tokens around an error, at index `found`.
struct Site<'c, 'src, 'f> {
    ctx: &'c ParseCtx<'src, 'f>,
    found: usize,
}

impl<'src, 'f> ParseCtx<'src, 'f> {
    /// Help for the error at the token at `found`, an index into this context's tokens
    /// (its length at the end of input), when `expected` was expected, if one token was.
    ///
    /// The rules run from the most specific context to the least, and the first with
    /// help wins. Only rules that can apply at the end of input run there.
    pub(crate) fn habit_help(&self, found: usize, expected: Option<&Token>) -> Option<String> {
        let site = Site { ctx: self, found };
        let help = expected
            .and_then(|expected| site.expected_token_help(expected))
            .or_else(|| site.dots_help())
            .or_else(|| site.missing_digits_help())
            .or_else(|| site.increment_help());
        if help.is_some() || site.at(0).is_none() {
            return help;
        }
        site.header_word_help()
            .or_else(|| site.statement_word_help())
            .or_else(|| site.comment_help())
            .or_else(|| site.touching_help())
            .or_else(|| site.keyword_as_name_help())
            .or_else(|| site.brace_help())
            .or_else(|| site.operand_word_help())
            .or_else(|| site.separator_help())
            .or_else(|| site.line_break_help())
            .or_else(|| site.found_token_help())
            .or_else(|| site.call_help())
    }
}

impl Site<'_, '_, '_> {
    /// Help for the found token alone, read in the brackets around it.
    fn found_token_help(&self) -> Option<String> {
        let token = &self.at(0)?.token;
        if let Some(help) = self.parameter_pattern_help() {
            return Some(help);
        }
        let open = self.ctx.innermost_bracket();
        let kind = open.map(|open| open.kind);
        let help = match token {
            Token::Assign => return self.assignment_help(open),
            Token::FatArrow | Token::SlimArrow => return self.arrow_help(open),
            Token::Pipe => return self.pipe_help(),
            Token::Colon => return self.colon_help(),
            Token::Comma => return self.comma_help(),
            Token::OpenBrace => return self.brace_after_header_help(),
            Token::CloseParen => return self.empty_group_help(),
            Token::SingleQuoteStringLiteral(_)
            | Token::DoubleQuoteStringLiteral(_)
            | Token::MultilineStringLiteral(_) => return self.string_help(),
            Token::DollarIdentifier(name) => return self.dollar_help(name),
            Token::DotDotDot => return self.spread_help(),
            token if self.token_at(-1) == Some(&Token::KwIs) => {
                return self.type_test_help(token);
            }
            Token::KwIf => CONDITIONAL_HELP,
            Token::KwElse | Token::KwElif => return self.stray_branch_help(),
            Token::KwAs if kind == Some(Bracket::MapPattern) => {
                "rename a Map entry with `key: name`, like `{a: b}`"
            }
            // `def a = 1 def b = 2`
            Token::KwDef | Token::KwDefn | Token::KwExport
                if self.at_statement_level(open) && self.follows_an_operand(0) =>
            {
                "separate statements with a line break or `;`"
            }
            Token::KwDef | Token::KwDefn => match self.token_before(self.found) {
                Some(Token::SlimArrow) => {
                    "a function body with statements goes in braces: `-> { ... }`"
                }
                _ => {
                    "a definition is a statement; for statements inside an expression, \
                     use a block: `do { ... }`"
                }
            },
            Token::KwExport => "`export` is only allowed at the top level",
            _ => return self.operator_help(),
        };
        Some(help.to_owned())
    }
}

/// Help for an operator Frost lacks that would update a value, `op`, as in `+=` or `++`.
fn rebind_help(op: &str) -> String {
    format!("Frost has no `{op}`; bind the result to a new name with `def`")
}

/// Help for a function defined as another language defines one, named `name`.
fn define_function_help(name: &str) -> String {
    format!("define a function with `defn {name}(...) -> ...`")
}

/// Help for `name :=`, another language's declaration.
fn walrus_help(name: &str) -> String {
    format!("Frost has no `:=`; bind a name with `def`: `def {name} = ...`")
}

/// Help for several names bound at once, as `pattern` binds them.
fn destructure_help(pattern: &str) -> String {
    format!("to bind several names, destructure with `def`: `def {pattern} = ...`")
}

/// Help for a Float written with no digits after its point, `example` being it with them.
fn float_point_help(example: &str) -> String {
    format!("a Float needs digits after its point, like `{example}`")
}

/// Help for a raw String written as another language writes one, `quote` being its quote.
fn raw_string_help(quote: char) -> String {
    format!("a raw String is written `R{quote}(...){quote}`")
}

fn is_iterative_keyword(token: &Token) -> bool {
    matches!(
        token,
        Token::KwMap | Token::KwFilter | Token::KwReduce | Token::KwForeach
    )
}

/// Help for `map`, `filter`, `reduce`, or `foreach` written as a function call;
/// `seeded` when that call passes `reduce` an initial value.
fn iterative_help(keyword: &Token, seeded: bool) -> String {
    if seeded && *keyword == Token::KwReduce {
        return "`reduce` is an expression, written `reduce xs init: v with f`; \
                its function form is `fold(xs, f, v)`"
            .to_owned();
    }
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
