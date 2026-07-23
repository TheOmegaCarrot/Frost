use crate::ast::{Expr, Spanned};
use crate::lex::Token;
use crate::parse::{ParseResult, ctx::ParseCtx};

enum IterativeKind {
    Map,
    Filter,
    Foreach,
}

impl<'src, 'f> ParseCtx<'src, 'f> {
    fn parse_iterative(
        &mut self,
        keyword: Token,
        kind: IterativeKind,
    ) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(keyword)?.span.start;

        let structure = self.parse_expression()?;
        self.skip_nl();
        self.expect(Token::KwWith)?;
        self.skip_nl();
        let operation = self.parse_expression()?;

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

    pub fn parse_map_iter(&mut self) -> ParseResult<Spanned<Expr>> {
        self.parse_iterative(Token::KwMap, IterativeKind::Map)
    }

    pub fn parse_filter(&mut self) -> ParseResult<Spanned<Expr>> {
        self.parse_iterative(Token::KwFilter, IterativeKind::Filter)
    }

    pub fn parse_foreach(&mut self) -> ParseResult<Spanned<Expr>> {
        self.parse_iterative(Token::KwForeach, IterativeKind::Foreach)
    }

    pub fn parse_reduce(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::KwReduce)?.span.start;

        let structure = self.parse_expression()?;
        self.skip_nl();

        let init = if matches!(self.peek().map(|t| &t.token), Some(Token::KwInit)) {
            self.advance(1);
            self.expect(Token::Colon)?;
            self.skip_nl();
            let expr = self.parse_expression()?;
            self.skip_nl();
            Some(Box::new(expr))
        } else {
            None
        };

        self.expect(Token::KwWith)?;
        self.skip_nl();
        let operation = self.parse_expression()?;
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
}
