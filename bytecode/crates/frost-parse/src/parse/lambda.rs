use crate::ast::{Binding, Expr, SourceSpan, Spanned, Statement};
use crate::lex::Token;
use crate::parse::statements::StatementContext;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub fn parse_lambda(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::KwFn)?.span.start;

        let peek = self.must_peek("lambda")?;

        let (self_name, params, variadic_param) = match peek.token {
            Token::SlimArrow => (None, Vec::new(), None),

            Token::OpenParen => {
                let (params, variadic) = self.parse_parenthesized_params()?;
                (None, params, variadic)
            }

            Token::DotDotDot => {
                self.advance(1);
                let variadic = self.parse_binding("parameter name")?;
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

            _ => return Err(self.unexpected_token(peek, "lambda")),
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
    /// Reusable for `defn`.
    pub fn parse_parenthesized_params(&mut self) -> ParseResult<Params> {
        self.expect(Token::OpenParen)?;
        self.enter_nl_context().maybe_skip_nl();

        let mut params = Vec::new();
        let mut variadic = None;

        if !matches!(self.peek().map(|t| &t.token), Some(Token::CloseParen)) {
            loop {
                self.maybe_skip_nl();

                if matches!(self.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
                    self.advance(1);
                    variadic = Some(self.parse_binding("parameter name")?);
                    self.maybe_skip_nl();
                    break;
                }

                params.push(self.parse_binding("parameter name")?);
                self.maybe_skip_nl();

                let peek = self.must_peek("function parameters")?;
                match peek.token {
                    Token::Comma => {
                        self.advance(1);
                        self.maybe_skip_nl();
                        if matches!(self.peek().map(|t| &t.token), Some(Token::CloseParen)) {
                            break;
                        }
                    }
                    Token::CloseParen => break,
                    _ => return Err(self.unexpected_token(peek, "function parameters")),
                }
            }
        }

        self.maybe_skip_nl().exit_nl_context();
        self.expect(Token::CloseParen)?;

        Ok((params, variadic))
    }

    /// Parse `-> expr` or `-> { stmts; expr }`.
    /// Returns `(body_stmts, return_expr, end_offset)`.
    /// Reusable for `defn`.
    pub fn parse_fn_body(
        &mut self,
    ) -> ParseResult<(Vec<Spanned<Statement>>, Spanned<Expr>, usize)> {
        self.expect(Token::SlimArrow)?;
        self.maybe_skip_nl();

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
        let open_start = self.expect(Token::OpenBrace)?.span.start;

        let mut body = self.parse_statements(StatementContext::Scope)?;

        let close_end = self.expect(Token::CloseBrace)?.span.end;

        let Some(last) = body.pop() else {
            return Err(Diagnostic::at(
                "lambda block body must contain at least one expression",
                (open_start..close_end).into(),
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

        Ok((body, return_expr, close_end))
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
                variadic = Some(self.parse_binding("parameter name")?);
                break;
            }

            params.push(self.parse_binding("parameter name")?);
        }

        Ok((params, variadic))
    }

    // -- Abbreviated lambdas: $(expr) --

    pub fn parse_abbreviated_lambda(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::DollarParen)?.span.start;
        self.enter_nl_context()
            .maybe_skip_nl()
            .enter_abbreviated_lambda();

        let body = self.parse_expression()?;

        let usage = self.exit_abbreviated_lambda();
        self.maybe_skip_nl().exit_nl_context();

        let close = self.expect(Token::CloseParen)?;

        Ok(Spanned::new(
            Expr::AbbreviatedLambda {
                used_params: usage.used,
                uses_rest: usage.rest,
                body: Box::new(body),
            },
            (start..close.span.end).into(),
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
/// - `{ identifier :` → map literal (expression)
/// - `{ [` → probably map, try expression first, backtrack to block on failure
/// - `{ }` → empty map (rare in practice, but there's nothing else that makes sense)
/// - `{ <anything else>` → block body
/// - no `{` → plain expression
fn brace_disambiguation(ctx: &ParseCtx) -> BraceKind {
    let Some(peek) = ctx.peek() else {
        return BraceKind::Expression;
    };
    if peek.token != Token::OpenBrace {
        return BraceKind::Expression;
    }

    let pos = ctx.here();
    let second = ctx.get(pos + 1);
    let third = ctx.get(pos + 2);

    match second.map(|t| &t.token) {
        Some(Token::Identifier(_)) if matches!(third.map(|t| &t.token), Some(Token::Colon)) => {
            BraceKind::Expression
        }
        Some(Token::OpenBracket) => BraceKind::TryMap,
        Some(Token::CloseBrace) => BraceKind::Expression,
        _ => BraceKind::Block,
    }
}
