//! Rules for words: keywords, and words other languages use that Frost does not.

use crate::lex::Token;
use crate::parse::ctx::Bracket;
use crate::parse::hints::site::{
    ArmPart, ends_an_expression, is_binary_operator, is_literal, is_word_keyword,
    starts_an_expression,
};
use crate::parse::hints::{
    CALL_HELP, CATCH_ALL_HELP, CONDITIONAL_HELP, ELIF_HELP, LAMBDA_HELP, RETURN_TYPE_HELP, Site,
    bare_name_help, define_function_help, function_form, is_iterative_keyword,
};
use crate::parse::match_expr::TYPE_CONSTRAINTS;

/// Words other languages use to declare a variable.
const DECLARATION_WORDS: &[&str] = &["let", "var", "const", "local", "val", "mut"];

/// Words other languages use to define a function.
const FUNCTION_WORDS: &[&str] = &["function", "func", "fun"];

/// Words other languages put between a declaration word and its name, as in `let mut x`.
/// A function word may stand there too, as in `local function f`.
const MODIFIER_WORDS: &[&str] = &["mut", "rec", "async"];

/// Words other languages use to start a loop.
const LOOP_WORDS: &[&str] = &["for", "while", "until"];

/// Words other languages use to print, which may take an argument without parentheses,
/// as in `print "hi"`.
const PRINT_WORDS: &[&str] = &["print", "println", "puts", "echo", "say"];

/// Words other languages use to declare a type, which look like a function's name
/// before its parameters, as in `type T = Int`.
const TYPE_DECLARATION_WORDS: &[&str] = &["type", "data", "newtype", "alias"];

/// Lowercase type names from other languages, which may follow a lambda's arrow as a
/// return type.
const LOWERCASE_TYPE_NAMES: &[&str] = &[
    "int", "str", "void", "double", "i32", "i64", "u8", "u32", "u64", "usize", "f32", "f64",
];

const UNLESS_HELP: &str = "Frost has no `unless`; write `if not cond: ...`";

const IN_HELP: &str = "Frost has no `in` operator; use `includes(xs, x)` for an Array, \
                       `has(m, k)` for a Map, or `contains(s, part)` for a String";

impl<'src> Site<'_, 'src, '_> {
    /// The word from another language that starts the error's statement, when it is in
    /// `words` and the error is in or just after its header: `for x in xs`,
    /// `for (i = 0; ...)`, or `while (x) { ... }`.
    fn header_word(&self, words: &[&str]) -> Option<&'src str> {
        let word_at = |index: usize| match self.ctx.get(index)?.token {
            Token::Identifier(word) if words.contains(&word) && self.starts_statement(index) => {
                Some(word)
            }
            _ => None,
        };
        // `for x in xs`, where the word alone was read as a statement
        if let Some(word) = self.statement_word().filter(|word| words.contains(word)) {
            return Some(word);
        }
        // `for (i = 0; ...)`, where the header's `(` was read as a call's
        if let Some(open) = self.ctx.innermost_bracket()
            && open.kind == Bracket::Call
            && let Some(word) = open.pos.checked_sub(1).and_then(word_at)
        {
            return Some(word);
        }
        // `while (x) { ... }`, where the header was read as a whole call
        let close = self.found.checked_sub(1)?;
        if self.ctx.get(close)?.token != Token::CloseParen {
            return None;
        }
        word_at(self.matching_open(close)?.checked_sub(1)?)
    }

    /// Help for a statement started by another language's loop, `switch`, or `unless`,
    /// with or without a parenthesized header.
    pub(super) fn header_word_help(&self) -> Option<String> {
        if let Some(word) = self.header_word(LOOP_WORDS) {
            return Some(format!(
                "Frost has no `{word}` loop; use `map`, `filter`, `reduce`, or `foreach` \
                 with a function, like `foreach xs with fn x -> ...`"
            ));
        }
        let help = match self.header_word(&["switch", "unless"])? {
            "switch" => "Frost has no `switch`; use `match x { ... }`",
            _ => UNLESS_HELP,
        };
        Some(help.to_owned())
    }

    /// Help for a word from another language read as a whole statement, as in
    /// `let x = 1` or `import math`, or as a whole expression, as in `fn x -> return x`.
    pub(super) fn statement_word_help(&self) -> Option<String> {
        // `match x { let y => 1 }`, where a pattern starts and no statement can
        if let Some(word) = self.expression_word()
            && self.innermost_kind() == Some(Bracket::MatchArms)
            && self.pattern_starts_at(self.found - 1)
        {
            return match self.token_at(0)? {
                Token::Identifier(name) if DECLARATION_WORDS.contains(&word) => {
                    Some(bare_name_help(name))
                }
                _ => None,
            };
        }
        if let Some(help) = self.expression_word_help() {
            return Some(help);
        }
        let word = self.statement_word()?;
        match word {
            "import" => return self.import_help(),
            "try" => {
                return Some(
                    "Frost has no `try`; to catch an error, call a function with `try_call(f)`"
                        .to_owned(),
                );
            }
            "elsif" | "elseif" => return Some(ELIF_HELP.to_owned()),
            _ => {}
        }
        let Token::Identifier(found) = self.token_at(0)? else {
            return None;
        };
        // `let mut x` or `local function f`: the name follows a modifier.
        let (word, name) = match self.token_at(1) {
            Some(Token::Identifier(name))
                if MODIFIER_WORDS.contains(found) || FUNCTION_WORDS.contains(found) =>
            {
                let word = if FUNCTION_WORDS.contains(found) {
                    *found
                } else {
                    word
                };
                (word, *name)
            }
            _ => (word, *found),
        };
        if DECLARATION_WORDS.contains(&word) {
            Some(declaration_help(word, name))
        } else if FUNCTION_WORDS.contains(&word) {
            Some(format!(
                "Frost has no `{word}`; define a function with `defn {name}(...) -> ...`"
            ))
        } else {
            None
        }
    }

    /// Help for a word from another language read as a whole expression where one
    /// begins, as `return` is in `return x` or `fn x -> return x`.
    fn expression_word_help(&self) -> Option<String> {
        let word = self.expression_word()?;
        let found = self.token_at(0)?;
        let starts_statement = self.starts_statement(self.found - 1);
        match word {
            "return" if starts_statement || starts_an_expression(found) => {
                Some("Frost has no `return`; a function's value is its last expression".to_owned())
            }
            // `lambda x: x`, or Python's `lambda: 1`
            "lambda"
                if starts_statement || matches!(found, Token::Identifier(_) | Token::Colon) =>
            {
                Some(LAMBDA_HELP.to_owned())
            }
            "throw" | "raise" if starts_an_expression(found) => Some(format!(
                "Frost has no `{word}`; raise an error with `error(value)`"
            )),
            // `if c: let x = 1`; at a statement's start, the statement rules take it.
            _ if !starts_statement && DECLARATION_WORDS.contains(&word) => {
                match (found, self.token_at(1)) {
                    (Token::Identifier(name), Some(Token::Assign)) => {
                        Some(declaration_help(word, name))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Help for `import` written as a statement, as in `import std.math`.
    fn import_help(&self) -> Option<String> {
        let found = self.at(0)?;
        // The path's last token
        let mut last = self.found;
        let path = match found.token {
            // `import std.math`: the dotted names, written touching
            Token::Identifier(_) => {
                while let (Some(dot), Some(name)) = (self.ctx.get(last + 1), self.ctx.get(last + 2))
                    && dot.token == Token::OpDot
                    && matches!(name.token, Token::Identifier(_))
                    && self.ctx.get(last)?.span.end == dot.span.start
                    && dot.span.end == name.span.start
                {
                    last += 2;
                }
                let end = self.ctx.get(last)?.span.end;
                self.ctx.source_text((found.span.start..end).into())
            }
            Token::SingleQuoteStringLiteral(path) | Token::DoubleQuoteStringLiteral(path) => path,
            _ => return None,
        };
        let name = match (self.ctx.get(last + 1), self.ctx.get(last + 2)) {
            // `import numpy as np`
            (Some(as_word), Some(name)) if as_word.token == Token::KwAs => match name.token {
                Token::Identifier(name) => Some(name),
                _ => None,
            },
            _ => path.rsplit(['.', '/']).next().filter(|name| {
                name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            }),
        }
        .unwrap_or("module");
        let quote = if path.contains('\'') { '"' } else { '\'' };
        let help = format!(
            "load a module by calling `import`: `def {name} = import({quote}{path}{quote})`"
        );
        // `import math`: the host's module, or Frost's own, which is under `std.`
        if path.contains(['.', '/']) {
            return Some(help);
        }
        Some(format!(
            "{help}; Frost's own modules are under `std.`, like `std.math`"
        ))
    }

    /// Help for a keyword written where a name belongs.
    pub(super) fn keyword_as_name_help(&self) -> Option<String> {
        let found = self.token_at(0)?;
        if !is_word_keyword(found) {
            return None;
        }
        let previous = self.token_at(-1);
        let next = self.token_at(1);
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
        let kind = self.innermost_kind();
        let is_map = matches!(kind, Some(Bracket::MapLiteral | Bracket::MapPattern));
        // `true` and `false` make literal keys, with help of their own.
        if next == Some(&Token::Colon) && is_map && !matches!(found, Token::KwTrue | Token::KwFalse)
        {
            return Some(format!(
                "`{found}` is a keyword; write the key in brackets: `[\"{found}\"]: ...`"
            ));
        }
        // `{map}` or `{filter, x}`, a keyword as a shorthand entry. Braces of `true`,
        // `false`, or `null`, as in `{true, null}`, are another language's set literal.
        if is_map
            && matches!(next, Some(Token::Comma | Token::CloseBrace))
            && matches!(
                self.token_before(self.found),
                Some(Token::OpenBrace | Token::Comma)
            )
            && !matches!(found, Token::KwTrue | Token::KwFalse | Token::KwNull)
        {
            return Some(keyword_name_help(found));
        }
        // A keyword starting a pattern is a name only when a name's pattern ends after it.
        let pattern_name = || {
            matches!(
                self.ctx.get_past_nl(self.found + 1).map(|(_, t)| &t.token),
                Some(
                    Token::FatArrow
                        | Token::Comma
                        | Token::Pipe
                        | Token::CloseBrace
                        | Token::CloseBracket
                        | Token::KwIs
                        | Token::KwIf
                        | Token::KwAs
                        | Token::Assign
                )
            )
        };
        if kind == Some(Bracket::MatchArms) && self.at_a_pattern_start() {
            return match found {
                // `else => 1` or `else: 1`
                Token::KwElse if next == Some(&Token::Colon) || pattern_name() => {
                    Some(CATCH_ALL_HELP.to_owned())
                }
                _ => pattern_name().then(|| keyword_name_help(found)),
            };
        }
        // `let map = 1`, both another language's declaration and a keyword as a name;
        // other words may be variables, as in `val if c else 2`.
        if let Some(word) = self.statement_word()
            && DECLARATION_WORDS.contains(&word)
            && next == Some(&Token::Assign)
        {
            return Some(format!(
                "Frost has no `{word}`; bind with `def`, and {}",
                keyword_name_help(found)
            ));
        }
        let in_destructure = matches!(kind, Some(Bracket::MapPattern | Bracket::ArrayPattern))
            && matches!(
                self.token_before(self.found),
                Some(Token::OpenBrace | Token::OpenBracket | Token::Comma | Token::Colon)
            )
            && pattern_name();
        let names_here = in_destructure
            || matches!(
                previous,
                Some(Token::KwDef | Token::KwDefn | Token::KwFn | Token::DotDotDot | Token::KwAs)
            )
            || (matches!(previous, Some(Token::OpenParen | Token::Comma))
                && kind == Some(Bracket::Parameters))
            || self.in_bare_parameters()
            // `function filter(x)`
            || self.statement_word().is_some_and(|word| {
                FUNCTION_WORDS.contains(&word) && next == Some(&Token::OpenParen)
            });
        names_here.then(|| keyword_name_help(found))
    }

    /// Help for a word from another language after an operand, as in `x in xs`,
    /// `x unless c`, or `[x for x in xs]`.
    pub(super) fn operand_word_help(&self) -> Option<String> {
        if !self.follows_an_operand(0) {
            return None;
        }
        let help = match self.token_at(0)? {
            Token::Identifier("in") => IN_HELP,
            // `x not in xs`
            Token::OpNot if self.token_at(1) == Some(&Token::Identifier("in")) => IN_HELP,
            // `x is None`; in a pattern, `is` is Frost's own.
            Token::KwIs if !self.in_patterns() => return Some(self.type_test_outside_a_match()),
            // `_ unless: c => 1`
            Token::Identifier("unless") if self.arm_part() == Some(ArmPart::Pattern) => {
                "a guard is written `if:`; negate it with `not`, like `_ if: not cond => ...`"
            }
            Token::Identifier("unless") => UNLESS_HELP,
            Token::Identifier("elsif" | "elseif") => ELIF_HELP,
            // `fn x -> x end`
            Token::Identifier("end") if self.ends_statement(self.found + 1) => {
                "Frost has no `end`; delete it"
            }
            Token::Identifier("for")
                if matches!(
                    self.innermost_kind(),
                    Some(Bracket::ArrayLiteral | Bracket::Group | Bracket::Call)
                ) =>
            {
                "Frost has no comprehensions; use `map` and `filter`, like \
                 `map xs with fn x -> ...`"
            }
            // `reduce xs with f from 0`
            Token::Identifier("from") | Token::KwInit
                if self
                    .nearest_on_line(|token| *token == Token::KwReduce)
                    .is_some() =>
            {
                "an initial value goes before `with`: `reduce xs init: v with f`"
            }
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Help for `is` outside a `match` pattern, as in `x is None` or `x is not Int`.
    fn type_test_outside_a_match(&self) -> String {
        let negated = self.token_at(1) == Some(&Token::OpNot);
        let tested = self.token_at(if negated { 2 } else { 1 });
        let type_name = tested.and_then(|token| match token {
            Token::Identifier(name) => type_named(name),
            keyword if is_word_keyword(keyword) => type_named(&keyword.to_string()),
            _ => None,
        });
        let Some(type_name) = type_name else {
            // `a is b`, Python's identity test
            if matches!(tested, Some(Token::Identifier(name)) if !is_type_like(name)) {
                return "`is` works only in a `match` pattern; to compare values, use `==` \
                        or `!=`"
                    .to_owned();
            }
            return "`is` works only in a `match` pattern; elsewhere, test a type with a \
                    function like `is_int(x)`"
                .to_owned();
        };
        // The name before `is` is the value tested only when it is the whole operand,
        // not the end of one, as `k` is in `m.k` and `b` is in `a + b`. `and`, `or`, and
        // `not` bind looser than `is` would, so the name after one is the whole operand.
        let value = match (self.token_at(-2), self.token_at(-1)) {
            (before, Some(Token::Identifier(name)))
                if !before.is_some_and(|before| {
                    (is_binary_operator(before) && !matches!(before, Token::OpAnd | Token::OpOr))
                        || matches!(before, Token::OpDot | Token::OpMinus | Token::OpThread)
                }) =>
            {
                name
            }
            _ => "x",
        };
        let not = if negated { "not " } else { "" };
        format!(
            "`is` works only in a `match` pattern; elsewhere, use `{not}is_{}({value})`",
            type_name.to_lowercase()
        )
    }

    /// Help for `token`, found after `is` in a pattern, where a type name belongs.
    pub(super) fn type_test_help(&self, token: &Token) -> Option<String> {
        match token {
            // `n is not Int`
            Token::OpNot => {
                // `_` binds nothing for the guard to test.
                let name = match self.token_at(-2) {
                    Some(Token::Identifier(name)) if *name != "_" => name,
                    _ => "n",
                };
                let test = match self.token_at(1) {
                    Some(Token::Identifier(type_name)) => type_named(type_name),
                    _ => None,
                }
                .unwrap_or("Int")
                .to_lowercase();
                Some(format!(
                    "to exclude a type, use a guard: `{name} if: not is_{test}({name}) => ...`"
                ))
            }
            Token::Identifier(name) => Some(type_name_help(name)),
            // `n is null`
            keyword if is_word_keyword(keyword) => Some(type_name_help(&keyword.to_string())),
            _ => None,
        }
    }

    /// Help for `else` or `elif` starting a statement after a complete one: after an
    /// `if` whose branch took several statements, or with no `if` before it to continue.
    pub(super) fn stray_branch_help(&self) -> Option<String> {
        if !self.starts_statement(self.found)
            || !self.at_statement_level(self.ctx.innermost_bracket())
            || !self.token_before(self.found).is_none_or(ends_an_expression)
        {
            return None;
        }
        // After an `else`, as in a second `else`, no branch continues.
        let nearest_branch = (0..self.found).rev().find_map(|index| {
            self.ctx
                .get(index)
                .map(|t| &t.token)
                .filter(|token| matches!(token, Token::KwIf | Token::KwElif | Token::KwElse))
        });
        let follows_an_if = matches!(nearest_branch, Some(Token::KwIf | Token::KwElif));
        Some(if follows_an_if {
            "an `if` branch is one expression; for several statements, use `do { ... }`".to_owned()
        } else {
            CONDITIONAL_HELP.to_owned()
        })
    }

    /// Help for a `{` after what reads as a function's header: a return type, as in
    /// `defn f(x) -> Int { x }`, or another language's function word, as in
    /// `function(a) { a }`.
    pub(super) fn brace_after_header_help(&self) -> Option<String> {
        if self.token_at(-2) == Some(&Token::SlimArrow)
            && matches!(self.token_at(-1), Some(Token::Identifier(name)) if is_type_like(name))
        {
            return Some(RETURN_TYPE_HELP.to_owned());
        }
        let close = self.found.checked_sub(1)?;
        if self.ctx.get(close)?.token != Token::CloseParen {
            return None;
        }
        let word = &self
            .ctx
            .get(self.matching_open(close)?.checked_sub(1)?)?
            .token;
        matches!(word, Token::Identifier(word) if FUNCTION_WORDS.contains(word))
            .then(|| LAMBDA_HELP.to_owned())
    }

    /// Help for a call written without parentheses, as in `print "hi"`, or a function
    /// defined by naming its parameters, as in `f x = x + 1`.
    pub(super) fn call_help(&self) -> Option<String> {
        if !self.at_statement_level(self.ctx.innermost_bracket()) {
            return None;
        }
        let callee = self.expression_word()?;
        let found = self.token_at(0)?;
        let callee_starts_statement = self.starts_statement(self.found - 1);
        if callee_starts_statement
            && !TYPE_DECLARATION_WORDS.contains(&callee)
            && self.names_then_assign()
        {
            return Some(define_function_help(callee));
        }
        let is_argument = is_literal(found) || matches!(found, Token::Identifier(_));
        // `a xor b` or `x unless c`: a word after the argument makes it no argument.
        let more_words = self
            .token_at(1)
            .is_some_and(|next| is_literal(next) || matches!(next, Token::Identifier(_)));
        // At a statement's start, a word followed by a name is often another language's
        // declaration, as in `package main`. A word touching a String is its prefix.
        let is_call = is_argument
            && !more_words
            && self.abutting_previous().is_none()
            && (!callee_starts_statement || PRINT_WORDS.contains(&callee));
        is_call.then(|| CALL_HELP.to_owned())
    }

    /// Whether lowercase names, the first of them the error's, run to an `=`, as the
    /// parameters do in `f x y = x + y`.
    fn names_then_assign(&self) -> bool {
        let mut index = self.found;
        while let Some(Token::Identifier(name)) = self.ctx.get(index).map(|t| &t.token) {
            if !name.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') {
                return false;
            }
            index += 1;
        }
        index > self.found && self.ctx.get(index).map(|t| &t.token) == Some(&Token::Assign)
    }
}

/// Help for `keyword` written where a name belongs.
fn keyword_name_help(keyword: &Token) -> String {
    format!("`{keyword}` is a keyword, so it cannot be a name")
}

/// Help for `word`, another language's declaration, declaring `name`.
fn declaration_help(word: &str, name: &str) -> String {
    format!("Frost has no `{word}`; bind a name with `def`: `def {name} = ...`")
}

/// The Frost type that `name`, written as another language or another case would write
/// it, names.
pub(super) fn type_named(name: &str) -> Option<&'static str> {
    if matches!(name, "None" | "nil") {
        return Some("Null");
    }
    TYPE_CONSTRAINTS
        .iter()
        .find(|(type_name, _)| type_name.eq_ignore_ascii_case(name))
        .map(|(type_name, _)| *type_name)
}

/// Help for `name` written where a type name belongs, after `is`.
fn type_name_help(name: &str) -> String {
    match type_named(name) {
        Some(type_name) => format!("did you mean `{type_name}`?"),
        None => {
            let names: Vec<String> = TYPE_CONSTRAINTS
                .iter()
                .map(|(type_name, _)| format!("`{type_name}`"))
                .collect();
            format!("the types are {}", names.join(", "))
        }
    }
}

/// Whether `name` looks like a type's name in another language: capitalized, a Frost
/// type's name in another case, or a common lowercase type name.
pub(super) fn is_type_like(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
        || type_named(name).is_some()
        || LOWERCASE_TYPE_NAMES.contains(&name)
}
