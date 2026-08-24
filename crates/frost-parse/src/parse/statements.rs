use crate::ast::{Binding, Destructure, Expr, SourceSpan, Spanned, Statement};
use crate::lex::Token;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

/// Statements are only allowed in a few contexts,
/// and the rules differ between contexts.
pub enum StatementContext {
    TopLevel,
    Scope,
}

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub fn parse_statements(
        &mut self,
        kind: StatementContext,
    ) -> ParseResult<Vec<Spanned<Statement>>> {
        let mut stmts = Vec::new();

        let allow_export = matches!(kind, StatementContext::TopLevel);
        let in_a_scope = !allow_export;

        while let Some(peek) = self.peek() {
            match peek.token {
                // Newlines between statements should be skipped.
                // Doing it at the top of the loop skips blank/comment lines at the top of a file.
                Token::Newline | Token::Semicolon => {
                    self.advance(1);
                    continue;
                }
                Token::CloseBrace if in_a_scope => break,
                _ => {}
            }

            match peek.token {
                Token::KwExport if allow_export => {
                    let next = self.get(self.here() + 1).map(|t| &t.token);
                    if matches!(next, Some(Token::KwDefn)) {
                        stmts.push(self.parse_defn(true)?);
                    } else {
                        stmts.push(self.parse_def(true)?);
                    }
                }
                Token::KwDef => stmts.push(self.parse_def(false)?),
                Token::KwDefn => stmts.push(self.parse_defn(false)?),
                _ => {
                    let expr = self.parse_expression()?;
                    let span = expr.span;
                    stmts.push(Spanned::new(Statement::Expr(expr), span));
                }
            }

            if let Some(peek) = self.peek() {
                if in_a_scope && peek.token == Token::CloseBrace {
                    break;
                }
                match peek.token {
                    Token::Semicolon | Token::Newline => {
                        self.advance(1);
                        self.maybe_skip_nl();
                    }
                    _ => {
                        return Err(self.unexpected_token(
                            peek,
                            "expected line break or semicolon after complete statement",
                        ));
                    }
                };
            }
        }

        Ok(stmts)
    }

    fn parse_def(&mut self, exported: bool) -> ParseResult<Spanned<Statement>> {
        let start = self.must_peek("definition")?.span.start;

        if exported {
            self.expect(Token::KwExport)?;
        }

        self.expect(Token::KwDef)?;

        let destructure = self.parse_destructure()?;

        self.expect(Token::Assign)?;

        let expr = self.parse_expression()?;
        let end = expr.span.end;

        Ok(Spanned::new(
            Statement::Def {
                exported,
                destructure,
                expr,
            },
            (start..end).into(),
        ))
    }

    fn parse_defn(&mut self, exported: bool) -> ParseResult<Spanned<Statement>> {
        let defn_start = self.must_peek("function definition")?.span.start;

        if exported {
            self.expect(Token::KwExport)?;
        }

        self.expect(Token::KwDefn)?;

        // Not yet checked to be a name, but parse_binding below will error if this isn't the case.
        let name_span: SourceSpan = self.must_peek("defn function name")?.span.clone().into();

        let name = match self.parse_binding("function name")?.node {
            Binding::Named(name) => name,
            Binding::Discarded => {
                return Err(Diagnostic::at(
                    "defn requires a function name, not '_'",
                    name_span,
                    "expected a name",
                ));
            }
        };

        let (params, variadic_param) = self.parse_parenthesized_params()?;
        let (body, return_expr, end) = self.parse_fn_body()?;

        let expr = Spanned::new(
            Expr::Lambda {
                params,
                variadic_param,
                self_name: Some(Spanned::new(name.clone(), name_span)),
                body,
                return_expr: Box::new(return_expr),
            },
            (name_span.start..end).into(),
        );

        Ok(Spanned::new(
            Statement::Def {
                exported,
                destructure: Spanned::new(
                    Destructure::Binding(Spanned::new(Binding::Named(name), name_span)),
                    name_span,
                ),
                expr,
            },
            (defn_start..end).into(),
        ))
    }
}
