use crate::ast::{Expr, Spanned};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx};

enum IterativeKind {
    Map,
    Filter,
    Foreach,
}

fn parse_iterative(
    ctx: &mut ParseCtx,
    keyword: Token,
    kind: IterativeKind,
) -> ParseResult<Spanned<Expr>> {
    let start = ctx.expect(keyword)?.span.start;

    let structure = parse_expression(ctx)?;
    ctx.skip_nl();
    ctx.expect(Token::KwWith)?;
    ctx.skip_nl();
    let operation = parse_expression(ctx)?;

    let end = operation.span.end;
    let node = match kind {
        IterativeKind::Map => Expr::MapIter {
            structure: Box::new(structure),
            operation: Box::new(operation),
        },
        IterativeKind::Filter => Expr::Filter {
            structure: Box::new(structure),
            operation: Box::new(operation),
        },
        IterativeKind::Foreach => Expr::Foreach {
            structure: Box::new(structure),
            operation: Box::new(operation),
        },
    };

    Ok(Spanned::new(node, (start..end).into()))
}

pub fn parse_map_iter(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    parse_iterative(ctx, Token::KwMap, IterativeKind::Map)
}

pub fn parse_filter(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    parse_iterative(ctx, Token::KwFilter, IterativeKind::Filter)
}

pub fn parse_foreach(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    parse_iterative(ctx, Token::KwForeach, IterativeKind::Foreach)
}

pub fn parse_reduce(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    let start = ctx.expect(Token::KwReduce)?.span.start;

    let structure = parse_expression(ctx)?;
    ctx.skip_nl();

    let init = if matches!(ctx.peek().map(|t| &t.token), Some(Token::KwInit)) {
        ctx.advance(1);
        ctx.expect(Token::Colon)?;
        ctx.skip_nl();
        let expr = parse_expression(ctx)?;
        ctx.skip_nl();
        Some(Box::new(expr))
    } else {
        None
    };

    ctx.expect(Token::KwWith)?;
    ctx.skip_nl();
    let operation = parse_expression(ctx)?;
    let end = operation.span.end;

    Ok(Spanned::new(
        Expr::Reduce {
            structure: Box::new(structure),
            operation: Box::new(operation),
            init,
        },
        (start..end).into(),
    ))
}
