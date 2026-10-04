use std::ops::Range;

use crate::ast::SourceSpan;
use crate::lex::Token;
use crate::parse::{Diagnostic, ParseResult};

use logos::Logos;

#[derive(Debug)]
pub(crate) struct ParseCtx<'src, 'f> {
    /// The source this context was lexed from. For a sub-context (a
    /// format-string interpolation) this is only the interpolation substring;
    /// `base_offset` maps its positions back into the whole source.
    full_source: &'src str,

    /// Byte offset of `full_source` within the original source. Zero for the
    /// top-level context; nonzero for interpolation sub-contexts, so that every
    /// diagnostic span is in whole-source coordinates.
    base_offset: usize,

    filename: &'f str,

    /// The full lexer result.
    input: Vec<SrcToken<'src>>,

    state: ParseState,
}

#[derive(Debug)]
pub(crate) struct SrcToken<'src> {
    pub token: Token<'src>,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ParseState {
    /// The current offset into `input`.
    /// Token-level position, not byte offset.
    pub pos: usize,

    /// When > 0, newlines are not significant (we're inside delimiters).
    pub nl_depth: u32,

    /// One frame per enclosing abbreviated lambda; dollar identifiers are
    /// permitted while non-empty and record into the innermost frame.
    /// Lives in the checkpointed state so backtracking discards speculative marks.
    pub abbrev_lambdas: Vec<DollarUsage>,
}

/// Dollar-identifier usage collected for one abbreviated lambda.
/// Filled in as the body parses; consumed by `exit_abbreviated_lambda`.
#[derive(Clone, Debug, Default)]
pub(crate) struct DollarUsage {
    /// Becomes `used_params` of [`Expr::AbbreviatedLambda`](crate::ast::Expr::AbbreviatedLambda).
    pub used: Vec<bool>,
    /// Whether the rest parameter `$$` was referenced.
    pub rest: bool,
}

impl DollarUsage {
    fn mark(&mut self, n: usize) {
        if self.used.len() < n {
            self.used.resize(n, false);
        }
        self.used[n - 1] = true;
    }
}

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn checkpoint(&self) -> ParseState {
        self.state.clone()
    }

    pub(crate) fn restore(&mut self, state: ParseState) {
        self.state = state;
    }

    pub(crate) fn new(filename: &'f str, src: &'src str) -> ParseResult<Self> {
        Self::new_with_offset(filename, src, 0)
    }

    pub(crate) fn new_with_offset(
        filename: &'f str,
        src: &'src str,
        base_offset: usize,
    ) -> ParseResult<Self> {
        let mut lexer = Token::lexer(src);
        let mut input = Vec::new();

        while let Some(token) = lexer.next() {
            let span = lexer.span();
            let shifted = (span.start + base_offset)..(span.end + base_offset);

            let Ok(token) = token else {
                return Err(lex_error(shifted));
            };

            input.push(SrcToken {
                token,
                span: shifted,
            });
        }

        Ok(Self {
            full_source: src,
            base_offset,
            filename,
            input,
            state: ParseState::default(),
        })
    }

    /// Peek the next token, if in-bounds, without advancing state.
    pub(crate) fn peek(&self) -> Option<&SrcToken<'src>> {
        self.input.get(self.state.pos)
    }

    /// Peek the next token, or fail as [`unexpected_eof`](Self::unexpected_eof) does.
    pub(crate) fn must_peek(&self, expected: &str) -> ParseResult<&SrcToken<'src>> {
        self.peek().ok_or_else(|| self.unexpected_eof(expected))
    }

    /// Get the current token, and advance the state.
    pub(crate) fn next(&mut self) -> Option<&SrcToken<'src>> {
        self.advance(1);
        self.input.get(self.state.pos - 1)
    }

    /// Advance the state by n tokens.
    pub(crate) fn advance(&mut self, n: usize) -> &mut Self {
        self.state.pos += n;
        self
    }

    pub(crate) fn expect(&mut self, token: Token) -> ParseResult<&SrcToken<'src>> {
        let expected = format!("`{token}`");
        let current = self.must_peek(&expected)?;
        if current.token != token {
            return Err(self.expected(&expected, current));
        }

        self.advance(1);
        Ok(&self.input[self.state.pos - 1])
    }

    pub(crate) fn enter_nl_context(&mut self) -> &mut Self {
        self.state.nl_depth += 1;
        self
    }

    pub(crate) fn exit_nl_context(&mut self) -> &mut Self {
        self.state.nl_depth -= 1;
        self
    }

    /// Run `parse` with newlines significant, whatever delimiters enclose it,
    /// then restore the enclosing newline context however `parse` ends.
    pub(crate) fn with_significant_newlines<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        let outer = std::mem::take(&mut self.state.nl_depth);
        let result = parse(self);
        self.state.nl_depth = outer;
        result
    }

    /// Skip newlines only when inside delimiters (nl_depth > 0).
    pub(crate) fn maybe_skip_nl(&mut self) -> &mut Self {
        if self.state.nl_depth > 0 {
            while matches!(self.peek().map(|t| &t.token), Some(Token::Newline)) {
                self.advance(1);
            }
        }
        self
    }

    /// Peek the next significant token, skipping any newlines, without
    /// advancing. Like `peek`, but sees past line breaks.
    pub(crate) fn peek_past_nl(&self) -> Option<&SrcToken<'src>> {
        self.get_past_nl(self.state.pos).map(|(_, token)| token)
    }

    /// The first token at or after `pos` that is not a newline, with its position.
    pub(crate) fn get_past_nl(&self, mut pos: usize) -> Option<(usize, &SrcToken<'src>)> {
        while matches!(self.input.get(pos).map(|t| &t.token), Some(Token::Newline)) {
            pos += 1;
        }
        self.input.get(pos).map(|token| (pos, token))
    }

    /// Advance past any run of newlines, regardless of `nl_depth`. The
    /// unconditional companion to `maybe_skip_nl`, used to commit a
    /// continuation after `peek_past_nl` confirms the following token.
    pub(crate) fn skip_nl(&mut self) -> &mut Self {
        while matches!(self.peek().map(|t| &t.token), Some(Token::Newline)) {
            self.advance(1);
        }
        self
    }

    /// Parse a comma-separated list of items inside delimiters.
    /// The opening delimiter must already be consumed and nl context entered.
    /// Consumes the closing delimiter and exits nl context.
    pub(crate) fn parse_comma_separated<T>(
        &mut self,
        close: Token,
        mut parse_item: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<(Vec<T>, &SrcToken<'src>)> {
        self.maybe_skip_nl();

        let mut items = Vec::new();
        let after_item = format!("`,` or `{close}`");

        if !matches!(self.peek().map(|t| &t.token), Some(t) if *t == close) {
            loop {
                self.maybe_skip_nl();
                items.push(parse_item(self)?);
                self.maybe_skip_nl();

                let peek = self.must_peek(&after_item)?;
                if peek.token == close {
                    break;
                }
                match peek.token {
                    Token::Comma => {
                        self.expect(Token::Comma)?;
                    }
                    _ => return Err(self.expected(&after_item, peek)),
                }

                self.maybe_skip_nl();
                if matches!(self.peek().map(|t| &t.token), Some(t) if *t == close) {
                    break;
                }
            }
        }

        self.maybe_skip_nl().exit_nl_context();
        let close_token = self.expect(close)?;
        Ok((items, close_token))
    }

    pub(crate) fn filename(&self) -> &'f str {
        self.filename
    }

    pub(crate) fn in_abbreviated_lambda(&self) -> bool {
        !self.state.abbrev_lambdas.is_empty()
    }

    pub(crate) fn enter_abbreviated_lambda(&mut self) -> &mut Self {
        self.state.abbrev_lambdas.push(DollarUsage::default());
        self
    }

    /// Ends the innermost abbreviated lambda, yielding the dollar-identifier
    /// usage its body recorded.
    pub(crate) fn exit_abbreviated_lambda(&mut self) -> DollarUsage {
        self.state
            .abbrev_lambdas
            .pop()
            .expect("IMPOSSIBLE: exit_abbreviated_lambda without a matching enter")
    }

    /// Records a dollar identifier against the innermost abbreviated lambda.
    /// `name` is the token text: `$`, `$1`..`$9`, or `$$`.
    pub(crate) fn record_dollar(&mut self, name: &str) {
        let frame = self
            .state
            .abbrev_lambdas
            .last_mut()
            .expect("IMPOSSIBLE: dollar identifier outside an abbreviated lambda");
        match name {
            "$$" => frame.rest = true,
            "$" => frame.mark(1),
            _ => {
                let n = name[1..]
                    .parse()
                    .expect("IMPOSSIBLE: the lexer only produces $1..$9 here");
                frame.mark(n);
            }
        }
    }

    /// Moves the abbreviated-lambda frames out, leaving none behind.
    ///
    /// Paired with [`restore_abbrev_frames`](Self::restore_abbrev_frames) to lend the frames to a format-string interpolation's sub-context (see `parse_interpolation`).
    pub(crate) fn take_abbrev_frames(&mut self) -> Vec<DollarUsage> {
        std::mem::take(&mut self.state.abbrev_lambdas)
    }

    /// Seats frames obtained from [`take_abbrev_frames`](Self::take_abbrev_frames).
    pub(crate) fn restore_abbrev_frames(&mut self, frames: Vec<DollarUsage>) {
        self.state.abbrev_lambdas = frames;
    }

    pub(crate) fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    pub(crate) fn here(&self) -> usize {
        self.state.pos
    }

    pub(crate) fn get(&self, pos: usize) -> Option<&SrcToken<'src>> {
        self.input.get(pos)
    }

    /// The source text `span` covers.
    /// `span` is in whole-source coordinates, and must lie within this context's source.
    pub(crate) fn source_text(&self, span: SourceSpan) -> &'src str {
        &self.full_source[span.start - self.base_offset..span.end - self.base_offset]
    }

    /// The error for finding `found` where `expected` belongs.
    /// `expected` is a noun phrase, such as "an expression" or "`,` or `]`".
    pub(crate) fn expected(&self, expected: &str, found: &SrcToken) -> Diagnostic {
        Diagnostic::at(
            format!("expected {expected}, but found {}", self.describe(found)),
            found.span.clone().into(),
            "unexpected",
        )
    }

    /// The error for running out of tokens where `expected` belongs.
    pub(crate) fn unexpected_eof(&self, expected: &str) -> Diagnostic {
        // End of this context's tokens, in whole-source coordinates.
        let end = self.base_offset + self.full_source.len();
        Diagnostic::at(
            format!("expected {expected}, but found the end of input"),
            (end..end).into(),
            "end of input",
        )
    }

    /// `token` as a diagnostic names it: its source text in backticks, up to any line break.
    fn describe(&self, token: &SrcToken) -> String {
        if token.token == Token::Newline {
            return "a line break".to_owned();
        }
        let text = self.source_text(token.span.clone().into());
        match text.split_once('\n') {
            Some((first_line, _)) => format!("`{}...`", first_line.trim_end_matches('\r')),
            None => format!("`{text}`"),
        }
    }
}

/// The Int an Int literal of `magnitude` denotes, negated if `negative`, or an
/// error at `span` if that is out of the Int range.
pub(crate) fn int_literal(magnitude: u64, negative: bool, span: Range<usize>) -> ParseResult<i64> {
    let value = if negative {
        0i64.checked_sub_unsigned(magnitude)
    } else {
        i64::try_from(magnitude).ok()
    };
    value.ok_or_else(|| {
        Diagnostic::at(
            "Int literal out of range",
            span.into(),
            "an Int is from -9223372036854775808 to 9223372036854775807",
        )
    })
}

fn lex_error(span: Range<usize>) -> Diagnostic {
    Diagnostic::at("unexpected character", span.into(), "unrecognized")
}
