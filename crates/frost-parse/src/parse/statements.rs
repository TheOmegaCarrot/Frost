use crate::ast::{Binding, Destructure, Expr, SourceSpan, Spanned, Statement};
use crate::lex::Token;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

/// Statements are only allowed in a few contexts,
/// and the rules differ between contexts.
pub(crate) enum StatementContext {
    TopLevel,
    Scope,
}

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_statements(
        &mut self,
        kind: StatementContext,
    ) -> ParseResult<Vec<Spanned<Statement>>> {
        // A block's newlines end its statements, even when the block sits inside
        // delimiters that make newlines insignificant around it.
        self.with_significant_newlines(|ctx| ctx.parse_statement_list(kind))
    }

    fn parse_statement_list(
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
                        let statement = stmts.last().expect("a statement was just parsed").span;
                        // A label across several lines renders as a cluttered bracket,
                        // so a multiline statement gets its last token labeled instead.
                        let (span, label) = if self.source_text(statement).contains('\n') {
                            let last = self
                                .get(self.here() - 1)
                                .expect("the statement's last token");
                            (last.span.clone().into(), "a complete statement ends here")
                        } else {
                            (statement, "this is a complete statement")
                        };
                        return Err(self
                            .expected("a line break or `;`", peek)
                            .with_label(span, label));
                    }
                };
            }
        }

        Ok(stmts)
    }

    fn parse_def(&mut self, exported: bool) -> ParseResult<Spanned<Statement>> {
        let start = self.must_peek("`def`")?.span.start;

        if exported {
            self.expect(Token::KwExport)?;
        }

        self.expect(Token::KwDef)?;

        let destructure = self.parse_destructure()?;

        if let Destructure::Binding(binding) = &destructure.node
            && let Binding::Named(name) = &binding.node
            && let Some(open) = self.peek()
            && open.token == Token::OpenParen
        {
            let defn = if exported { "export defn" } else { "defn" };
            return Err(Diagnostic::at(
                "`def` takes no parameters",
                open.span.clone().into(),
                format!("to define a function, write `{defn} {name}(...) -> ...`"),
            ));
        }

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
        let defn_start = self.must_peek("`defn`")?.span.start;

        if exported {
            self.expect(Token::KwExport)?;
        }

        self.expect(Token::KwDefn)?;

        // Not yet checked to be a name, but parse_binding below will error if this isn't the case.
        const EXPECTED: &str = "a function name";
        let name_span: SourceSpan = self.must_peek(EXPECTED)?.span.clone().into();

        let name = match self.parse_binding(EXPECTED)?.node {
            Binding::Named(name) => name,
            Binding::Discarded => {
                return Err(Diagnostic::at(
                    "`defn` requires a function name, not `_`",
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
