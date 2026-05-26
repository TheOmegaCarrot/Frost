use crate::ast::{BinOp, Expr, ExprKind, Literal, UnaryOp};
use crate::lex::Token;
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_expression(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    parse_expr_bp(ctx, 0)
}

// bp is short for "binding power"

fn parse_expr_bp(ctx: &mut ParseCtx, min_bp: u8) -> ParseResult<Expr> {
    let mut lhs = parse_prefix(ctx)?;

    loop {
        ctx.maybe_skip_nl();

        let Some(peek) = ctx.peek() else { break };

        let Some((op, bp)) = infix_bp(&peek.token) else {
            break;
        };

        if bp < min_bp {
            break;
        }

        let non_chainable = is_non_chainable(&peek.token);
        ctx.advance(1);
        ctx.maybe_skip_nl();

        let rhs = parse_expr_bp(ctx, bp + 1)?;
        let span = (lhs.span.start..rhs.span.end).into();
        lhs = Expr {
            span,
            kind: ExprKind::BinOp {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            },
        };

        if non_chainable
            && let Some(next) = ctx.peek()
            && is_non_chainable(&next.token)
        {
            return Err(
                ctx.unexpected_token(next, "expression (cannot chain comparison operators)")
            );
        }
    }

    Ok(lhs)
}

fn parse_prefix(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let peek = ctx.must_peek("expression")?;

    match peek.token {
        Token::OpMinus => {
            let start = peek.span.start;
            ctx.advance(1);
            ctx.maybe_skip_nl();
            let operand = parse_expr_bp(ctx, PREFIX_BP)?;
            Ok(Expr {
                span: (start..operand.span.end).into(),
                kind: ExprKind::UnaryOp {
                    op: UnaryOp::Negate,
                    operand: Box::new(operand),
                },
            })
        }
        Token::OpNot => {
            let start = peek.span.start;
            ctx.advance(1);
            ctx.maybe_skip_nl();
            let operand = parse_expr_bp(ctx, PREFIX_BP)?;
            Ok(Expr {
                span: (start..operand.span.end).into(),
                kind: ExprKind::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                },
            })
        }
        _ => parse_atom(ctx),
    }
}

fn parse_atom(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let peek = ctx.must_peek("expression")?;

    match peek.token {
        Token::IntLiteral(n) => {
            let span = peek.span.clone();
            ctx.advance(1);
            Ok(Expr {
                span: span.into(),
                kind: ExprKind::Literal(Literal::Int(n)),
            })
        }

        Token::FloatLiteral(n) => {
            let span = peek.span.clone();
            ctx.advance(1);
            Ok(Expr {
                span: span.into(),
                kind: ExprKind::Literal(Literal::Float(n)),
            })
        }

        Token::KwTrue => {
            let span = peek.span.clone();
            ctx.advance(1);
            Ok(Expr {
                span: span.into(),
                kind: ExprKind::Literal(Literal::Bool(true)),
            })
        }

        Token::KwFalse => {
            let span = peek.span.clone();
            ctx.advance(1);
            Ok(Expr {
                span: span.into(),
                kind: ExprKind::Literal(Literal::Bool(false)),
            })
        }

        Token::KwNull => {
            let span = peek.span.clone();
            ctx.advance(1);
            Ok(Expr {
                span: span.into(),
                kind: ExprKind::Literal(Literal::Null),
            })
        }

        Token::Identifier(name) => {
            let span = peek.span.clone();
            let name = name.to_owned();
            ctx.advance(1);
            Ok(Expr {
                span: span.into(),
                kind: ExprKind::NameLookup(name),
            })
        }

        Token::OpenParen => {
            let start = peek.span.start;

            ctx.advance(1).enter_nl_context().maybe_skip_nl();

            let expr = parse_expr_bp(ctx, 0)?;

            ctx.maybe_skip_nl().exit_nl_context();

            let close = ctx.expect(Token::CloseParen)?;

            // Parenthesized expression inherits the inner expression's kind,
            // but gets the outer span (including parens).
            Ok(Expr {
                span: (start..close.span.end).into(),
                kind: expr.kind,
            })
        }

        // there's a LOT of TODO here...
        _ => Err(ctx.unexpected_token(peek, "expression")),
    }
}

// -- Binding powers --
// Higher number = tighter binding.
// Nothing in Frost is right-associative, so BP is a single number.

const PREFIX_BP: u8 = 14;

fn infix_bp(token: &Token) -> Option<(BinOp, u8)> {
    match token {
        Token::OpOr => Some((BinOp::Or, 2)),
        Token::OpAnd => Some((BinOp::And, 4)),
        Token::OpEq => Some((BinOp::Eq, 6)),
        Token::OpNeq => Some((BinOp::Neq, 6)),
        Token::OpLt => Some((BinOp::Lt, 8)),
        Token::OpLte => Some((BinOp::Lte, 8)),
        Token::OpGt => Some((BinOp::Gt, 8)),
        Token::OpGte => Some((BinOp::Gte, 8)),
        Token::OpPlus => Some((BinOp::Add, 10)),
        Token::OpMinus => Some((BinOp::Sub, 10)),
        Token::OpTimes => Some((BinOp::Mul, 12)),
        Token::OpDiv => Some((BinOp::Div, 12)),
        Token::OpMod => Some((BinOp::Mod, 12)),
        _ => None,
    }
}

fn is_non_chainable(token: &Token) -> bool {
    matches!(
        token,
        Token::OpEq | Token::OpNeq | Token::OpLt | Token::OpLte | Token::OpGt | Token::OpGte
    )
}
