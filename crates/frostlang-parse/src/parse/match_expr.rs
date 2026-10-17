use crate::ast::{Expr, MatchArm, Pattern, Spanned};
use crate::lex::Token;
use crate::parse::ParseResult;
use crate::parse::ctx::{Bracket, ParseCtx};

impl<'src> ParseCtx<'src> {
    #[inline(never)] // See `parse_atom`.
    pub(crate) fn parse_match(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::KwMatch)?.span.start;

        let target = self.parse_expression()?;

        let (arms, span) = self.parse_comma_separated(Bracket::MatchArms, Self::parse_arm)?;

        Ok(Spanned::new(
            Expr::Match {
                target: Box::new(target),
                arms,
            },
            (start..span.end).into(),
        ))
    }

    fn parse_arm(&mut self) -> ParseResult<Spanned<MatchArm>> {
        // The pattern and guard are parsed apart, so that this frame, on the path through
        // nested arm results, holds none of their locals.
        let (pattern, guard) = self.parse_arm_head()?;
        let result = self.parse_expression()?;

        let span = (pattern.span.start..result.span.end).into();
        Ok(Spanned::new(
            MatchArm {
                pattern,
                guard,
                result,
            },
            span,
        ))
    }

    /// An arm's pattern and guard, through its `=>`.
    fn parse_arm_head(&mut self) -> ParseResult<(Spanned<Pattern>, Option<Spanned<Expr>>)> {
        let pattern = self.parse_pattern()?;

        self.maybe_skip_nl();
        let guard = if matches!(self.peek().map(|t| &t.token), Some(Token::KwIf)) {
            self.advance(1);
            self.expect_colon_on_the_line()?;
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.maybe_skip_nl();
        self.expect(Token::FatArrow)?;
        self.maybe_skip_nl();
        Ok((pattern, guard))
    }
}
