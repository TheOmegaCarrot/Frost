use crate::ast::{BinOp, Expr, ExprKind, Literal, UnaryOp};
use crate::lex::Token;
use crate::parse::control_flow::{parse_do, parse_if};
use crate::parse::format_string;
use crate::parse::lambda::parse_lambda;
use crate::parse::match_expr::parse_match;
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
    ctx.enter_nl_context();

    let (args, close) =
        ctx.parse_comma_separated(Token::CloseParen, "function call arguments", |ctx| {
            parse_expr_bp(ctx, 0)
        })?;

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

    let peek = ctx.must_peek("dot access")?;
    if let Token::Identifier(name) = peek.token {
        let name = name.to_owned();
        let field_span = ctx.next().unwrap().span.clone();
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
    } else {
        Err(ctx.unexpected_token(peek, "dot access (expected identifier)"))
    }
}

fn parse_thread(ctx: &mut ParseCtx, lhs: Expr) -> ParseResult<Expr> {
    let start = lhs.span.start;
    ctx.expect(Token::OpThread)?;
    ctx.maybe_skip_nl();

    let mut callee = parse_atom(ctx)?;

    while let Some(peek) = ctx.peek() {
        match peek.token {
            Token::OpDot => callee = parse_dot_access(ctx, callee)?,
            Token::OpenBracket => callee = parse_index(ctx, callee)?,
            _ => break,
        }
    }

    ctx.expect(Token::OpenParen)?;
    ctx.enter_nl_context();

    let (mut args, close) =
        ctx.parse_comma_separated(Token::CloseParen, "threaded call arguments", |ctx| {
            parse_expr_bp(ctx, 0)
        })?;

    args.insert(0, lhs);

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

    let unary_op = match peek.token {
        Token::OpMinus => Some((Token::OpMinus, UnaryOp::Negate)),
        Token::OpNot => Some((Token::OpNot, UnaryOp::Not)),
        _ => None,
    };

    if let Some((token, op)) = unary_op {
        let start = ctx.expect(token)?.span.start;
        ctx.maybe_skip_nl();
        let operand = parse_expr_bp(ctx, PREFIX_BP)?;
        return Ok(Expr {
            span: (start..operand.span.end).into(),
            kind: ExprKind::UnaryOp {
                op,
                operand: Box::new(operand),
            },
        });
    }

    parse_atom(ctx)
}

fn parse_atom(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let peek = ctx.must_peek("expression")?;
    let span = peek.span.clone();

    match peek.token {
        Token::IntLiteral(n) => { ctx.advance(1); Ok(Expr { span: span.into(), kind: ExprKind::Literal(Literal::Int(n)) }) }
        Token::FloatLiteral(n) => { ctx.advance(1); Ok(Expr { span: span.into(), kind: ExprKind::Literal(Literal::Float(n)) }) }
        Token::KwTrue => { ctx.advance(1); Ok(Expr { span: span.into(), kind: ExprKind::Literal(Literal::Bool(true)) }) }
        Token::KwFalse => { ctx.advance(1); Ok(Expr { span: span.into(), kind: ExprKind::Literal(Literal::Bool(false)) }) }
        Token::KwNull => { ctx.advance(1); Ok(Expr { span: span.into(), kind: ExprKind::Literal(Literal::Null) }) }
        Token::Identifier(name) => { let name = name.to_owned(); ctx.advance(1); Ok(Expr { span: span.into(), kind: ExprKind::NameLookup(name) }) }

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
        Token::KwDo => parse_do(ctx),
        Token::KwMatch => parse_match(ctx),

        // -- Atoms: functions --
        Token::KwFn => parse_lambda(ctx),
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
