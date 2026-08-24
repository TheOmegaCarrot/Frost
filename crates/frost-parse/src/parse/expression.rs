use crate::ast::{BinOp, Expr, Literal, LogicalOp, SourceSpan, Spanned, UnaryOp};
use crate::lex::Token;
use crate::parse::strings;
use crate::parse::{ParseResult, ctx::ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub fn parse_expression(&mut self) -> ParseResult<Spanned<Expr>> {
        self.maybe_skip_nl();
        self.parse_expr_bp(0)
    }

    // bp is short for "binding power"

    fn parse_expr_bp(&mut self, min_bp: u8) -> ParseResult<Spanned<Expr>> {
        let mut lhs = self.parse_prefix()?;

        loop {
            // `.` and `@` may continue an expression across line breaks (leading-dot
            // / leading-`@` chaining): neither can begin a statement, so a following
            // one is unambiguously a continuation. Absorb the intervening newlines
            // here so the postfix dispatch below treats it as same-line. Call `()`
            // and index `[]` are NOT continued (a `(`/`[` on a new line begins a
            // fresh statement), so newlines are only skipped for `.`/`@`.
            if POSTFIX_BP >= min_bp
                && matches!(
                    self.peek_past_nl().map(|t| &t.token),
                    Some(Token::OpDot | Token::OpThread)
                )
            {
                self.skip_nl();
            }

            // Postfix operators bind tightest and sit on the same line as their
            // operand (after any continuation absorbed above).
            if let Some(peek) = self.peek() {
                match peek.token {
                    Token::OpenParen if POSTFIX_BP >= min_bp => {
                        lhs = self.parse_call(lhs)?;
                        continue;
                    }
                    Token::OpenBracket if POSTFIX_BP >= min_bp => {
                        lhs = self.parse_index(lhs)?;
                        continue;
                    }
                    Token::OpDot if POSTFIX_BP >= min_bp => {
                        lhs = self.parse_dot_access(lhs)?;
                        continue;
                    }
                    Token::OpThread if POSTFIX_BP >= min_bp => {
                        lhs = self.parse_thread(lhs)?;
                        continue;
                    }
                    _ => {}
                }
            }

            self.maybe_skip_nl();

            let Some(peek) = self.peek() else { break };

            let Some((op, bp)) = infix_bp(&peek.token) else {
                break;
            };

            if bp < min_bp {
                break;
            }

            let non_chainable = is_non_chainable(&peek.token);
            let op_span: SourceSpan = peek.span.clone().into();
            self.advance(1);
            self.maybe_skip_nl();

            let rhs = self.parse_expr_bp(bp + 1)?;
            let span = (lhs.span.start..rhs.span.end).into();
            lhs = Spanned::new(
                match op {
                    Infix::Bin(op) => Expr::BinOp {
                        left: Box::new(lhs),
                        op: Spanned::new(op, op_span),
                        right: Box::new(rhs),
                    },
                    Infix::Logical(op) => Expr::Logical {
                        left: Box::new(lhs),
                        op: Spanned::new(op, op_span),
                        right: Box::new(rhs),
                    },
                },
                span,
            );

            if non_chainable
                && let Some(next) = self.peek()
                && is_non_chainable(&next.token)
            {
                return Err(
                    self.unexpected_token(next, "expression (cannot chain comparison operators)")
                );
            }
        }

        Ok(lhs)
    }

    fn parse_call(&mut self, callee: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
        let start = callee.span.start;
        self.expect(Token::OpenParen)?;
        self.enter_nl_context();

        let (args, close) =
            self.parse_comma_separated(Token::CloseParen, "function call arguments", |ctx| {
                ctx.parse_expr_bp(0)
            })?;

        Ok(Spanned::new(
            Expr::Call {
                callee: Box::new(callee),
                args,
            },
            (start..close.span.end).into(),
        ))
    }

    fn parse_index(&mut self, target: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
        let start = target.span.start;
        self.expect(Token::OpenBracket)?;
        self.enter_nl_context().maybe_skip_nl();

        let key = self.parse_expr_bp(0)?;

        self.maybe_skip_nl().exit_nl_context();
        let close = self.expect(Token::CloseBracket)?;

        Ok(Spanned::new(
            Expr::SoftIndex {
                target: Box::new(target),
                key: Box::new(key),
            },
            (start..close.span.end).into(),
        ))
    }

    fn parse_dot_access(&mut self, target: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
        let start = target.span.start;
        self.expect(Token::OpDot)?;

        let peek = self.must_peek("dot access")?;
        if let Token::Identifier(name) = peek.token {
            let name = name.to_owned();
            let field_span = self.next().unwrap().span.clone();
            Ok(Spanned::new(
                Expr::HardIndex {
                    target: Box::new(target),
                    key: Spanned::new(name, field_span.clone().into()),
                },
                (start..field_span.end).into(),
            ))
        } else {
            Err(self.unexpected_token(peek, "dot access (expected identifier)"))
        }
    }

    fn parse_thread(&mut self, lhs: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
        let start = lhs.span.start;
        self.expect(Token::OpThread)?;
        self.maybe_skip_nl();

        let mut callee = self.parse_atom()?;

        while let Some(peek) = self.peek() {
            match peek.token {
                Token::OpDot => callee = self.parse_dot_access(callee)?,
                Token::OpenBracket => callee = self.parse_index(callee)?,
                _ => break,
            }
        }

        self.expect(Token::OpenParen)?;
        self.enter_nl_context();

        let (mut args, close) =
            self.parse_comma_separated(Token::CloseParen, "threaded call arguments", |ctx| {
                ctx.parse_expr_bp(0)
            })?;

        args.insert(0, lhs);

        Ok(Spanned::new(
            Expr::Call {
                callee: Box::new(callee),
                args,
            },
            (start..close.span.end).into(),
        ))
    }

    fn parse_prefix(&mut self) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("expression")?;

        let unary_op = match peek.token {
            Token::OpMinus => Some((Token::OpMinus, UnaryOp::Negate)),
            Token::OpNot => Some((Token::OpNot, UnaryOp::Not)),
            _ => None,
        };

        if let Some((token, op)) = unary_op {
            let op_span: SourceSpan = self.expect(token)?.span.clone().into();
            let start = op_span.start;
            self.maybe_skip_nl();
            let operand = self.parse_expr_bp(PREFIX_BP)?;
            let end = operand.span.end;
            return Ok(Spanned::new(
                Expr::UnaryOp {
                    op: Spanned::new(op, op_span),
                    operand: Box::new(operand),
                },
                (start..end).into(),
            ));
        }

        self.parse_atom()
    }

    fn parse_atom(&mut self) -> ParseResult<Spanned<Expr>> {
        let peek = self.must_peek("expression")?;
        let span = peek.span.clone();

        match peek.token {
            Token::IntLiteral(n) => {
                self.advance(1);
                Ok(Spanned::new(Expr::Literal(Literal::Int(n)), span.into()))
            }
            Token::FloatLiteral(n) => {
                self.advance(1);
                Ok(Spanned::new(Expr::Literal(Literal::Float(n)), span.into()))
            }
            Token::KwTrue => {
                self.advance(1);
                Ok(Spanned::new(
                    Expr::Literal(Literal::Bool(true)),
                    span.into(),
                ))
            }
            Token::KwFalse => {
                self.advance(1);
                Ok(Spanned::new(
                    Expr::Literal(Literal::Bool(false)),
                    span.into(),
                ))
            }
            Token::KwNull => {
                self.advance(1);
                Ok(Spanned::new(Expr::Literal(Literal::Null), span.into()))
            }
            Token::Identifier(name) => {
                let name = name.to_owned();
                self.advance(1);
                Ok(Spanned::new(Expr::NameLookup(name), span.into()))
            }
            Token::DollarIdentifier(name) if self.in_abbreviated_lambda() => {
                let name = name.to_owned();
                self.record_dollar(&name);
                self.advance(1);
                Ok(Spanned::new(Expr::NameLookup(name), span.into()))
            }

            Token::OpenParen => {
                let start = peek.span.start;

                self.advance(1).enter_nl_context().maybe_skip_nl();

                let expr = self.parse_expr_bp(0)?;

                self.maybe_skip_nl().exit_nl_context();

                let close = self.expect(Token::CloseParen)?;

                // Parenthesized expression inherits the inner expression's node,
                // but gets the outer span (including parens).
                Ok(Spanned::new(expr.node, (start..close.span.end).into()))
            }

            // -- Atoms: strings --
            Token::SingleQuoteStringLiteral(_) => {
                self.parse_simple_string(strings::QuoteStyle::Single)
            }
            Token::DoubleQuoteStringLiteral(_) => {
                self.parse_simple_string(strings::QuoteStyle::Double)
            }
            Token::RawStringLiteral(_) => self.parse_raw_string(),
            Token::MultilineStringLiteral(_) => self.parse_multiline_string(),
            Token::SingleQuoteFormatStringLiteral(_) => {
                self.parse_format_string(strings::QuoteStyle::Single)
            }
            Token::DoubleQuoteFormatStringLiteral(_) => {
                self.parse_format_string(strings::QuoteStyle::Double)
            }
            Token::BytesLiteral(_) => self.parse_bytes_literal(),

            // -- Atoms: composite literals --
            Token::OpenBracket => self.parse_array_literal(),
            Token::OpenBrace => self.parse_map_literal(),

            // -- Atoms: control flow --
            Token::KwIf => self.parse_if(),
            Token::KwDo => self.parse_do(),
            Token::KwMatch => self.parse_match(),

            // -- Atoms: functions --
            Token::KwFn => self.parse_lambda(),
            Token::DollarParen => self.parse_abbreviated_lambda(),

            // -- Atoms: iterative expressions --
            Token::KwMap => self.parse_map_iter(),
            Token::KwFilter => self.parse_filter(),
            Token::KwForeach => self.parse_foreach(),
            Token::KwReduce => self.parse_reduce(),

            _ => Err(self.unexpected_token(peek, "expression")),
        }
    }
}

// -- Binding powers --
// Higher number = tighter binding.
// Nothing in Frost is right-associative, so BP is a single number.

const POSTFIX_BP: u8 = 16;
const PREFIX_BP: u8 = 14;

/// An infix operator: strict ([`Expr::BinOp`]) or short-circuiting
/// ([`Expr::Logical`]). The Pratt loop treats both identically for
/// precedence; only node construction differs.
enum Infix {
    Bin(BinOp),
    Logical(LogicalOp),
}

fn infix_bp(token: &Token) -> Option<(Infix, u8)> {
    match token {
        Token::OpOr => Some((Infix::Logical(LogicalOp::Or), 2)),
        Token::OpAnd => Some((Infix::Logical(LogicalOp::And), 4)),
        Token::OpEq => Some((Infix::Bin(BinOp::Eq), 6)),
        Token::OpNeq => Some((Infix::Bin(BinOp::Neq), 6)),
        Token::OpLt => Some((Infix::Bin(BinOp::Lt), 6)),
        Token::OpLte => Some((Infix::Bin(BinOp::Lte), 6)),
        Token::OpGt => Some((Infix::Bin(BinOp::Gt), 6)),
        Token::OpGte => Some((Infix::Bin(BinOp::Gte), 6)),
        Token::OpPlus => Some((Infix::Bin(BinOp::Add), 8)),
        Token::OpMinus => Some((Infix::Bin(BinOp::Sub), 8)),
        Token::OpTimes => Some((Infix::Bin(BinOp::Mul), 10)),
        Token::OpDiv => Some((Infix::Bin(BinOp::Div), 10)),
        Token::OpMod => Some((Infix::Bin(BinOp::Mod), 10)),
        _ => None,
    }
}

fn is_non_chainable(token: &Token) -> bool {
    matches!(
        token,
        Token::OpEq | Token::OpNeq | Token::OpLt | Token::OpLte | Token::OpGt | Token::OpGte
    )
}
