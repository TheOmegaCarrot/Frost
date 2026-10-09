//! Rules for operators and punctuation.

use crate::lex::Token;
use crate::parse::ctx::{Bracket, OpenBracket};
use crate::parse::hints::site::{
    ArmPart, ends_an_expression, is_binary_operator, is_literal, starts_an_expression,
};
use crate::parse::hints::{
    ALTERNATIVES_HELP, ANNOTATION_HELP, BITWISE_HELP, BLOCK_HELP, CALL_HELP, CATCH_ALL_HELP,
    DEFAULTS_HELP, EQUALITY_HELP, FORMAT_STRING_HELP, INEQUALITY_HELP, LAMBDA_HELP,
    LINE_CONTINUATION_HELP, MAP_ENTRY_HELP, MAP_REST_HELP, NAMED_ARGUMENTS_HELP, POWER_HELP,
    REST_HELP, SLICE_HELP, SPREAD_HELP, Site, define_function_help, destructure_help,
    float_point_help, is_iterative_keyword, raw_string_help, rebind_help, walrus_help,
};

const PATH_HELP: &str = "Frost has no `::`; reach a module's entries with `.`, like `math.sqrt(x)`";

const THREAD_HELP: &str = "Frost threads a value into a call with `@`: `x @ f()`";

const RANGE_HELP: &str =
    "Frost has no range operator; for a range of Ints, use `range(start, stop)`";

const COMMENT_HELP: &str = "a comment starts with `#`";

const OR_HELP: &str = "Frost's \"or\" is `or`";

const IMMUTABLE_HELP: &str =
    "Frost values are immutable; bind an updated value to a new name with `def`";

const ARRAY_REST_HELP: &str = "only Array patterns take a rest binding: `[a, ...rest]`";

const REST_AFTER_COMMA_HELP: &str = "a `...rest` binding follows a comma, as in `a, ...rest`";

const CALL_SPREAD_HELP: &str =
    "Frost has no spread; to pass an Array's elements as arguments, use `call(f, args)`";

impl Site<'_, '_> {
    /// Help for `..` or `...` written as a range, as in `1..n`, `xs[2..]`, or `1...5`,
    /// or as Lua joins Strings, as in `'a' .. b`, or for `..` written for a rest
    /// binding, as in `[a, ..rest]`.
    pub(super) fn dots_help(&self) -> Option<String> {
        // The offsets of the operands before and after the dots
        let (before, after) = match self.token_at(0)? {
            // `a..b`, `0..=n`, or `xs[2..]`, failing at the second `.`
            Token::OpDot if self.abutting_previous() == Some(&Token::OpDot) => {
                if !self.follows_an_operand(-1) {
                    return None;
                }
                (-2, 1)
            }
            // `1..5`, whose second `.` starts the Float `.5`
            Token::FloatLiteral(_)
                if self.abutting_previous() == Some(&Token::OpDot) && self.is_point_float(0) =>
            {
                (-2, 0)
            }
            // `..`, failing at the first `.`: after an operand in a pattern, as in
            // `1..3 => 2`, or with no operand before it
            Token::OpDot
                if self.abutting_next() == Some(&Token::OpDot)
                    || (self.abutting_next().is_some() && self.is_point_float(1)) =>
            {
                if !self.follows_an_operand(0) && self.binds_names() {
                    // `[a, ..rest]` or `[a, ..]`
                    if self.abutting_next() != Some(&Token::OpDot) {
                        return None;
                    }
                    let in_map = self.innermost_kind() == Some(Bracket::MapPattern);
                    return Some(if in_map { MAP_REST_HELP } else { REST_HELP }.to_owned());
                }
                (-1, if self.is_point_float(1) { 1 } else { 2 })
            }
            // `1...5`, `1 ...5`, or `a ... b`. A `...` touching only a name after it,
            // as in `[a ...rest]`, is a spread or rest missing its comma.
            Token::DotDotDot
                if self.follows_an_operand(0)
                    && match self.token_at(1) {
                        Some(Token::IntLiteral(_) | Token::FloatLiteral(_)) => true,
                        Some(Token::Identifier(_) | Token::OpenParen) => {
                            self.abutting_previous().is_some() == self.abutting_next().is_some()
                        }
                        _ => false,
                    } =>
            {
                (-1, 1)
            }
            _ => return None,
        };
        let operand_before = self.token_at(before);
        // `[first..rest]` or `[h..]`, a rest binding written with dots between
        if self.innermost_kind() == Some(Bracket::ArrayPattern) {
            if !matches!(operand_before, Some(Token::Identifier(_))) {
                return None;
            }
            let help = match self.token_at(after) {
                Some(Token::Identifier(_)) => REST_AFTER_COMMA_HELP,
                _ => REST_HELP,
            };
            return Some(help.to_owned());
        }
        // `1..3 => 2`. With a name, as in `a..b => 2`, the pattern is no clear range.
        let in_arm_pattern = self.arm_part() == Some(ArmPart::Pattern);
        if in_arm_pattern || self.innermost_kind() == Some(Bracket::MapPattern) {
            return (in_arm_pattern && operand_before.is_some_and(is_literal)).then(|| {
                "a pattern cannot be a range; use a guard, like `n if: n >= 1 and n <= 3 => ...`"
                    .to_owned()
            });
        }
        let joins_strings = [before, after]
            .into_iter()
            .any(|offset| self.token_at(offset).is_some_and(is_string));
        let help = if joins_strings {
            "Frost joins Strings with `+`"
        } else if self.innermost_kind() == Some(Bracket::Index) {
            SLICE_HELP
        } else {
            RANGE_HELP
        };
        Some(help.to_owned())
    }

    /// Help for `1.`, an Int with a point but no digits after it, which may end the input.
    /// `1..n` got the range help.
    pub(super) fn missing_digits_help(&self) -> Option<String> {
        let (int, dot) = (self.at(-2)?, self.at(-1)?);
        let is_point_without_digits = dot.token == Token::OpDot
            && matches!(int.token, Token::IntLiteral(_))
            && int.span.end == dot.span.start
            && !matches!(
                self.token_at(0),
                Some(Token::Identifier(_) | Token::FloatLiteral(_))
            );
        is_point_without_digits.then(|| float_point_help("1.0"))
    }

    /// Help for `x--`, read as `x - -` with nothing after it, so that the error is past
    /// the end of its statement.
    pub(super) fn increment_help(&self) -> Option<String> {
        let (first, second) = (self.at(-2)?, self.at(-1)?);
        let is_decrement = first.token == Token::OpMinus
            && second.token == Token::OpMinus
            && first.span.end == second.span.start
            && self.follows_an_operand(-2)
            && self.ends_statement(self.found);
        is_decrement.then(|| rebind_help("--"))
    }

    /// Help for `//` or `/*` written to start a comment, or `//` for floor division.
    pub(super) fn comment_help(&self) -> Option<String> {
        let found = self.token_at(0)?;
        let help = match (self.abutting_previous(), found) {
            // `7 // 2`, failing at the second `/`
            (Some(Token::OpDiv), Token::OpDiv) if self.follows_an_operand(-1) => {
                "Frost has no `//`; `/` on Ints already gives an Int, rounding toward zero; \
                 a comment starts with `#`"
            }
            (Some(Token::OpDiv), Token::OpDiv | Token::OpTimes) => COMMENT_HELP,
            // `// note`, failing at the first `/`: no operand came before it.
            (_, Token::OpDiv)
                if matches!(self.abutting_next(), Some(Token::OpDiv | Token::OpTimes)) =>
            {
                COMMENT_HELP
            }
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Help for tokens touching with no space between, the shape of a token Frost lacks:
    /// `**`, `===`, `++`, `<>`, `0xFF`, `1e`, `t.0`, and the like.
    pub(super) fn touching_help(&self) -> Option<String> {
        let found = self.token_at(0)?;
        let help = match (self.abutting_previous()?, found) {
            (Token::OpEq, Token::Assign) => EQUALITY_HELP,
            (Token::OpNeq, Token::Assign) | (Token::OpLt, Token::OpGt) => INEQUALITY_HELP,
            (Token::OpTimes, Token::OpTimes) => {
                return Some(format!("Frost has no `**`; {POWER_HELP}"));
            }
            // `xs ++ ys`, failing at the second `+`
            (Token::OpPlus, Token::OpPlus)
                if self.follows_an_operand(-1)
                    && self.token_at(1).is_some_and(starts_an_expression) =>
            {
                "Frost has no `++`; join with `+`, like `xs + ys`"
            }
            // `x++`
            (Token::OpPlus, Token::OpPlus) if self.follows_an_operand(-1) => {
                return Some(rebind_help("++"));
            }
            (Token::OpLt, Token::OpLt) | (Token::OpGt, Token::OpGt) => {
                "Frost has no shift operators"
            }
            (Token::DollarIdentifier(_), Token::IntLiteral(_)) => {
                "the placeholders are `$`, `$1` to `$9`, and `$$`"
            }
            // `1e` or `1.5e`
            (Token::IntLiteral(_) | Token::FloatLiteral(_), Token::Identifier(e @ ("e" | "E"))) => {
                let number = self.ctx.source_text(self.at(-1)?.span.clone().into());
                return Some(format!("an exponent needs digits, like `{number}{e}3`"));
            }
            (Token::IntLiteral(_), Token::Identifier(word)) => {
                if word.starts_with('_') {
                    "Frost number literals have no `_` separators"
                } else if self.ctx.source_text(self.at(-1)?.span.clone().into()) == "0"
                    && word.starts_with(['x', 'b', 'o', 'X', 'B', 'O'])
                {
                    "Frost number literals are decimal only"
                } else {
                    return None;
                }
            }
            // `t.0` or `f().0`, but not `1.2.3`
            (
                Token::Identifier(_) | Token::CloseParen | Token::CloseBracket | Token::CloseBrace,
                Token::FloatLiteral(_),
            ) if self.is_point_float(0) => "index an Array with brackets, like `xs[0]`",
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Help for an expression broken across lines where Frost does not continue it:
    /// after an operator or `=` that ends a line, or before an operator that starts the
    /// next one. The expression's rest must be there; otherwise it is only missing.
    pub(super) fn line_break_help(&self) -> Option<String> {
        let found = self.token_at(0)?;
        let rest = self.ctx.get_past_nl(self.found);
        // `m.`, then `b` on the next line, which parentheses would not continue
        if *found == Token::Newline && self.token_at(-1) == Some(&Token::OpDot) {
            return rest
                .is_some_and(|(_, rest)| matches!(rest.token, Token::Identifier(_)))
                .then(|| {
                    "move the `.` to the start of the next line, where it continues the \
                     expression"
                        .to_owned()
                });
        }
        let is_broken = match found {
            // `def x = a +` or `def x =`, then the rest on the next line. A rest starting
            // `.5` starts with a `.` that continues no line.
            Token::Newline => {
                self.token_at(-1).is_some_and(|previous| {
                    is_binary_operator(previous)
                        || matches!(
                            previous,
                            Token::OpMinus | Token::OpNot | Token::Assign | Token::OpThread
                        )
                }) && rest.is_some_and(|(_, rest)| {
                    starts_an_expression(&rest.token)
                        && !self
                            .ctx
                            .source_text(rest.span.clone().into())
                            .starts_with('.')
                })
            }
            // `def x = a`, then `+ b` on the next line
            token if is_binary_operator(token) => {
                self.token_at(-1) == Some(&Token::Newline)
                    && self
                        .token_before(self.found)
                        .is_some_and(ends_an_expression)
                    && self.token_at(1).is_some_and(starts_an_expression)
            }
            _ => false,
        };
        is_broken.then(|| LINE_CONTINUATION_HELP.to_owned())
    }

    /// Help for `=` where Frost takes none, `open` being the innermost bracket.
    pub(super) fn assignment_help(&self, open: Option<OpenBracket>) -> Option<String> {
        if let Some(op) = self.abutting_operator() {
            return Some(rebind_help(&format!("{op}=")));
        }
        // An assignment's target ends just before its `=`.
        if !ends_an_expression(self.token_at(-1)?) {
            return None;
        }
        if self.at_statement_level(open) {
            // `x = 1`, but not where `x` may be bound already, as after `def x = 1`
            if let Some(name) = self.statement_word()
                && !self.named_before(self.found - 1, name)
            {
                return Some(format!(
                    "Frost has no assignment; bind a new name with `def`: `def {name} = ...`"
                ));
            }
            // `f(x) = 1`
            if let Some(name) = self.called_name_starting_statement() {
                return Some(define_function_help(name));
            }
            // `[a, b] = [1, 2]`
            if let Some(pattern) = self.pattern_starting_statement() {
                return Some(destructure_help(pattern));
            }
            return Some(IMMUTABLE_HELP.to_owned());
        }
        let open = open?;
        let help = match open.kind {
            // `if c: { y = 1 }`, meant as a block
            Bracket::MapLiteral
                if self.first_inside(open) == self.found.checked_sub(1)
                    && self.opens_a_branch(open) =>
            {
                BLOCK_HELP
            }
            Bracket::MapLiteral => MAP_ENTRY_HELP,
            // `{a = 1} => ...`, where `{a: 1}` matches a value
            Bracket::MapPattern
                if self.ctx.outer_bracket().map(|outer| outer.kind) == Some(Bracket::MatchArms)
                    && matches!(self.token_at(-2), Some(Token::OpenBrace | Token::Comma)) =>
            {
                MAP_ENTRY_HELP
            }
            // `def {a = 1} = m` or `def [a = 1] = xs`, but not `def [a, b = [1, 2]`,
            // where the pattern's closer is what is missing
            Bracket::MapPattern | Bracket::ArrayPattern if self.closes_in_kind(open.pos) => {
                "Frost patterns have no default values"
            }
            Bracket::MatchArms if self.arm_part() == Some(ArmPart::Result) => IMMUTABLE_HELP,
            Bracket::MatchArms => EQUALITY_HELP,
            Bracket::Call => NAMED_ARGUMENTS_HELP,
            Bracket::Parameters => DEFAULTS_HELP,
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Help for `=>` or `->` where Frost takes neither, `open` being the innermost bracket.
    pub(super) fn arrow_help(&self, open: Option<OpenBracket>) -> Option<String> {
        let arrow = self.token_at(0)?;
        let previous = self.token_at(-1);
        let kind = open.map(|open| open.kind);
        // `fn x, -> x`
        if previous == Some(&Token::Comma) && self.in_bare_parameters() {
            return Some("parameters without parentheses take no trailing comma".to_owned());
        }
        // `match x ( _ => 1 )`, read as a call of `x`; with a `{` after the call,
        // the call is the value to match.
        if let Some(open) = open
            && open.kind == Bracket::Call
            && self.token_before(open.pos.checked_sub(1)?) == Some(&Token::KwMatch)
            && self
                .matching_close(open.pos)
                .and_then(|close| self.ctx.get(close + 1))
                .is_none_or(|t| t.token != Token::OpenBrace)
        {
            return Some("`match` arms go in braces: `match x { ... }`".to_owned());
        }
        let help = match (arrow, previous) {
            // `{a => 1}`, unless the entry may be the next arm after an unclosed Map
            (Token::FatArrow, _) if kind == Some(Bracket::MapLiteral) => {
                let entry = self.found.checked_sub(1)?;
                return (!self.may_be_next_arm(entry)).then(|| MAP_ENTRY_HELP.to_owned());
            }
            // `{ x, y -> x + y }`, a lambda in braces, as Kotlin, Groovy, and Swift write one
            (Token::SlimArrow, _)
                if open.is_some_and(|open| {
                    open.kind == Bracket::MapLiteral && self.only_names_inside(open)
                }) =>
            {
                LAMBDA_HELP
            }
            // `->(x) { x + 1 }`, Ruby's lambda
            (Token::SlimArrow, _)
                if !previous.is_some_and(ends_an_expression)
                    && self.token_at(1) == Some(&Token::OpenParen) =>
            {
                LAMBDA_HELP
            }
            // `f("a" => 1)`, a named argument as Ruby and PHP write one
            (
                Token::FatArrow,
                Some(Token::SingleQuoteStringLiteral(_) | Token::DoubleQuoteStringLiteral(_)),
            ) if kind == Some(Bracket::Call) => NAMED_ARGUMENTS_HELP,
            // `fn x -> y -> x + y`, a curried lambda missing its inner `fn`
            (Token::SlimArrow, Some(Token::Identifier(_) | Token::CloseParen))
                if self.operand_follows_an_arrow() =>
            {
                "each lambda takes its own `fn`: `fn x -> fn y -> ...`"
            }
            // `x => x + 1` or `(x) => x + 1`, where the name or parentheses could be
            // parameters. In a pattern or a Map, the arrow is not a lambda's.
            (_, Some(Token::Identifier(_) | Token::CloseParen))
                if match open {
                    None => self.may_start_a_lambda(arrow),
                    Some(open) => match open.kind {
                        Bracket::Block => self.may_start_a_lambda(arrow),
                        Bracket::Group => !self.opens_a_pattern(open),
                        Bracket::Call | Bracket::ArrayLiteral => true,
                        _ => false,
                    },
                } =>
            {
                LAMBDA_HELP
            }
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Whether the operand just before the error, a name or parentheses, follows `->`,
    /// as `y` does in `fn x -> y -> 1`.
    fn operand_follows_an_arrow(&self) -> bool {
        let Some(last) = self.found.checked_sub(1) else {
            return false;
        };
        let start = match self.ctx.get(last).map(|t| &t.token) {
            Some(Token::CloseParen) => self.matching_open(last),
            _ => Some(last),
        };
        start.is_some_and(|start| self.token_before(start) == Some(&Token::SlimArrow))
    }

    /// Whether `arrow`, found in a statement, may make a lambda of what is before it:
    /// for a `=>`, no lambda's `fn` or `defn` is there already, as in `fn x -> x => 1`;
    /// and it is no name after `match` arms, as `y` is in `match x { 1 => 2 }` then
    /// `y => 3`.
    fn may_start_a_lambda(&self, arrow: &Token) -> bool {
        let start = self.statement_start(self.found);
        let in_a_lambda = *arrow == Token::FatArrow
            && (start..self.found).any(|index| {
                matches!(
                    self.ctx.get(index).map(|t| &t.token),
                    Some(Token::KwFn | Token::KwDefn)
                )
            });
        let arm_after_match = start + 1 == self.found
            && self
                .index_before(start)
                .filter(|&close| self.ctx.get(close).map(|t| &t.token) == Some(&Token::CloseBrace))
                .and_then(|close| self.matching_open(close))
                .is_some_and(|open| self.opened_match_arms(open));
        !in_a_lambda && !arm_after_match
    }

    /// Help for `|` where Frost takes none: `||`, `|>`, `|x| ...`, or a bitwise or.
    pub(super) fn pipe_help(&self) -> Option<String> {
        let help = match self.abutting_next() {
            Some(Token::Pipe) => return self.double_pipe_help(),
            Some(Token::OpGt) => THREAD_HELP,
            // `|x| ...` or `|x, y| ...`, but not in a pattern, which no lambda is
            Some(Token::Identifier(_))
                if matches!(self.token_at(2), Some(Token::Pipe | Token::Comma))
                    && !self.binds_names() =>
            {
                LAMBDA_HELP
            }
            // `1 || 2 => 1`, failing at the second `|`
            _ if self.abutting_previous() == Some(&Token::Pipe) && self.binds_names() => {
                ALTERNATIVES_HELP
            }
            // `xs | f(x)`, a shell's pipe into a call
            _ if self.follows_an_operand(0)
                && self.holds_expressions()
                && self.token_at(1).is_some_and(|callee| {
                    matches!(callee, Token::Identifier(_)) || is_iterative_keyword(callee)
                })
                && self.token_at(2) == Some(&Token::OpenParen) =>
            {
                THREAD_HELP
            }
            // `a | b`, outside a pattern
            _ if self.follows_an_operand(0)
                && self.holds_expressions()
                && self.token_at(1).is_some_and(starts_an_expression) =>
            {
                return Some(format!("{BITWISE_HELP}; its \"or\" is `or`"));
            }
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Help for `||`: another language's "or", or Rust's lambda without parameters.
    fn double_pipe_help(&self) -> Option<String> {
        // `a || b`
        if self.follows_an_operand(0) {
            return Some(OR_HELP.to_owned());
        }
        // `def x = a`, then `|| b` on the next line
        if self.token_at(-1) == Some(&Token::Newline)
            && self
                .token_before(self.found)
                .is_some_and(ends_an_expression)
        {
            return Some(format!("{OR_HELP}; {LINE_CONTINUATION_HELP}"));
        }
        // `|| 1`, with no operand before it
        (!self.binds_names())
            .then(|| "a lambda without parameters is written `fn -> ...`".to_owned())
    }

    /// Whether the innermost bracket holds an expression where `|` could be a bitwise
    /// or: not a pattern, a group starting one, or a list where another language puts
    /// `|`, as Haskell's `[x | x <- xs]` and Elm's `{ r | x = 1 }` do.
    fn holds_expressions(&self) -> bool {
        match self.ctx.innermost_bracket() {
            None => true,
            Some(open) => match open.kind {
                Bracket::Block | Bracket::Call | Bracket::Index | Bracket::AbbreviatedLambda => {
                    true
                }
                Bracket::Group => !self.opens_a_pattern(open),
                _ => false,
            },
        }
    }

    /// Help for `:` where Frost takes none.
    pub(super) fn colon_help(&self) -> Option<String> {
        let previous = self.token_at(-1);
        let help = match self.innermost_kind() {
            // `Foo::bar()`
            _ if self.abutting_next() == Some(&Token::Colon) => PATH_HELP,
            // `obj:method(x)`, Lua's method call
            _ if matches!(self.abutting_previous(), Some(Token::Identifier(_)))
                && matches!(self.abutting_next(), Some(Token::Identifier(_)))
                && self.token_at(2) == Some(&Token::OpenParen) =>
            {
                "Frost values have no methods; call a function with the value first, \
                 like `f(x)` or `x @ f()`"
            }
            // `x := 1`
            _ if self.abutting_next() == Some(&Token::Assign) => {
                return Some(walrus_help(self.statement_word()?));
            }
            Some(Bracket::Parameters) => ANNOTATION_HELP,
            // `xs[:2]`
            Some(Bracket::Index) if previous == Some(&Token::OpenBracket) => SLICE_HELP,
            // `f(a: 1)`
            Some(Bracket::Call)
                if matches!(previous, Some(Token::Identifier(_)))
                    && matches!(
                        self.token_before(self.found - 1),
                        Some(Token::OpenParen | Token::Comma)
                    ) =>
            {
                NAMED_ARGUMENTS_HELP
            }
            // `if x: 1 elif: 2`
            _ if previous == Some(&Token::KwElif) => {
                "`elif` takes a condition; for the last branch, use `else:`"
            }
            _ => return None,
        };
        Some(help.to_owned())
    }

    /// Help for a `$` placeholder, `name`, where Frost does not take one.
    pub(super) fn dollar_help(&self, name: &str) -> Option<String> {
        if name == "$" && self.abutting_next() == Some(&Token::OpenBrace) {
            return Some(if self.ctx.in_interpolation() {
                // `$'${a ${b}}'`
                "`${` interpolates only in a format String's text, not inside another `${...}`"
                    .to_owned()
            } else {
                // `${x}`
                FORMAT_STRING_HELP.to_owned()
            });
        }
        // `$($1 $2)`: inside `$( ... )`, the placeholder is fine, and something else,
        // such as an operator, is missing. An interpolation is lent the lambda's frame
        // but records no brackets opened outside it; and the lambda's `)` is expected
        // after its frame is gone but while its bracket is still open. So either shows
        // that the error is inside `$( ... )`.
        if self.ctx.in_abbreviated_lambda() || self.ctx.is_inside(Bracket::AbbreviatedLambda) {
            return None;
        }
        if name == "$" {
            // `$x = 1`, a name with another language's sigil; `_` is no name to read
            if let Some(Token::Identifier(word)) = self.abutting_next()
                && *word != "_"
            {
                return Some(format!("Frost names have no `$` sigil; write `{word}`"));
            }
            // `match v { $ => 1 }`
            if self.innermost_kind() == Some(Bracket::MatchArms) && self.at_a_pattern_start() {
                return Some(CATCH_ALL_HELP.to_owned());
            }
        }
        // `print $ x`, a `$` between a function and its argument
        if name == "$"
            && self.follows_an_operand(0)
            && self.abutting_previous().is_none()
            && self.abutting_next().is_none()
        {
            return Some(format!("Frost has no `$` operator; {CALL_HELP}"));
        }
        Some("`$` placeholders work only inside `$( ... )`, like `$($ * 2)`".to_owned())
    }

    /// Help for `...` where Frost takes no spread or rest binding.
    pub(super) fn spread_help(&self) -> Option<String> {
        let help = match self.innermost_kind() {
            // `[a ...rest]` or `fn a ...rest -> a`
            Some(Bracket::ArrayPattern | Bracket::Parameters) if self.follows_an_operand(0) => {
                REST_AFTER_COMMA_HELP
            }
            None if self.follows_an_operand(0) && self.token_at(-2) == Some(&Token::KwFn) => {
                REST_AFTER_COMMA_HELP
            }
            Some(Bracket::MapPattern) => MAP_REST_HELP,
            // `match x { ...a => 1 }`
            Some(Bracket::MatchArms) if self.at_a_pattern_start() => ARRAY_REST_HELP,
            // `f(a, ...)` or `def x = ...`, a placeholder spreading nothing
            _ if !self.token_at(1).is_some_and(starts_an_expression) => return None,
            Some(Bracket::Call) => CALL_SPREAD_HELP,
            _ => SPREAD_HELP,
        };
        Some(help.to_owned())
    }

    /// Help for a String literal written as another language writes one: with a prefix
    /// Frost lacks, as in `r'...'`, or beside another String, as in `'a' 'b'`.
    pub(super) fn string_help(&self) -> Option<String> {
        if let Some(Token::Identifier(prefix)) = self.abutting_previous() {
            let found = self.at(0)?;
            let quote = self
                .ctx
                .source_text(found.span.clone().into())
                .chars()
                .next()?;
            return match *prefix {
                "f" | "F" => Some(FORMAT_STRING_HELP.to_owned()),
                // `R"""(...)"""`
                "r" | "R" if matches!(found.token, Token::MultilineStringLiteral(_)) => {
                    let quotes = quote.to_string().repeat(3);
                    Some(format!(
                        "Frost has no multiline raw String; use a multiline String, \
                         `{quotes}...{quotes}`"
                    ))
                }
                "r" | "R" => Some(raw_string_help(quote)),
                "b" | "B" | "X" => Some(
                    "a Bytes literal is written in hex, like `x'00ff'`; for a String's bytes, \
                     use `to_bytes(s)`"
                        .to_owned(),
                ),
                "u" | "U" => Some("a String needs no `u` prefix".to_owned()),
                _ => None,
            };
        }
        let quote_of = |token: &Token| match token {
            Token::SingleQuoteStringLiteral(_) => Some('\''),
            Token::DoubleQuoteStringLiteral(_) => Some('"'),
            _ => None,
        };
        // `'it''s'`, a quote doubled to escape it
        if let Some(quote) = self.abutting_previous().and_then(quote_of)
            && self.token_at(0).and_then(quote_of) == Some(quote)
        {
            return Some(format!(
                "write a quote inside a String with a backslash: `\\{quote}`"
            ));
        }
        // `print('a' 'b')`, or the Strings on separate lines
        let follows_a_string = self.token_before(self.found).and_then(quote_of).is_some();
        if follows_a_string && !self.in_patterns() {
            // On one line in a list, the Strings may be items missing their comma; across
            // lines, they are Python's String joined over a line break.
            let may_be_items = matches!(
                self.innermost_kind(),
                Some(Bracket::Call | Bracket::ArrayLiteral)
            ) && self.token_at(-1).and_then(quote_of).is_some();
            return Some(
                if may_be_items {
                    "adjacent Strings do not join; join them with `+`, or separate them with `,`"
                } else {
                    "adjacent Strings do not join; join them with `+`"
                }
                .to_owned(),
            );
        }
        None
    }

    /// Help for an operator where no rule above gave any: `++x`, Python's `*args`, or
    /// Haskell's operator section `(+ 1)`.
    pub(super) fn operator_help(&self) -> Option<String> {
        let found = self.token_at(0)?;
        let after_operand = self.follows_an_operand(0);
        // `++x`
        if *found == Token::OpPlus && self.abutting_next() == Some(&Token::OpPlus) && !after_operand
        {
            return Some(rebind_help("++"));
        }
        // `f(*args)` or `fn (*args) -> args`
        if *found == Token::OpTimes
            && !after_operand
            && matches!(self.abutting_next(), Some(Token::Identifier(_)))
        {
            let help = match self.innermost_kind() {
                Some(Bracket::Call) => CALL_SPREAD_HELP,
                Some(Bracket::Parameters | Bracket::ArrayPattern) => REST_HELP,
                None if self.binds_names() => REST_HELP,
                Some(Bracket::ArrayLiteral) => SPREAD_HELP,
                _ => return None,
            };
            return Some(help.to_owned());
        }
        // `(+ 1)` or `(*)`, but not `(+1)`, which writes a sign
        let is_section = is_binary_operator(found)
            && self.token_at(-1) == Some(&Token::OpenParen)
            && matches!(self.innermost_kind(), Some(Bracket::Group | Bracket::Call))
            && (self.token_at(1) == Some(&Token::CloseParen)
                || self.token_at(2) == Some(&Token::CloseParen))
            && !(*found == Token::OpPlus && self.abutting_next().is_some());
        is_section.then(|| {
            // Abbreviated lambdas do not nest.
            if self.ctx.in_abbreviated_lambda() {
                "Frost has no operator sections; use a lambda, like `fn x -> x + 1`".to_owned()
            } else {
                "Frost has no operator sections; use an abbreviated lambda, like `$($ + 1)`"
                    .to_owned()
            }
        })
    }
}

/// Whether `token` is a String literal of any kind.
fn is_string(token: &Token) -> bool {
    matches!(
        token,
        Token::SingleQuoteStringLiteral(_)
            | Token::DoubleQuoteStringLiteral(_)
            | Token::RawStringLiteral(_)
            | Token::MultilineStringLiteral(_)
            | Token::SingleQuoteFormatStringLiteral(_)
            | Token::DoubleQuoteFormatStringLiteral(_)
    )
}
