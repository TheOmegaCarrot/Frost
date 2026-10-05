use crate::ast::{Expr, SourceSpan, Spanned};
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
        let keyword_span: SourceSpan = self.expect(keyword)?.span.clone().into();

        let structure = self.parse_expression()?;
        self.skip_nl();
        self.expect_with(keyword_span, structure.span.end)?;
        self.skip_nl();
        let operation = self.parse_expression()?;

        let span = keyword_span.start..operation.span.end;
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

        Ok(Spanned::new(node, span.into()))
    }

    /// Expect the `with` of the expression that `keyword` starts, whose operand
    /// ends at `operand_end`.
    /// When what is found instead is on a later line than the operand's end, the
    /// keyword is labeled too, since the line the error points at may hold nothing wrong.
    fn expect_with(&mut self, keyword: SourceSpan, operand_end: usize) -> ParseResult<()> {
        let on_a_later_line = self
            .next_start()
            .is_some_and(|found| self.source_text((operand_end..found).into()).contains('\n'));
        let keyword_label =
            on_a_later_line.then(|| format!("this `{}` needs `with`", self.source_text(keyword)));
        self.expect(Token::KwWith)
            .map_err(|diagnostic| match keyword_label {
                Some(label) => diagnostic.with_label(keyword, label),
                None => diagnostic,
            })?;
        Ok(())
    }

    pub(crate) fn parse_map_iter(&mut self) -> ParseResult<Spanned<Expr>> {
        self.parse_iterative(Token::KwMap, IterativeKind::Map)
    }

    pub(crate) fn parse_filter(&mut self) -> ParseResult<Spanned<Expr>> {
        self.parse_iterative(Token::KwFilter, IterativeKind::Filter)
    }

    pub(crate) fn parse_foreach(&mut self) -> ParseResult<Spanned<Expr>> {
        self.parse_iterative(Token::KwForeach, IterativeKind::Foreach)
    }

    pub(crate) fn parse_reduce(&mut self) -> ParseResult<Spanned<Expr>> {
        let keyword_span: SourceSpan = self.expect(Token::KwReduce)?.span.clone().into();

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

        let operand_end = init
            .as_ref()
            .map_or(structure.span.end, |init| init.span.end);
        self.expect_with(keyword_span, operand_end)?;
        self.skip_nl();
        let operation = self.parse_expression()?;
        let span = keyword_span.start..operation.span.end;

        Ok(Spanned::new(
            Expr::Reduce {
                structure: Box::new(structure),
                operation: Box::new(operation),
                init,
            },
            span.into(),
        ))
    }
}
