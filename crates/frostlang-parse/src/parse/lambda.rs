use crate::ast::{Binding, Expr, SourceSpan, Spanned, Statement};
use crate::lex::Token;
use crate::parse::ctx::{Bracket, ParseCtx};
use crate::parse::statements::StatementContext;
use crate::parse::{Diagnostic, ParseResult};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_lambda(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::KwFn)?.span.start;

        const EXPECTED: &str = "parameters or `->`";
        let peek = self.must_peek(EXPECTED)?;

        let (self_name, params, variadic_param) = match peek.token {
            Token::SlimArrow => (None, Vec::new(), None),

            Token::OpenParen => {
                let (params, variadic) = self.parse_parenthesized_params()?;
                (None, params, variadic)
            }

            Token::DotDotDot => {
                self.advance(1);
                let variadic = self.parse_binding("a name after `...`")?;
                (None, Vec::new(), Some(variadic))
            }

            Token::Identifier(name) => {
                let name = name.to_owned();
                let name_span: SourceSpan = peek.span.clone().into();
                self.advance(1);

                if matches!(self.peek().map(|t| &t.token), Some(Token::OpenParen)) {
                    let (params, variadic) = self.parse_parenthesized_params()?;
                    (Some(Spanned::new(name, name_span)), params, variadic)
                } else {
                    let binding = if name == "_" {
                        Binding::Discarded
                    } else {
                        Binding::Named(name)
                    };
                    let (mut params, variadic) = self.parse_bare_params_tail()?;
                    params.insert(0, Spanned::new(binding, name_span));
                    (None, params, variadic)
                }
            }

            _ => return Err(self.expected(EXPECTED, peek)),
        };

        let (body, return_expr, end) = self.parse_fn_body()?;

        Ok(Spanned::new(
            Expr::Lambda {
                params,
                variadic_param,
                self_name,
                body,
                return_expr: Box::new(return_expr),
            },
            (start..end).into(),
        ))
    }

    /// Parse `(a, b, ...rest)`. Caller has not consumed the `(`.
    pub(crate) fn parse_parenthesized_params(&mut self) -> ParseResult<Params> {
        let (params, _) = self.delimited(Bracket::Parameters, |ctx| {
            let mut params = Vec::new();

            if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseParen)) {
                return Ok((params, None));
            }
            loop {
                ctx.maybe_skip_nl();

                if matches!(ctx.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
                    ctx.advance(1);
                    let variadic = ctx.parse_binding("a name after `...`")?;
                    return Ok((params, Some(variadic)));
                }

                params.push(ctx.parse_binding("a parameter name")?);
                ctx.maybe_skip_nl();

                let peek = ctx.must_peek("`,` or `)`")?;
                match peek.token {
                    Token::Comma => {
                        ctx.advance(1);
                        ctx.maybe_skip_nl();
                        if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseParen)) {
                            return Ok((params, None));
                        }
                    }
                    Token::CloseParen => return Ok((params, None)),
                    _ => return Err(ctx.expected_in_list("`,` or `)`", peek)),
                }
            }
        })?;
        Ok(params)
    }

    /// Parse `-> expr` or `-> { stmts; expr }`.
    /// Returns `(body_stmts, return_expr, end_offset)`.
    pub(crate) fn parse_fn_body(
        &mut self,
    ) -> ParseResult<(Vec<Spanned<Statement>>, Spanned<Expr>, usize)> {
        self.expect(Token::SlimArrow)?;
        // A body may start on the line after its arrow.
        self.skip_nl();

        match brace_disambiguation(self) {
            BraceKind::Block => self.parse_block_body(),
            BraceKind::TryMap => {
                let checkpoint = self.checkpoint();
                match self.parse_expression() {
                    Ok(expr) => {
                        let end = expr.span.end;
                        Ok((Vec::new(), expr, end))
                    }
                    Err(_) => {
                        self.restore(checkpoint);
                        self.parse_block_body()
                    }
                }
            }
            BraceKind::Expression => {
                let expr = self.parse_expression()?;
                let end = expr.span.end;
                Ok((Vec::new(), expr, end))
            }
        }
    }

    fn parse_block_body(&mut self) -> ParseResult<(Vec<Spanned<Statement>>, Spanned<Expr>, usize)> {
        let (mut body, span) = self.block(|ctx| ctx.parse_statements(StatementContext::Scope))?;

        let Some(last) = body.pop() else {
            return Err(Diagnostic::at(
                "lambda block body must contain at least one expression",
                span,
                "empty block",
            ));
        };

        let return_expr = match last.node {
            Statement::Expr(expr) => expr,
            Statement::Def { .. } => {
                return Err(Diagnostic::at(
                    "lambda block body must end with an expression, not a definition",
                    last.span,
                    "definition here",
                ));
            }
        };

        Ok((body, return_expr, span.end))
    }

    /// Parse the tail of a bare param list: `[, param]* [, ...rest]`.
    /// Stops at `->` (which is not consumed).
    fn parse_bare_params_tail(&mut self) -> ParseResult<Params> {
        let mut params = Vec::new();
        let mut variadic = None;

        while matches!(self.peek().map(|t| &t.token), Some(Token::Comma)) {
            self.advance(1);

            if matches!(self.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
                self.advance(1);
                variadic = Some(self.parse_binding("a name after `...`")?);
                break;
            }

            params.push(self.parse_binding("a parameter name")?);
        }

        Ok((params, variadic))
    }

    // -- Abbreviated lambdas: $(expr) --

    pub(crate) fn parse_abbreviated_lambda(&mut self) -> ParseResult<Spanned<Expr>> {
        let ((body, usage), span) = self.delimited(Bracket::AbbreviatedLambda, |ctx| {
            ctx.enter_abbreviated_lambda();
            let body = ctx.parse_expression()?;
            Ok((body, ctx.exit_abbreviated_lambda()))
        })?;

        Ok(Spanned::new(
            Expr::AbbreviatedLambda {
                used_params: usage.used,
                uses_rest: usage.rest,
                body: Box::new(body),
            },
            span,
        ))
    }
}

/// A parsed parameter list: the positional params and an optional `...rest`.
type Params = (Vec<Spanned<Binding>>, Option<Spanned<Binding>>);

enum BraceKind {
    Block,
    TryMap,
    Expression,
}

/// Disambiguate `{` after `->`:
/// - `{ identifier :`, `{ identifier ,`, and `{ identifier }` are map literals (expression);
///   as a block, `{ identifier }` would only mean `identifier`
/// - `{ [` is probably a map: try expression first, backtrack to block on failure
/// - `{ }` is an empty map (the only sensible reading)
/// - `{ <anything else>` is a block body
/// - no `{` is a plain expression
///
/// Newlines are skipped, as they are inside a Map literal.
fn brace_disambiguation(ctx: &ParseCtx) -> BraceKind {
    let Some(peek) = ctx.peek() else {
        return BraceKind::Expression;
    };
    if peek.token != Token::OpenBrace {
        return BraceKind::Expression;
    }

    let (second, third) = match ctx.get_past_nl(ctx.here() + 1) {
        Some((pos, second)) => (Some(second), ctx.get_past_nl(pos + 1)),
        None => (None, None),
    };

    match second.map(|t| &t.token) {
        Some(Token::Identifier(_))
            if matches!(
                third.map(|(_, t)| &t.token),
                Some(Token::Colon | Token::Comma | Token::CloseBrace)
            ) =>
        {
            BraceKind::Expression
        }
        Some(Token::OpenBracket) => BraceKind::TryMap,
        Some(Token::CloseBrace) => BraceKind::Expression,
        _ => BraceKind::Block,
    }
}
