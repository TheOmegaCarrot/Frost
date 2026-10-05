//! Rules for an error where the parser expected one token.

use crate::lex::Token;
use crate::parse::ctx::Bracket;
use crate::parse::hints::site::{is_closer, is_opener, starts_an_expression};
use crate::parse::hints::words::{is_type_like, type_named};
use crate::parse::hints::{
    ALTERNATIVES_HELP, ANNOTATION_HELP, DEFAULTS_HELP, ELIF_HELP, EQUALITY_HELP, GUARD_HELP,
    LAMBDA_HELP, MAP_ENTRY_HELP, RETURN_TYPE_HELP, SLICE_HELP, Site, destructure_help,
    is_iterative_keyword, iterative_help, walrus_help,
};

const ARM_HELP: &str = "a `match` arm is written `pattern => result`";

const AS_HELP: &str = "`as` binds a whole Map pattern, like `{name} as person`";

impl Site<'_, '_, '_> {
    /// Help for the error, given the one token that was expected there.
    pub(super) fn expected_token_help(&self, expected: &Token) -> Option<String> {
        let found = self.token_at(0);
        let previous = self.token_at(-1);
        let help = match expected {
            Token::Colon => match previous {
                Some(Token::KwIf) => GUARD_HELP,
                Some(Token::KwElse) if found == Some(&Token::KwIf) => ELIF_HELP,
                Some(Token::KwElse) if found == Some(&Token::OpenBrace) => {
                    "`else` is followed by a colon; for a block, write `else: do { ... }`"
                }
                Some(Token::KwElse) => "`else` is followed by a colon: `else: ...`",
                _ => return self.missing_colon_help(),
            },
            Token::FatArrow => match found {
                // `n: Int => 1`
                Some(Token::Colon) => {
                    return Some(self.typed_name_help().unwrap_or(ARM_HELP.to_owned()));
                }
                Some(Token::SlimArrow) => ARM_HELP,
                // `n when n > 0 => n` or `n is Int and n > 1 => 2`
                Some(Token::Identifier("when") | Token::OpAnd) => GUARD_HELP,
                Some(Token::OpOr) => {
                    return Some(
                        self.typed_alternatives_help()
                            .unwrap_or(ALTERNATIVES_HELP.to_owned()),
                    );
                }
                Some(Token::KwAs) => AS_HELP,
                _ if previous == Some(&Token::Identifier("case")) => {
                    "Frost's `match` has no `case`; an arm is written `pattern => result`"
                }
                _ => return None,
            },
            Token::SlimArrow => match found? {
                Token::OpenBrace => "add `->` before the body: `-> { ... }`",
                Token::FatArrow => "a lambda's arrow is `->`: `fn x -> ...`",
                // `defn f(x): Int -> x`
                Token::Colon
                    if previous == Some(&Token::CloseParen)
                        && matches!(self.token_at(1), Some(Token::Identifier(_)))
                        && matches!(
                            self.token_at(2),
                            Some(Token::SlimArrow | Token::OpenBrace)
                        ) =>
                {
                    RETURN_TYPE_HELP
                }
                // `defn f(x) = x` or `defn f(x):`
                written @ (Token::Assign | Token::Colon)
                    if previous == Some(&Token::CloseParen) =>
                {
                    return Some(format!("a function body follows `->`, not `{written}`"));
                }
                // `fn x = 1 -> x`
                Token::Assign => DEFAULTS_HELP,
                // `fn x: Int -> x`, `fn x: Int = 1 -> x`, or `fn x: List[Int] -> x`, but
                // not `fn x: x + 1`, Python's lambda
                Token::Colon
                    if let Some(Token::Identifier(name)) = self.token_at(1)
                        && is_type_like(name)
                        && match self.token_at(2) {
                            None
                            | Some(
                                Token::SlimArrow
                                | Token::Comma
                                | Token::CloseParen
                                | Token::Assign
                                | Token::Newline
                                | Token::Semicolon,
                            ) => true,
                            Some(Token::OpenBracket) => {
                                name.starts_with(|c: char| c.is_ascii_uppercase())
                            }
                            _ => false,
                        } =>
                {
                    ANNOTATION_HELP
                }
                Token::Colon => LAMBDA_HELP,
                // `fn x y -> x`
                Token::Identifier(_)
                    if matches!(previous, Some(Token::Identifier(_)))
                        && self.arrow_ends_names() =>
                {
                    "separate parameters with `,`, like `fn x, y -> ...`"
                }
                // `defn f(x) x + 1`
                token
                    if starts_an_expression(token)
                        && matches!(previous, Some(Token::CloseParen | Token::Identifier(_))) =>
                {
                    "add `->` before the body: `-> ...`"
                }
                _ => return None,
            },
            Token::Assign => match found? {
                Token::KwAs => AS_HELP,
                // `def x := 1`
                Token::Colon if self.abutting_next() == Some(&Token::Assign) => {
                    let Some(Token::Identifier(name)) = previous else {
                        return None;
                    };
                    return Some(walrus_help(name));
                }
                // `def x: Int = 1`
                Token::Colon
                    if self.token_at(-2) == Some(&Token::KwDef)
                        && matches!(self.token_at(1), Some(Token::Identifier(_))) =>
                {
                    "Frost has no type annotations"
                }
                // `def a, b = 1, 2`
                Token::Comma if self.token_at(-2) == Some(&Token::KwDef) => {
                    let (names, after) = self.names_from(self.found - 1);
                    if self.ctx.get(after).map(|t| &t.token) != Some(&Token::Assign) {
                        return None;
                    }
                    return Some(destructure_help(&format!("[{}]", names.join(", "))));
                }
                _ => return None,
            },
            Token::KwDef if previous == Some(&Token::KwExport) => {
                "`export` takes a definition: `export def x = ...` or `export defn f(...) -> ...`"
            }
            Token::KwWith => {
                let (_, keyword) = self.nearest_on_line(is_iterative_keyword)?;
                return Some(iterative_help(keyword, false));
            }
            // `map(xs, f)` or `(a, b)`
            Token::CloseParen if found == Some(&Token::Comma) => return self.group_comma_help(),
            // `xs[1:2]`
            Token::CloseBracket
                if found == Some(&Token::Colon)
                    && self.innermost_kind() == Some(Bracket::Index) =>
            {
                SLICE_HELP
            }
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Help for a colon missing after a condition or a Map key.
    fn missing_colon_help(&self) -> Option<String> {
        // `if x in xs:` or `if x is Int:`
        if let Some(help) = self.operand_word_help() {
            return Some(help);
        }
        let found = self.token_at(0);
        // The colon is the condition's only when no bracket opened after its keyword is
        // still open.
        let open = self.ctx.innermost_bracket();
        let keyword = self
            .keyword_owing_colon()
            .filter(|&(index, _)| open.is_none_or(|open| open.pos < index));
        let help = match (keyword, open.map(|open| open.kind)) {
            // `if x = 1: 2`, where the colon is there
            (Some(_), _) if found == Some(&Token::Assign) => EQUALITY_HELP,
            // `if (c) { 1 }`
            (Some((_, keyword)), _) if found == Some(&Token::OpenBrace) => {
                return Some(format!(
                    "`{keyword}` takes a colon after its condition; for a block, \
                     write `{keyword} x: do {{ ... }}`"
                ));
            }
            (Some((_, keyword)), _) => {
                return Some(format!(
                    "`{keyword}` takes a colon after its condition: `{keyword} x: ...`"
                ));
            }
            // `{[k] 1}`
            (None, Some(Bracket::MapLiteral | Bracket::MapPattern)) => MAP_ENTRY_HELP,
            (None, _) => return None,
        };
        Some(help.to_owned())
    }

    /// The nearest `if` or `elif` before the error on its line whose condition's colon
    /// is not there, with its index. Read backward, each colon outside brackets is
    /// taken by the nearest `if`, `elif`, or `else` before it, as `y`'s is in
    /// `elif if y: 2`, where the `elif` owes the colon.
    fn keyword_owing_colon(&self) -> Option<(usize, &Token<'_>)> {
        let mut colons = 0usize;
        let mut depth = 0usize;
        self.nearest_on_line(|token| {
            match token {
                token if is_closer(token) => depth += 1,
                // An opener with nothing to close holds the error: the search ends there.
                token if is_opener(token) => match depth.checked_sub(1) {
                    Some(outer) => depth = outer,
                    None => return true,
                },
                _ if depth > 0 => {}
                Token::Colon => colons += 1,
                Token::KwIf | Token::KwElif | Token::KwElse => match colons.checked_sub(1) {
                    Some(fewer) => colons = fewer,
                    None => return *token != Token::KwElse,
                },
                _ => {}
            }
            false
        })
        .filter(|(_, token)| matches!(token, Token::KwIf | Token::KwElif))
    }

    /// Help for `n: Int` in a pattern, a type written as another language writes one.
    fn typed_name_help(&self) -> Option<String> {
        let Some(Token::Identifier(name)) = self.token_at(-1) else {
            return None;
        };
        let Some(Token::Identifier(written)) = self.token_at(1) else {
            return None;
        };
        let type_name = type_named(written)?;
        Some(format!(
            "test a type with `is`: `{name} is {type_name} => ...`"
        ))
    }

    /// Help for `or` after a type test, as in `n is Int or String`: each alternative
    /// tests its own type, so the name is written in each.
    fn typed_alternatives_help(&self) -> Option<String> {
        let (Some(Token::Identifier(name)), Some(Token::KwIs), Some(Token::Identifier(first))) =
            (self.token_at(-3), self.token_at(-2), self.token_at(-1))
        else {
            return None;
        };
        // `or String` or `or n is String`
        let second = match (self.token_at(1), self.token_at(2), self.token_at(3)) {
            (Some(Token::Identifier(_)), Some(Token::KwIs), Some(Token::Identifier(second))) => {
                second
            }
            (Some(Token::Identifier(second)), _, _) => second,
            _ => return None,
        };
        Some(format!(
            "pattern alternatives are separated by `|`, each with its own test: \
             `{name} is {first} | {name} is {second} => ...`"
        ))
    }

    /// Whether names, perhaps with commas between, run from the error to a `->` on its
    /// line, as the parameters do in `fn x y -> x`.
    fn arrow_ends_names(&self) -> bool {
        (self.found..)
            .map(|index| self.ctx.get(index).map(|t| &t.token))
            .find(|token| !matches!(token, Some(Token::Identifier(_) | Token::Comma)))
            .flatten()
            == Some(&Token::SlimArrow)
    }
}
