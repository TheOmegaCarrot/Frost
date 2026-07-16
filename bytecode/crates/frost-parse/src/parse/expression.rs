use crate::ast::{BinOp, Expr, Literal, SourceSpan, Spanned, UnaryOp};
use crate::lex::Token;
use crate::parse::control_flow::{parse_do, parse_if};
use crate::parse::format_string;
use crate::parse::iterative;
use crate::parse::lambda::{parse_abbreviated_lambda, parse_lambda};
use crate::parse::match_expr::parse_match;
use crate::parse::strings;
use crate::parse::structures::{parse_array_literal, parse_map_literal};
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_expression(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    ctx.maybe_skip_nl();
    parse_expr_bp(ctx, 0)
}

// bp is short for "binding power"

fn parse_expr_bp(ctx: &mut ParseCtx, min_bp: u8) -> ParseResult<Spanned<Expr>> {
    let mut lhs = parse_prefix(ctx)?;

    loop {
        // `.` and `@` may continue an expression across line breaks (leading-dot
        // / leading-`@` chaining): neither can begin a statement, so a following
        // one is unambiguously a continuation. Absorb the intervening newlines
        // here so the postfix dispatch below treats it as same-line. Call `()`
        // and index `[]` are NOT continued (a `(`/`[` on a new line begins a
        // fresh statement), so newlines are only skipped for `.`/`@`.
        if POSTFIX_BP >= min_bp
            && matches!(
                ctx.peek_past_nl().map(|t| &t.token),
                Some(Token::OpDot | Token::OpThread)
            )
        {
            ctx.skip_nl();
        }

        // Postfix operators bind tightest and sit on the same line as their
        // operand (after any continuation absorbed above).
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
        let op = Spanned::new(op, peek.span.clone().into());
        ctx.advance(1);
        ctx.maybe_skip_nl();

        let rhs = parse_expr_bp(ctx, bp + 1)?;
        let span = (lhs.span.start..rhs.span.end).into();
        lhs = Spanned::new(
            Expr::BinOp {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            },
            span,
        );

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

fn parse_call(ctx: &mut ParseCtx, callee: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
    let start = callee.span.start;
    ctx.expect(Token::OpenParen)?;
    ctx.enter_nl_context();

    let (args, close) =
        ctx.parse_comma_separated(Token::CloseParen, "function call arguments", |ctx| {
            parse_expr_bp(ctx, 0)
        })?;

    Ok(Spanned::new(
        Expr::Call {
            callee: Box::new(callee),
            args,
        },
        (start..close.span.end).into(),
    ))
}

fn parse_index(ctx: &mut ParseCtx, target: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
    let start = target.span.start;
    ctx.expect(Token::OpenBracket)?;
    ctx.enter_nl_context().maybe_skip_nl();

    let key = parse_expr_bp(ctx, 0)?;

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseBracket)?;

    Ok(Spanned::new(
        Expr::SoftIndex {
            target: Box::new(target),
            key: Box::new(key),
        },
        (start..close.span.end).into(),
    ))
}

fn parse_dot_access(ctx: &mut ParseCtx, target: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
    let start = target.span.start;
    ctx.expect(Token::OpDot)?;

    let peek = ctx.must_peek("dot access")?;
    if let Token::Identifier(name) = peek.token {
        let name = name.to_owned();
        let field_span = ctx.next().unwrap().span.clone();
        Ok(Spanned::new(
            Expr::HardIndex {
                target: Box::new(target),
                key: name,
            },
            (start..field_span.end).into(),
        ))
    } else {
        Err(ctx.unexpected_token(peek, "dot access (expected identifier)"))
    }
}

fn parse_thread(ctx: &mut ParseCtx, lhs: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
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

    Ok(Spanned::new(
        Expr::Call {
            callee: Box::new(callee),
            args,
        },
        (start..close.span.end).into(),
    ))
}

fn parse_prefix(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    let peek = ctx.must_peek("expression")?;

    let unary_op = match peek.token {
        Token::OpMinus => Some((Token::OpMinus, UnaryOp::Negate)),
        Token::OpNot => Some((Token::OpNot, UnaryOp::Not)),
        _ => None,
    };

    if let Some((token, op)) = unary_op {
        let op_span: SourceSpan = ctx.expect(token)?.span.clone().into();
        let start = op_span.start;
        ctx.maybe_skip_nl();
        let operand = parse_expr_bp(ctx, PREFIX_BP)?;
        let end = operand.span.end;
        return Ok(Spanned::new(
            Expr::UnaryOp {
                op: Spanned::new(op, op_span),
                operand: Box::new(operand),
            },
            (start..end).into(),
        ));
    }

    parse_atom(ctx)
}

fn parse_atom(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    let peek = ctx.must_peek("expression")?;
    let span = peek.span.clone();

    match peek.token {
        Token::IntLiteral(n) => {
            ctx.advance(1);
            Ok(Spanned::new(Expr::Literal(Literal::Int(n)), span.into()))
        }
        Token::FloatLiteral(n) => {
            ctx.advance(1);
            Ok(Spanned::new(Expr::Literal(Literal::Float(n)), span.into()))
        }
        Token::KwTrue => {
            ctx.advance(1);
            Ok(Spanned::new(Expr::Literal(Literal::Bool(true)), span.into()))
        }
        Token::KwFalse => {
            ctx.advance(1);
            Ok(Spanned::new(
                Expr::Literal(Literal::Bool(false)),
                span.into(),
            ))
        }
        Token::KwNull => {
            ctx.advance(1);
            Ok(Spanned::new(Expr::Literal(Literal::Null), span.into()))
        }
        Token::Identifier(name) => {
            let name = name.to_owned();
            ctx.advance(1);
            Ok(Spanned::new(Expr::NameLookup(name), span.into()))
        }
        Token::DollarIdentifier(name) if ctx.in_abbreviated_lambda() => {
            let name = name.to_owned();
            ctx.advance(1);
            Ok(Spanned::new(Expr::NameLookup(name), span.into()))
        }

        Token::OpenParen => {
            let start = peek.span.start;

            ctx.advance(1).enter_nl_context().maybe_skip_nl();

            let expr = parse_expr_bp(ctx, 0)?;

            ctx.maybe_skip_nl().exit_nl_context();

            let close = ctx.expect(Token::CloseParen)?;

            // Parenthesized expression inherits the inner expression's node,
            // but gets the outer span (including parens).
            Ok(Spanned::new(
                expr.node,
                (start..close.span.end).into(),
            ))
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
        Token::DollarParen => parse_abbreviated_lambda(ctx),

        // -- Atoms: iterative expressions --
        Token::KwMap => iterative::parse_map_iter(ctx),
        Token::KwFilter => iterative::parse_filter(ctx),
        Token::KwForeach => iterative::parse_foreach(ctx),
        Token::KwReduce => iterative::parse_reduce(ctx),

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
        Token::OpLt => Some((BinOp::Lt, 6)),
        Token::OpLte => Some((BinOp::Lte, 6)),
        Token::OpGt => Some((BinOp::Gt, 6)),
        Token::OpGte => Some((BinOp::Gte, 6)),
        Token::OpPlus => Some((BinOp::Add, 8)),
        Token::OpMinus => Some((BinOp::Sub, 8)),
        Token::OpTimes => Some((BinOp::Mul, 10)),
        Token::OpDiv => Some((BinOp::Div, 10)),
        Token::OpMod => Some((BinOp::Mod, 10)),
        _ => None,
    }
}

fn is_non_chainable(token: &Token) -> bool {
    matches!(
        token,
        Token::OpEq | Token::OpNeq | Token::OpLt | Token::OpLte | Token::OpGt | Token::OpGte
    )
}
