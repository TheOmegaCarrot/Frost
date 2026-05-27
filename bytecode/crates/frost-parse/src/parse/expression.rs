use crate::ast::{BinOp, Expr, ExprKind, Literal, UnaryOp};
use crate::lex::Token;
use crate::parse::control_flow::parse_if;
use crate::parse::format_string;
use crate::parse::strings;
use crate::parse::structures::{parse_array_literal, parse_map_literal};
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_expression(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    ctx.maybe_skip_nl();
    parse_expr_bp(ctx, 0)
}

// bp is short for "binding power"

fn parse_expr_bp(ctx: &mut ParseCtx, min_bp: u8) -> ParseResult<Expr> {
    let mut lhs = parse_prefix(ctx)?;

    loop {
        // Postfix operators bind tightest and must appear on the same line
        // (unless we're inside delimiters where newlines are insignificant).
        // Check them BEFORE skipping newlines.
        if let Some(peek) = ctx.peek() {
            match peek.token {
                Token::OpenParen if POSTFIX_BP >= min_bp => {
                    lhs = parse_call(ctx, lhs)?;
                    continue;
                }
                Token::OpenBracket if POSTFIX_BP >= min_bp => {
                    lhs = parse_index(ctx, lhs)?;
                    continue;
                }
                Token::OpDot if POSTFIX_BP >= min_bp => {
                    lhs = parse_dot_access(ctx, lhs)?;
                    continue;
                }
                Token::OpThread if POSTFIX_BP >= min_bp => {
                    lhs = parse_thread(ctx, lhs)?;
                    continue;
                }
                _ => {}
            }
        }

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

const POSTFIX_BP: u8 = 16;

fn parse_call(ctx: &mut ParseCtx, callee: Expr) -> ParseResult<Expr> {
    let start = callee.span.start;
    ctx.expect(Token::OpenParen)?;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut args = Vec::new();

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseParen)) {
        loop {
            ctx.maybe_skip_nl();
            args.push(parse_expr_bp(ctx, 0)?);
            ctx.maybe_skip_nl();

            let peek = ctx.must_peek("function call arguments")?;
            match peek.token {
                Token::Comma => {
                    ctx.expect(Token::Comma)?;
                }
                Token::CloseParen => break,
                _ => return Err(ctx.unexpected_token(peek, "function call arguments")),
            }

            ctx.maybe_skip_nl();
            if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseParen)) {
                break;
            }
        }
    }

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseParen)?;

    Ok(Expr {
        span: (start..close.span.end).into(),
        kind: ExprKind::Call {
            callee: Box::new(callee),
            args,
        },
    })
}

fn parse_index(ctx: &mut ParseCtx, target: Expr) -> ParseResult<Expr> {
    let start = target.span.start;
    ctx.expect(Token::OpenBracket)?;
    ctx.enter_nl_context().maybe_skip_nl();

    let key = parse_expr_bp(ctx, 0)?;

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseBracket)?;

    Ok(Expr {
        span: (start..close.span.end).into(),
        kind: ExprKind::Index {
            target: Box::new(target),
            key: Box::new(key),
        },
    })
}

fn parse_dot_access(ctx: &mut ParseCtx, target: Expr) -> ParseResult<Expr> {
    let start = target.span.start;
    ctx.expect(Token::OpDot)?;

    let field = ctx.must_peek("dot access")?;
    let field_span = field.span.clone();
    match field.token {
        Token::Identifier(name) => {
            let name = name.to_owned();
            ctx.advance(1);
            Ok(Expr {
                span: (start..field_span.end).into(),
                kind: ExprKind::Index {
                    target: Box::new(target),
                    key: Box::new(Expr {
                        span: field_span.into(),
                        kind: ExprKind::Literal(Literal::String(name.into_bytes())),
                    }),
                },
            })
        }
        _ => Err(ctx.unexpected_token(field, "dot access (expected identifier)")),
    }
}

fn parse_thread(ctx: &mut ParseCtx, lhs: Expr) -> ParseResult<Expr> {
    let start = lhs.span.start;
    ctx.expect(Token::OpThread)?;
    ctx.maybe_skip_nl();

    // Parse callee — restricted to atoms + dot/index postfix only
    let mut callee = parse_atom(ctx)?;

    while let Some(peek) = ctx.peek() {
        match peek.token {
            Token::OpDot => callee = parse_dot_access(ctx, callee)?,
            Token::OpenBracket => callee = parse_index(ctx, callee)?,
            _ => break,
        }
    }

    ctx.expect(Token::OpenParen)?;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut args = vec![lhs];

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseParen)) {
        loop {
            ctx.maybe_skip_nl();
            args.push(parse_expr_bp(ctx, 0)?);
            ctx.maybe_skip_nl();

            let peek = ctx.must_peek("threaded call arguments")?;
            match peek.token {
                Token::Comma => {
                    ctx.expect(Token::Comma)?;
                }
                Token::CloseParen => break,
                _ => return Err(ctx.unexpected_token(peek, "threaded call arguments")),
            }

            ctx.maybe_skip_nl();
            if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseParen)) {
                break;
            }
        }
    }

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseParen)?;

    Ok(Expr {
        span: (start..close.span.end).into(),
        kind: ExprKind::Call {
            callee: Box::new(callee),
            args,
        },
    })
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

        // -- Atoms: strings --
        Token::SingleQuoteStringLiteral(_) => {
            strings::parse_simple_string(ctx, strings::QuoteStyle::Single)
        }
        Token::DoubleQuoteStringLiteral(_) => {
            strings::parse_simple_string(ctx, strings::QuoteStyle::Double)
        }
        Token::RawStringLiteral(_) => strings::parse_raw_string(ctx),
        Token::MultilineStringLiteral(_) => strings::parse_multiline_string(ctx),
        Token::SingleQuoteFormatStringLiteral(_) => {
            format_string::parse_format_string(ctx, strings::QuoteStyle::Single)
        }
        Token::DoubleQuoteFormatStringLiteral(_) => {
            format_string::parse_format_string(ctx, strings::QuoteStyle::Double)
        }

        // -- Atoms: composite literals --
        Token::OpenBracket => parse_array_literal(ctx),
        Token::OpenBrace => parse_map_literal(ctx),

        // -- Atoms: control flow --
        Token::KwIf => parse_if(ctx),
        Token::KwDo => todo!("do block"),
        Token::KwMatch => todo!("match expression"),

        // -- Atoms: functions --
        Token::KwFn => todo!("lambda"),
        Token::DollarParen => todo!("abbreviated lambda"),

        // -- Atoms: iterative expressions --
        Token::KwMap => todo!("map expression"),
        Token::KwFilter => todo!("filter expression"),
        Token::KwReduce => todo!("reduce expression"),
        Token::KwForeach => todo!("foreach expression"),

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
