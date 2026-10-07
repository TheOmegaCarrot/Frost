//! What the tokens around an error make of it: the queries the rules are built from.

use crate::lex::Token;
use crate::parse::ctx::{Bracket, OpenBracket, SrcToken};
use crate::parse::hints::Site;

impl<'src> Site<'_, 'src> {
    // -- Tokens and statements --

    /// The token `offset` places from the error's.
    pub(super) fn at(&self, offset: isize) -> Option<&SrcToken<'src>> {
        self.ctx.get(self.found.checked_add_signed(offset)?)
    }

    /// The token `offset` places from the error's, without its span.
    pub(super) fn token_at(&self, offset: isize) -> Option<&Token<'src>> {
        self.at(offset).map(|t| &t.token)
    }

    /// Whether the token at `index` begins a statement: it starts the input, a line,
    /// or follows a `;` or the `{` of a block.
    pub(super) fn starts_statement(&self, index: usize) -> bool {
        match index.checked_sub(1) {
            None => true,
            Some(before) => matches!(
                self.ctx.get(before).map(|t| &t.token),
                None | Some(Token::Newline | Token::Semicolon | Token::OpenBrace)
            ),
        }
    }

    /// Whether the token at `index` ends a statement, as a line break, a `;`, or the end
    /// of input does.
    pub(super) fn ends_statement(&self, index: usize) -> bool {
        matches!(
            self.ctx.get(index).map(|t| &t.token),
            None | Some(Token::Newline | Token::Semicolon)
        )
    }

    /// The word before the error, when it alone was read as a whole statement.
    pub(super) fn statement_word(&self) -> Option<&'src str> {
        match self.at(-1)?.token {
            Token::Identifier(word) if self.starts_statement(self.found - 1) => Some(word),
            _ => None,
        }
    }

    /// The word before the error, when it alone was read as a whole expression where
    /// one begins: it starts a statement, or follows `->`, `=>`, `:`, `(`, `,`, `=`,
    /// or `with`, as `return` does in `fn x -> return x`.
    pub(super) fn expression_word(&self) -> Option<&'src str> {
        let Token::Identifier(word) = self.at(-1)?.token else {
            return None;
        };
        let begins_expression = self.starts_statement(self.found - 1)
            || matches!(
                self.token_before(self.found - 1),
                Some(
                    Token::SlimArrow
                        | Token::FatArrow
                        | Token::Colon
                        | Token::OpenParen
                        | Token::Comma
                        | Token::Assign
                        | Token::KwWith
                )
            );
        begins_expression.then_some(word)
    }

    /// Whether a place whose innermost enclosing bracket is `open` holds statements:
    /// true when no bracket is open there and this is not an interpolation, or when
    /// that bracket is a block.
    pub(super) fn at_statement_level(&self, open: Option<OpenBracket>) -> bool {
        match open {
            None => !self.ctx.in_interpolation(),
            Some(open) => open.kind == Bracket::Block,
        }
    }

    /// The kind of the innermost open bracket, if any.
    pub(super) fn innermost_kind(&self) -> Option<Bracket> {
        self.ctx.innermost_bracket().map(|open| open.kind)
    }

    /// The index of the first token inside the bracket opened at `open`, past any
    /// line breaks.
    pub(super) fn first_inside(&self, open: OpenBracket) -> Option<usize> {
        self.ctx.get_past_nl(open.pos + 1).map(|(index, _)| index)
    }

    /// The index of the nearest token before the one at `index` that is not a line break.
    pub(super) fn index_before(&self, index: usize) -> Option<usize> {
        (0..index).rev().find(|&before| {
            self.ctx
                .get(before)
                .is_some_and(|t| t.token != Token::Newline)
        })
    }

    /// The nearest token before the one at `index` that is not a line break.
    pub(super) fn token_before(&self, index: usize) -> Option<&Token<'src>> {
        Some(&self.ctx.get(self.index_before(index)?)?.token)
    }

    /// The previous token, when it touches the error's with no space between.
    pub(super) fn abutting_previous(&self) -> Option<&Token<'src>> {
        let (previous, found) = (self.at(-1)?, self.at(0)?);
        (previous.span.end == found.span.start).then_some(&previous.token)
    }

    /// The next token, when it touches the error's with no space between.
    pub(super) fn abutting_next(&self) -> Option<&Token<'src>> {
        let (found, next) = (self.at(0)?, self.at(1)?);
        (found.span.end == next.span.start).then_some(&next.token)
    }

    /// Whether the token `offset` places from the error's follows one that can end an
    /// expression.
    pub(super) fn follows_an_operand(&self, offset: isize) -> bool {
        self.token_at(offset - 1).is_some_and(ends_an_expression)
    }

    /// The index of the token that starts the statement holding the token at `index`.
    pub(super) fn statement_start(&self, index: usize) -> usize {
        (0..=index)
            .rev()
            .find(|&start| self.starts_statement(start))
            .unwrap_or(0)
    }

    /// Whether `name` is written as a name before the token at `index`, as `x` is
    /// before `x = x + 1` in `def x = 1`.
    pub(super) fn named_before(&self, index: usize, name: &str) -> bool {
        (0..index).any(|before| {
            matches!(
                self.ctx.get(before).map(|t| &t.token),
                Some(Token::Identifier(written)) if *written == name
            )
        })
    }

    /// The nearest token before the error and on its line that `wanted` accepts, with
    /// its index, offered nearest first. A `;` ends the search as a line break does.
    pub(super) fn nearest_on_line(
        &self,
        mut wanted: impl FnMut(&Token) -> bool,
    ) -> Option<(usize, &Token<'_>)> {
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
                    .map(|t| (index, &t.token))
            })
            .find(|(_, token)| wanted(token))
    }

    /// The names at `index` and after, separated by commas, as in `a, b, c`, with the
    /// index of the token after the last name.
    pub(super) fn names_from(&self, mut index: usize) -> (Vec<&'src str>, usize) {
        let mut names = Vec::new();
        while let Some(Token::Identifier(name)) = self.ctx.get(index).map(|t| &t.token) {
            names.push(*name);
            if self.ctx.get(index + 1).map(|t| &t.token) != Some(&Token::Comma) {
                return (names, index + 1);
            }
            index += 2;
        }
        (names, index)
    }

    // -- Brackets --

    /// The index of the opener that the closer at `close` closes, if a closer is there.
    pub(super) fn matching_open(&self, close: usize) -> Option<usize> {
        if !is_closer(&self.ctx.get(close)?.token) {
            return None;
        }
        let mut depth = 0usize;
        for index in (0..=close).rev() {
            let token = &self.ctx.get(index)?.token;
            if is_closer(token) {
                depth += 1;
            } else if is_opener(token) {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
        }
        None
    }

    /// The index of the closer that closes the opener at `open`, if one does.
    pub(super) fn matching_close(&self, open: usize) -> Option<usize> {
        let mut depth = 0usize;
        for index in open.. {
            let token = &self.ctx.get(index)?.token;
            if is_opener(token) {
                depth += 1;
            } else if is_closer(token) {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
        }
        None
    }

    /// Whether a closer of its own kind closes the opener at `open`, with every bracket
    /// between closed by its own kind too. A missing closer leaves the next closer to
    /// another bracket, as the `}` is left in `match x { 1 => f(2 }`.
    pub(super) fn closes_in_kind(&self, open: usize) -> bool {
        let mut awaited = Vec::new();
        for index in open.. {
            let Some(token) = self.ctx.get(index).map(|t| &t.token) else {
                return false;
            };
            match token {
                Token::OpenParen | Token::DollarParen => awaited.push(Token::CloseParen),
                Token::OpenBracket => awaited.push(Token::CloseBracket),
                Token::OpenBrace => awaited.push(Token::CloseBrace),
                closer if is_closer(closer) => {
                    if awaited.pop().as_ref() != Some(closer) {
                        return false;
                    }
                    if awaited.is_empty() {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Whether `wanted` follows the error at its depth before the next comma or the
    /// closer of the bracket around it, as the next arm's `=>` does after the `;` in
    /// `1 => 2; 3 => 4`.
    pub(super) fn follows_in_item(&self, wanted: &Token) -> bool {
        let mut depth = 0usize;
        for index in self.found + 1.. {
            match self.ctx.get(index).map(|t| &t.token) {
                Some(token) if depth == 0 && token == wanted => return true,
                Some(token) if is_opener(token) => depth += 1,
                Some(token) if is_closer(token) => match depth.checked_sub(1) {
                    Some(outer) => depth = outer,
                    None => return false,
                },
                Some(Token::Comma) if depth == 0 => return false,
                None => return false,
                _ => {}
            }
        }
        false
    }

    /// Whether the brace opened at `open`, now closed, held a `match`'s arms: a `match`
    /// stands before it in its statement, outside other brackets.
    pub(super) fn opened_match_arms(&self, open: usize) -> bool {
        let mut depth = 0usize;
        for index in (self.statement_start(open)..open).rev() {
            match self.ctx.get(index).map(|t| &t.token) {
                Some(Token::KwMatch) if depth == 0 => return true,
                Some(token) if is_closer(token) => depth += 1,
                Some(token) if is_opener(token) => match depth.checked_sub(1) {
                    Some(outer) => depth = outer,
                    None => return false,
                },
                _ => {}
            }
        }
        false
    }

    /// The commas directly inside the bracket opened at `open`, not inside a bracket
    /// within it, when the bracket closes.
    pub(super) fn commas_inside(&self, open: usize) -> Option<usize> {
        let close = self.matching_close(open)?;
        let mut depth = 0usize;
        let mut commas = 0;
        for index in open + 1..close {
            match &self.ctx.get(index)?.token {
                token if is_opener(token) => depth += 1,
                token if is_closer(token) => depth -= 1,
                Token::Comma if depth == 0 => commas += 1,
                _ => {}
            }
        }
        Some(commas)
    }

    /// The name called just before the error, when that call starts a statement,
    /// as in `f(x) = 1`.
    pub(super) fn called_name_starting_statement(&self) -> Option<&'src str> {
        let close = self.found.checked_sub(1)?;
        if self.ctx.get(close)?.token != Token::CloseParen {
            return None;
        }
        let callee = self.matching_open(close)?.checked_sub(1)?;
        match self.ctx.get(callee)?.token {
            Token::Identifier(name) if self.starts_statement(callee) => Some(name),
            _ => None,
        }
    }

    /// Whether only names and commas stand between the bracket opened at `open` and
    /// the error, as before the arrow of `{ x, y -> x + y }`.
    pub(super) fn only_names_inside(&self, open: OpenBracket) -> bool {
        (open.pos + 1..self.found).all(|index| {
            matches!(
                self.ctx.get(index).map(|t| &t.token),
                Some(Token::Identifier(_) | Token::Comma | Token::Newline)
            )
        })
    }

    /// Whether the bracket opened at `open`, the innermost one, starts a `match` arm's
    /// pattern, or one of its alternatives.
    pub(super) fn opens_a_pattern(&self, open: OpenBracket) -> bool {
        self.ctx.outer_bracket().map(|outer| outer.kind) == Some(Bracket::MatchArms)
            && matches!(
                self.token_before(open.pos),
                Some(Token::OpenBrace | Token::Comma | Token::Pipe)
            )
    }

    // -- Names and patterns --

    /// Whether the error follows `fn a,` or `fn a, b,`: a lambda's parameters
    /// without parentheses.
    pub(super) fn in_bare_parameters(&self) -> bool {
        let token = |index: usize| self.ctx.get(index).map(|t| &t.token);
        let mut index = self.found;
        while index >= 2
            && token(index - 1) == Some(&Token::Comma)
            && matches!(token(index - 2), Some(Token::Identifier(_)))
        {
            index -= 2;
        }
        index < self.found && index >= 1 && token(index - 1) == Some(&Token::KwFn)
    }

    /// Whether the error is where a name is bound: in a pattern or a parameter list.
    pub(super) fn binds_names(&self) -> bool {
        match self.innermost_kind() {
            Some(Bracket::ArrayPattern | Bracket::MapPattern | Bracket::Parameters) => true,
            Some(Bracket::MatchArms) => self.at_a_pattern_start(),
            _ => {
                matches!(self.token_at(-1), Some(Token::KwDef | Token::KwFn))
                    || self.in_bare_parameters()
            }
        }
    }

    /// Whether the error starts a `match` arm's pattern, or one of its alternatives.
    pub(super) fn at_a_pattern_start(&self) -> bool {
        self.pattern_starts_at(self.found)
    }

    /// Whether the token at `index` starts a `match` arm's pattern, or one of its
    /// alternatives.
    pub(super) fn pattern_starts_at(&self, index: usize) -> bool {
        matches!(
            self.token_before(index),
            Some(Token::OpenBrace | Token::Comma | Token::Pipe)
        )
    }

    /// The part of a `match` arm the error is in, when the innermost bracket holds
    /// `match` arms.
    pub(super) fn arm_part(&self) -> Option<ArmPart> {
        let open = self
            .ctx
            .innermost_bracket()
            .filter(|open| open.kind == Bracket::MatchArms)?;
        // The brackets between the arms' `{` and the error are closed.
        let mut depth = 0usize;
        let mut part = ArmPart::Pattern;
        for index in (open.pos + 1..self.found).rev() {
            match &self.ctx.get(index)?.token {
                token if is_closer(token) => depth += 1,
                token if is_opener(token) => depth = depth.saturating_sub(1),
                _ if depth > 0 => {}
                Token::Comma => break,
                Token::FatArrow => return Some(ArmPart::Result),
                Token::KwIf => part = ArmPart::Guard,
                _ => {}
            }
        }
        Some(part)
    }

    /// Whether the innermost bracket holds patterns: a `match`'s arms, or an Array or
    /// Map pattern.
    pub(super) fn in_patterns(&self) -> bool {
        matches!(
            self.innermost_kind(),
            Some(Bracket::MatchArms | Bracket::ArrayPattern | Bracket::MapPattern)
        )
    }

    // -- Literals and braces --

    /// The source text of the literal key at `index` and the number of tokens it spans,
    /// if a literal starts there that a Map key could be: one written in brackets.
    pub(super) fn literal_key(&self, index: usize) -> Option<(&'src str, usize)> {
        let start = self.ctx.get(index)?;
        let length = match start.token {
            Token::OpMinus
                if matches!(
                    self.ctx.get(index + 1)?.token,
                    Token::IntLiteral(_) | Token::FloatLiteral(_)
                ) =>
            {
                2
            }
            Token::IntLiteral(_)
            | Token::FloatLiteral(_)
            | Token::SingleQuoteStringLiteral(_)
            | Token::DoubleQuoteStringLiteral(_)
            | Token::RawStringLiteral(_)
            | Token::SingleQuoteFormatStringLiteral(_)
            | Token::DoubleQuoteFormatStringLiteral(_)
            | Token::BytesLiteral(_)
            | Token::KwTrue
            | Token::KwFalse => 1,
            _ => return None,
        };
        let end = self.ctx.get(index + length - 1)?.span.end;
        Some((self.ctx.source_text((start.span.start..end).into()), length))
    }

    /// Whether the token `offset` places from the error's is a Float written from its
    /// point, as `.5` is.
    pub(super) fn is_point_float(&self, offset: isize) -> bool {
        self.at(offset).is_some_and(|t| {
            matches!(t.token, Token::FloatLiteral(_))
                && self.ctx.source_text(t.span.clone().into()).starts_with('.')
        })
    }

    /// Whether a block could stand where the Map literal opened at `open`, the innermost
    /// bracket, does: as a statement, a branch, or a definition's value.
    pub(super) fn block_could_stand(&self, open: OpenBracket) -> bool {
        self.opens_a_branch(open)
            || self.token_before(open.pos) == Some(&Token::Assign)
            || (self.starts_statement(open.pos)
                && self.at_statement_level(self.ctx.outer_bracket()))
    }

    /// Whether the brace opened at `open`, the innermost bracket, starts a branch:
    /// an `if` branch or a `match` arm's result.
    pub(super) fn opens_a_branch(&self, open: OpenBracket) -> bool {
        let Some(before) = self.index_before(open.pos) else {
            return false;
        };
        match self.ctx.get(before).map(|t| &t.token) {
            Some(Token::FatArrow) => true,
            Some(Token::Colon) => match self.ctx.outer_bracket().map(|outer| outer.kind) {
                // A Map entry's colon is not a branch's.
                Some(Bracket::MapLiteral | Bracket::MapPattern) => false,
                // Nor is a guard's `if:`.
                Some(Bracket::MatchArms) => self.token_before(before) != Some(&Token::KwIf),
                _ => true,
            },
            _ => false,
        }
    }

    /// Whether the entry starting at `entry`, in the innermost bracket, a Map literal,
    /// may be a `match` arm after the Map was left unclosed: the entry starts a line,
    /// and the Map is in a `match`.
    pub(super) fn may_be_next_arm(&self, entry: usize) -> bool {
        let starts_line = entry
            .checked_sub(1)
            .and_then(|before| self.ctx.get(before))
            .is_some_and(|t| t.token == Token::Newline);
        starts_line && self.ctx.outer_bracket().map(|outer| outer.kind) == Some(Bracket::MatchArms)
    }

    /// The arithmetic operator touching the error's token from before, as in `+=`.
    pub(super) fn abutting_operator(&self) -> Option<&'static str> {
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

/// A part of a `match` arm.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ArmPart {
    /// The pattern, before any guard
    Pattern,
    /// The guard, from its `if` to the `=>`
    Guard,
    /// The result, after the `=>`
    Result,
}

/// Whether `token` opens a bracket.
pub(super) fn is_opener(token: &Token) -> bool {
    matches!(
        token,
        Token::OpenParen | Token::DollarParen | Token::OpenBracket | Token::OpenBrace
    )
}

/// Whether `token` closes a bracket.
pub(super) fn is_closer(token: &Token) -> bool {
    matches!(
        token,
        Token::CloseParen | Token::CloseBracket | Token::CloseBrace
    )
}

/// Whether `token` is a literal: a number, a String of any kind, Bytes, `true`,
/// `false`, or `null`.
pub(super) fn is_literal(token: &Token) -> bool {
    matches!(
        token,
        Token::IntLiteral(_)
            | Token::FloatLiteral(_)
            | Token::SingleQuoteStringLiteral(_)
            | Token::DoubleQuoteStringLiteral(_)
            | Token::RawStringLiteral(_)
            | Token::MultilineStringLiteral(_)
            | Token::SingleQuoteFormatStringLiteral(_)
            | Token::DoubleQuoteFormatStringLiteral(_)
            | Token::BytesLiteral(_)
            | Token::KwTrue
            | Token::KwFalse
            | Token::KwNull
    )
}

/// Whether `token` can end an expression: a name, a literal, or a closing bracket.
pub(super) fn ends_an_expression(token: &Token) -> bool {
    is_literal(token)
        || matches!(
            token,
            Token::Identifier(_) | Token::CloseParen | Token::CloseBracket | Token::CloseBrace
        )
}

/// Whether `token` starts an expression or statement, but not a Map entry.
pub(super) fn starts_an_expression_not_a_key(token: &Token) -> bool {
    is_literal(token)
        || matches!(
            token,
            Token::OpenParen
                | Token::OpenBrace
                | Token::DollarParen
                | Token::OpMinus
                | Token::OpNot
                | Token::KwIf
                | Token::KwDo
                | Token::KwMatch
                | Token::KwFn
                | Token::KwMap
                | Token::KwFilter
                | Token::KwReduce
                | Token::KwForeach
                | Token::KwDef
                | Token::KwDefn
        )
}

/// Whether `token` can start an expression.
pub(super) fn starts_an_expression(token: &Token) -> bool {
    !matches!(token, Token::KwDef | Token::KwDefn)
        && (starts_an_expression_not_a_key(token)
            || matches!(
                token,
                Token::Identifier(_) | Token::DollarIdentifier(_) | Token::OpenBracket
            ))
}

/// Whether `token` can start a `match` pattern.
pub(super) fn starts_a_pattern(token: &Token) -> bool {
    is_literal(token)
        || matches!(
            token,
            Token::Identifier(_)
                | Token::OpenBracket
                | Token::OpenBrace
                | Token::OpenParen
                | Token::OpMinus
        )
}

/// Whether `token` is an operator that stands only between two operands.
pub(super) fn is_binary_operator(token: &Token) -> bool {
    matches!(
        token,
        Token::OpPlus
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
    )
}

/// Whether `token` continues an expression after a name: an operator or a postfix.
pub(super) fn infix_or_postfix(token: &Token) -> bool {
    is_binary_operator(token)
        || matches!(
            token,
            Token::OpMinus | Token::OpDot | Token::OpThread | Token::OpenParen | Token::OpenBracket
        )
}

/// Whether `token` is a keyword spelled as a word, which looks like a name.
pub(super) fn is_word_keyword(token: &Token) -> bool {
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
