use crate::ast::{Binding, Expr, Pattern, SourceSpan, Spanned, Statement};
use crate::lex::Token;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

/// Statements are only allowed in a few contexts,
/// and the rules differ between contexts.
pub(crate) enum StatementContext {
    TopLevel,
    Scope,
}

impl<'src> ParseCtx<'src> {
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

            // A definition is parsed apart, so that only an expression statement's
            // result takes room in this frame, on the path through nested blocks.
            let statement = match peek.token {
                Token::KwExport if allow_export => self.parse_definition()?,
                Token::KwDef | Token::KwDefn => self.parse_definition()?,
                _ => {
                    let expr = self.parse_expression()?;
                    let span = expr.span;
                    Spanned::new(Statement::Expr(expr), span)
                }
            };
            let statement_span = statement.span;
            stmts.push(statement);

            if let Some(peek) = self.peek() {
                if in_a_scope && peek.token == Token::CloseBrace {
                    break;
                }
                match peek.token {
                    Token::Semicolon | Token::Newline => {
                        self.advance(1);
                        self.maybe_skip_nl();
                    }
                    _ => return Err(self.statement_not_ended(statement_span)),
                };
            }
        }

        Ok(stmts)
    }

    /// A `def`, `defn`, `export def`, or `export defn` statement.
    fn parse_definition(&mut self) -> ParseResult<Spanned<Statement>> {
        let exported = self.peek().is_some_and(|t| t.token == Token::KwExport);
        let keyword = self.get(self.here() + usize::from(exported));
        if keyword.is_some_and(|t| t.token == Token::KwDefn) {
            self.parse_defn(exported)
        } else {
            self.parse_def(exported)
        }
    }

    /// The error for the token after `statement`, which is neither a line break nor `;`.
    fn statement_not_ended(&self, statement: SourceSpan) -> Diagnostic {
        let peek = self.peek().expect("a token follows the statement");
        // A token touching the statement's last name or literal, as `b101` touches the
        // `0` of `0b101`, makes one malformed token with it, not a statement after a
        // complete one. After a closer, as in `print("a")print("b")`, it is the next
        // statement.
        let last_closes = self.get(self.here() - 1).is_some_and(|last| {
            matches!(
                last.token,
                Token::CloseParen | Token::CloseBracket | Token::CloseBrace
            )
        });
        if statement.end == peek.span.start && !last_closes {
            return self.expected("a line break or `;`", peek);
        }
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
        self.expected("a line break or `;`", peek)
            .with_label(span, label)
    }

    fn parse_def(&mut self, exported: bool) -> ParseResult<Spanned<Statement>> {
        let start = self.must_peek("`def`")?.span.start;

        if exported {
            self.expect(Token::KwExport)?;
        }

        self.expect(Token::KwDef)?;

        let pattern = self.parse_pattern()?;

        if let Pattern::Binding {
            name: binding,
            type_constraint: None,
        } = &pattern.node
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
                pattern,
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
                pattern: Spanned::new(
                    Pattern::Binding {
                        name: Spanned::new(Binding::Named(name), name_span),
                        type_constraint: None,
                    },
                    name_span,
                ),
                expr,
            },
            (defn_start..end).into(),
        ))
    }
}
