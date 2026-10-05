//! Rules for what brackets hold: braces read as one thing where another was meant,
//! lists missing their commas, and parentheses holding what Frost writes otherwise.

use crate::lex::{Token, tokens};
use crate::parse::ctx::{Bracket, OpenBracket};
use crate::parse::hints::site::{
    ends_an_expression, infix_or_postfix, is_closer, is_literal, is_opener, starts_a_pattern,
    starts_an_expression, starts_an_expression_not_a_key,
};
use crate::parse::hints::{
    BLOCK_HELP, CATCH_ALL_HELP, LAMBDA_HELP, MAP_ENTRY_HELP, MAP_REST_HELP, REST_HELP, SPREAD_HELP,
    Site, destructure_help, is_iterative_keyword, iterative_help,
};

impl<'src> Site<'_, 'src, '_> {
    /// Help for braces whose contents were read as one thing where another was meant:
    /// a Map where a block was meant, a block where a Map was, or a Map key not written
    /// as Frost writes one.
    pub(super) fn brace_help(&self) -> Option<String> {
        let open = self.ctx.innermost_bracket()?;
        match open.kind {
            Bracket::MapLiteral | Bracket::MapPattern => self.map_help(open),
            Bracket::Block => self.block_key_help(open),
            _ => None,
        }
    }

    /// Help for an error in the Map literal or pattern opened at `open`.
    fn map_help(&self, open: OpenBracket) -> Option<String> {
        let found = self.token_at(0)?;
        let next = self.token_at(1);
        let is_map_literal = open.kind == Bracket::MapLiteral;
        let first = self.first_inside(open) == Some(self.found);

        // `match { 1 => 2 }`, where the arms were read as a Map
        if is_map_literal
            && self.token_before(open.pos) == Some(&Token::KwMatch)
            && (*found == Token::FatArrow || next == Some(&Token::FatArrow))
        {
            return Some(
                "`match` takes the value to match before its `{`: `match x { ... }`".to_owned(),
            );
        }

        // At the start of an entry
        if matches!(
            self.token_before(self.found),
            Some(Token::OpenBrace | Token::Comma)
        ) {
            if let Some((key, length)) = self.literal_key(self.found) {
                let after_key = self.ctx.get_past_nl(self.found + length);
                match after_key.map(|(_, t)| &t.token) {
                    // `{"a": 1}`
                    Some(Token::Colon) => return Some(key_help(key)),
                    // `{"a" => 1}`
                    Some(Token::FatArrow) => {
                        return (!self.may_be_next_arm(self.found)).then(|| key_help(key));
                    }
                    // `{1, 2, 3}`
                    Some(Token::Comma) if is_map_literal && first => {
                        return Some(
                            "Frost has no set literal; use an Array, like `[1, 2, 3]`".to_owned(),
                        );
                    }
                    _ => {}
                }
            }
            match found {
                // `{:a => 1}`
                Token::Colon if matches!(next, Some(Token::Identifier(_))) => {
                    return Some(MAP_ENTRY_HELP.to_owned());
                }
                // `{**m, a: 1}`, or `def {a, **r} = m`
                Token::OpTimes if self.abutting_next() == Some(&Token::OpTimes) => {
                    let help = if is_map_literal {
                        SPREAD_HELP
                    } else {
                        MAP_REST_HELP
                    };
                    return Some(help.to_owned());
                }
                // `{,}`
                Token::Comma if first && next == Some(&Token::CloseBrace) => {
                    return Some("an empty Map is written `{}`".to_owned());
                }
                _ => {}
            }
        }

        // `{ 1 }`, `{ f(x) }`, or `{ def y = 1; y }`, meant as blocks
        let meant_as_block = if first {
            starts_an_expression_not_a_key(found)
        } else {
            let continues_a_name = self.first_inside(open) == self.found.checked_sub(1)
                && matches!(self.token_at(-1), Some(Token::Identifier(_)))
                && infix_or_postfix(found);
            // `{a.b: 1}`, whose colon shows a Map
            if continues_a_name && let Some(key) = self.unbracketed_key(open) {
                return Some(format!(
                    "a computed key goes in brackets, like `{{[{key}]: ...}}`"
                ));
            }
            continues_a_name
        };
        (is_map_literal && meant_as_block && self.block_could_stand(open))
            .then(|| BLOCK_HELP.to_owned())
    }

    /// The source of an expression written as a key without its brackets, from the
    /// first token in the braces opened at `open` to a `:` after the error on its line,
    /// as `a.b` is in `{a.b: 1}`. An `if` or a `,` before the `:` ends the search.
    fn unbracketed_key(&self, open: OpenBracket) -> Option<&'src str> {
        let first = self.first_inside(open)?;
        let mut depth = 0usize;
        for index in self.found.. {
            match &self.ctx.get(index)?.token {
                Token::Colon if depth == 0 => {
                    let span = self.ctx.get(first)?.span.start..self.ctx.get(index - 1)?.span.end;
                    return Some(self.ctx.source_text(span.into()));
                }
                token if is_opener(token) => depth += 1,
                token if is_closer(token) => depth = depth.checked_sub(1)?,
                Token::Newline | Token::Semicolon => return None,
                Token::Comma | Token::KwIf | Token::KwElif | Token::KwElse if depth == 0 => {
                    return None;
                }
                _ => {}
            }
        }
        None
    }

    /// Help for a literal key that starts the block opened at `open`, as in
    /// `fn -> { "a": 1 }`, where a Map was meant.
    fn block_key_help(&self, open: OpenBracket) -> Option<String> {
        if self.token_at(0)? != &Token::Colon {
            return None;
        }
        let first = self.first_inside(open)?;
        let (key, length) = self.literal_key(first)?;
        let after_key = self.ctx.get_past_nl(first + length).map(|(index, _)| index);
        if after_key != Some(self.found) {
            return None;
        }
        // After `do`, braces are always a block's.
        if self.token_before(open.pos) == Some(&Token::KwDo) {
            return Some(format!(
                "a `do` block holds statements; a Map is written without `do`, like `{}`",
                entry_example(key)
            ));
        }
        Some(key_help(key))
    }

    /// Help for list items separated by something other than a comma: a line break, as
    /// in a `match` with an arm on each line, or a `;`. The list must close as its own
    /// kind; otherwise its closer is likely what is missing, and the error's line is a
    /// statement after it.
    pub(super) fn separator_help(&self) -> Option<String> {
        let open = self.ctx.innermost_bracket()?;
        let items = list_items(open.kind)?;
        let found = self.token_at(0)?;
        if !self.closes_in_kind(open.pos) {
            return None;
        }
        if *found == Token::Semicolon && self.follows_an_operand(0) {
            // `1 => f(); g()`, where the arm's result was meant as statements; in
            // `1 => 2; 3 => 4`, the next arm's `=>` follows.
            if open.kind == Bracket::MatchArms && !self.follows_in_item(&Token::FatArrow) {
                return Some(
                    "a `match` arm's result is one expression; for several statements, \
                     use `do { ... }`"
                        .to_owned(),
                );
            }
            return Some(format!("separate {items} with `,`, not `;`"));
        }
        let previous = self.token_before(self.found);
        let on_a_later_line =
            self.token_at(-1) == Some(&Token::Newline) && previous.is_some_and(ends_an_expression);
        // `print("a"` then `"b")`: Python joins the Strings, which the String rule covers.
        if is_plain_string(found) && previous.is_some_and(is_plain_string) {
            return None;
        }
        let starts_an_item = match open.kind {
            Bracket::Call | Bracket::ArrayLiteral => starts_an_expression(found),
            Bracket::MatchArms => {
                starts_a_pattern(found)
                    || (*found == Token::KwElse
                        && matches!(self.token_at(1), Some(Token::FatArrow | Token::Colon)))
            }
            Bracket::ArrayPattern => starts_a_pattern(found),
            // A line starting `key` after an unclosed Map in a `match` may be the next arm.
            Bracket::MapLiteral => {
                matches!(found, Token::Identifier(_) | Token::OpenBracket)
                    && !self.may_be_next_arm(self.found)
            }
            Bracket::MapPattern => matches!(found, Token::Identifier(_) | Token::OpenBracket),
            Bracket::Parameters => matches!(found, Token::Identifier(_) | Token::DotDotDot),
            _ => false,
        };
        if !(on_a_later_line && starts_an_item) {
            return None;
        }
        // `if c: {` with a name on each line, meant as a block's statements. After `=`,
        // the names may be a Map's.
        if open.kind == Bracket::MapLiteral
            && self.only_names_inside(open)
            && self.block_could_stand(open)
            && self.token_before(open.pos) != Some(&Token::Assign)
        {
            return Some(BLOCK_HELP.to_owned());
        }
        let help = format!("separate {items} with `,`, even across lines");
        // `else => 'b'` on its own line
        if *found == Token::KwElse {
            return Some(format!("{help}; {CATCH_ALL_HELP}"));
        }
        Some(help)
    }

    /// Help for a comma in parentheses around an expression, as in `(a, b)`, where the
    /// parser expected the `)`.
    pub(super) fn group_comma_help(&self) -> Option<String> {
        let open = self.ctx.innermost_bracket()?;
        if open.kind != Bracket::Group {
            return None;
        }
        // `map(xs, f)`: the parenthesized collection is cut off at the comma.
        if let Some(keyword) = open
            .pos
            .checked_sub(1)
            .and_then(|before| self.ctx.get(before))
            .map(|t| &t.token)
            .filter(|token| is_iterative_keyword(token))
        {
            // `xs @ reduce(0, f)` passes `xs` as well.
            let threaded = self.token_before(open.pos - 1) == Some(&Token::OpThread);
            let arguments =
                self.commas_inside(open.pos).map_or(0, |commas| commas + 1) + usize::from(threaded);
            return Some(iterative_help(keyword, arguments >= 3));
        }
        Some(
            if self.holds_parameters(open) {
                LAMBDA_HELP
            } else {
                "Frost has no tuples; use an Array, like `[1, 2]`"
            }
            .to_owned(),
        )
    }

    /// Help for `()` before an arrow, as in `() => 1`.
    pub(super) fn empty_group_help(&self) -> Option<String> {
        let open = self.ctx.innermost_bracket()?;
        let is_empty_parameters = open.kind == Bracket::Group
            && open.pos + 1 == self.found
            && self.holds_parameters(open);
        is_empty_parameters.then(|| LAMBDA_HELP.to_owned())
    }

    /// Whether the parentheses opened at `open`, the innermost bracket, hold a lambda's
    /// parameters as another language writes them: an arrow follows them, as in
    /// `(a, b) => a + b`, and they do not start a `match` pattern.
    fn holds_parameters(&self, open: OpenBracket) -> bool {
        !self.opens_a_pattern(open)
            && self
                .matching_close(open.pos)
                .and_then(|close| self.ctx.get(close + 1))
                .is_some_and(|t| matches!(t.token, Token::FatArrow | Token::SlimArrow))
    }

    /// Help for a pattern or a literal where a parameter belongs, as in `fn [a, b] -> a`
    /// or `defn fib(0) -> 1`, or for a rest binding with no name, as in `[a, ...]`.
    pub(super) fn parameter_pattern_help(&self) -> Option<String> {
        let found = self.token_at(0)?;
        let previous = self.token_at(-1)?;
        let after_fn = *previous == Token::KwFn;
        let starts_a_parameter = after_fn
            || (self.innermost_kind() == Some(Bracket::Parameters)
                && matches!(previous, Token::OpenParen | Token::Comma));
        if !starts_a_parameter {
            let is_unnamed_rest = *previous == Token::DotDotDot
                && matches!(
                    found,
                    Token::CloseBracket | Token::CloseParen | Token::Comma | Token::SlimArrow
                )
                && self.binds_names();
            return is_unnamed_rest.then(|| {
                // `[..., a]`, where naming the rest would leave it out of place
                if *found == Token::Comma {
                    format!("{REST_HELP}, and comes last")
                } else {
                    REST_HELP.to_owned()
                }
            });
        }
        match found {
            Token::OpenBrace | Token::OpenBracket => {
                // `fn {a} -> a`; without the arrow, the braces may be something else.
                if after_fn
                    && self
                        .matching_close(self.found)
                        .and_then(|close| self.ctx.get(close + 1))
                        .is_none_or(|t| t.token != Token::SlimArrow)
                {
                    return None;
                }
                let pattern = if *found == Token::OpenBrace {
                    "{a, b}"
                } else {
                    "[a, b]"
                };
                Some(format!(
                    "to destructure an argument, use `def` in the body: \
                     `fn x -> {{ def {pattern} = x; ... }}`"
                ))
            }
            token if is_literal(token) => Some(
                "to handle particular values, `match` the argument in the body: \
                 `match n { 0 => ... }`"
                    .to_owned(),
            ),
            _ => None,
        }
    }

    /// Help for a comma where no list continues: after a rest binding, or between names
    /// at a statement's start, as in `x, y -> x` or `a, b = 1, 2`.
    pub(super) fn comma_help(&self) -> Option<String> {
        if self.token_at(-2) == Some(&Token::DotDotDot) {
            return Some("a `...rest` binding comes last, with nothing after it".to_owned());
        }
        if !self.at_statement_level(self.ctx.innermost_bracket()) {
            return None;
        }
        let first = self.found.checked_sub(1)?;
        let starts_statement = self.starts_statement(first);
        if !starts_statement && self.token_before(first) != Some(&Token::Assign) {
            return None;
        }
        let (names, after) = self.names_from(first);
        match self.ctx.get(after).map(|t| &t.token) {
            // `x, y -> x`
            Some(Token::SlimArrow | Token::FatArrow) => Some(LAMBDA_HELP.to_owned()),
            // `a, b = 1, 2`
            Some(Token::Assign) if starts_statement => {
                Some(destructure_help(&format!("[{}]", names.join(", "))))
            }
            _ => None,
        }
    }

    /// The source of the Array or Map literal just before the error, when it starts a
    /// statement, holds several items, and fits on one line, as `[a, b]` does in
    /// `[a, b] = [1, 2]`.
    pub(super) fn pattern_starting_statement(&self) -> Option<&'src str> {
        let close = self.found.checked_sub(1)?;
        let open = self.matching_open(close)?;
        if !matches!(
            self.ctx.get(open)?.token,
            Token::OpenBracket | Token::OpenBrace
        ) || !self.starts_statement(open)
            || self.commas_inside(open)? == 0
        {
            return None;
        }
        let span = self.ctx.get(open)?.span.start..self.ctx.get(close)?.span.end;
        let text = self.ctx.source_text(span.into());
        (!text.contains('\n')).then_some(text)
    }
}

/// The items of the list a `kind` bracket holds, as a help names them, if it holds a
/// comma-separated list.
fn list_items(kind: Bracket) -> Option<&'static str> {
    match kind {
        Bracket::MatchArms => Some("`match` arms"),
        Bracket::Call => Some("arguments"),
        Bracket::MapLiteral | Bracket::MapPattern => Some("Map entries"),
        Bracket::ArrayLiteral | Bracket::ArrayPattern => Some("Array elements"),
        Bracket::Parameters => Some("parameters"),
        Bracket::Block
        | Bracket::Group
        | Bracket::Index
        | Bracket::ComputedKey
        | Bracket::AbbreviatedLambda => None,
    }
}

/// Whether `token` is a String literal in single or double quotes.
fn is_plain_string(token: &Token) -> bool {
    matches!(
        token,
        Token::SingleQuoteStringLiteral(_) | Token::DoubleQuoteStringLiteral(_)
    )
}

/// Help for a literal Map key, `key`, written as Frost does not write one.
fn key_help(key: &str) -> String {
    let example = entry_example(key);
    if quoted_name(key).is_some() {
        format!("a Map key that is a name needs no quotes, like `{example}`")
    } else {
        format!("a Map key that is not a name goes in brackets: `{example}`")
    }
}

/// A Map with the literal key `key`, written as Frost writes it.
fn entry_example(key: &str) -> String {
    match quoted_name(key) {
        Some(name) => format!("{{{name}: ...}}"),
        None => format!("{{[{key}]: ...}}"),
    }
}

/// The name that `key` quotes, when it is a String in plain quotes holding only a
/// name, as `'a'` does.
fn quoted_name(key: &str) -> Option<&str> {
    let quote = key.chars().next().filter(|c| matches!(c, '\'' | '"'))?;
    let text = key.strip_prefix(quote)?.strip_suffix(quote)?;
    let mut tokens = tokens(text);
    match (tokens.next(), tokens.next()) {
        (Some((Ok(Token::Identifier(name)), _)), None) if name == text => Some(text),
        _ => None,
    }
}
