use std::ops::Range;

use crate::ast::SourceSpan;
use crate::lex::Token;
use crate::parse::{Diagnostic, ParseResult, hints};

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

    /// For an interpolation sub-context, the `}` that closes it.
    /// Past the context's last token, a parse finds this brace rather than the end of input.
    closing_brace: Option<SourceSpan>,

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
        Self::lex(filename, src, 0, None)
    }

    /// A sub-context for a format-string interpolation: `src` is its text, which
    /// starts at `base_offset` in the whole source and is closed by the `}` right after it.
    pub(crate) fn new_interpolation(
        filename: &'f str,
        src: &'src str,
        base_offset: usize,
    ) -> ParseResult<Self> {
        let brace = base_offset + src.len();
        Self::lex(filename, src, base_offset, Some((brace..brace + 1).into()))
    }

    fn lex(
        filename: &'f str,
        src: &'src str,
        base_offset: usize,
        closing_brace: Option<SourceSpan>,
    ) -> ParseResult<Self> {
        let mut lexer = Token::lexer(src);
        let mut input = Vec::new();

        while let Some(token) = lexer.next() {
            let span = lexer.span();
            let shifted = (span.start + base_offset)..(span.end + base_offset);

            let Ok(token) = token else {
                let rest_of_line = src[span.start..].lines().next().unwrap_or_default();
                return Err(lex_error(rest_of_line, shifted));
            };

            input.push(SrcToken {
                token,
                span: shifted,
            });
        }

        Ok(Self {
            full_source: src,
            base_offset,
            closing_brace,
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
        let Some(current) = self.peek() else {
            let help = self.habit_help(self.input.len(), Some(&token));
            return Err(self.unexpected_eof(&expected).with_help(help));
        };
        if current.token != token {
            return Err(self.expected_token(&expected, current, Some(&token)));
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
    /// `open` is the opening delimiter, and `list` names the list for an error, as in
    /// [`expected_in_list`](Self::expected_in_list).
    pub(crate) fn parse_comma_separated<T>(
        &mut self,
        open: SourceSpan,
        list: &str,
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
                    _ => return Err(self.expected_in_list(&after_item, peek, open, list)),
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
        self.expected_token(expected, found, None)
    }

    /// [`expected`](Self::expected), knowing the one `token` that was expected, if so.
    fn expected_token(
        &self,
        expected: &str,
        found: &SrcToken,
        token: Option<&Token>,
    ) -> Diagnostic {
        let help = self
            .input
            .iter()
            .position(|t| t.span == found.span)
            .and_then(|index| self.habit_help(index, token));
        Diagnostic::at(
            format!("expected {expected}, but found {}", self.describe(found)),
            found.span.clone().into(),
            "unexpected",
        )
        .with_help(help)
    }

    /// [`expected`](Self::expected), for a list opened at `open`.
    /// When the list runs across lines to `found`, the opener is labeled too,
    /// as "in this {list}", so the error shows which list it is in.
    pub(crate) fn expected_in_list(
        &self,
        expected: &str,
        found: &SrcToken,
        open: SourceSpan,
        list: &str,
    ) -> Diagnostic {
        let diagnostic = self.expected(expected, found);
        if self
            .source_text((open.start..found.span.start).into())
            .contains('\n')
        {
            diagnostic.with_label(open, format!("in this {list}"))
        } else {
            diagnostic
        }
    }

    /// The error for running out of tokens where `expected` belongs.
    /// Any bracket still open is labeled, since closing it is the likely fix.
    pub(crate) fn unexpected_eof(&self, expected: &str) -> Diagnostic {
        let unclosed = self.innermost_unclosed();
        let last = self.input.iter().rev().find(|t| t.token != Token::Newline);

        let diagnostic = if let Some(brace) = self.closing_brace {
            Diagnostic::at(
                format!("expected {expected}, but found `}}`"),
                brace,
                "unexpected",
            )
        } else {
            let message = format!("expected {expected}, but found the end of input");
            match last {
                // An unclosed opener as the last token gets just its own label.
                Some(last) if unclosed.is_some_and(|open| open.span == last.span) => {
                    Diagnostic::new(message)
                }
                Some(last) => Diagnostic::at(
                    message,
                    last.span.clone().into(),
                    "the input ends after this",
                ),
                // No tokens, only line breaks and comments: label the end itself,
                // though a zero-width label is not drawn.
                None => {
                    let end = self.base_offset + self.full_source.len();
                    Diagnostic::at(message, (end..end).into(), "end of input")
                }
            }
        };

        match unclosed {
            Some(open) => diagnostic.with_label(
                open.span.clone().into(),
                format!("this `{}` is not closed", open.token),
            ),
            None => diagnostic,
        }
    }

    /// The innermost opening bracket in this context with no closing bracket after it.
    /// Brackets are matched by nesting alone, which suffices to pick a label.
    fn innermost_unclosed(&self) -> Option<&SrcToken<'src>> {
        let mut open = Vec::new();
        for token in &self.input {
            match token.token {
                Token::OpenParen | Token::DollarParen | Token::OpenBracket | Token::OpenBrace => {
                    open.push(token);
                }
                Token::CloseParen | Token::CloseBracket | Token::CloseBrace => {
                    open.pop();
                }
                _ => {}
            }
        }
        open.pop()
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

/// The error for source the lexer cannot read at `span`, `rest_of_line` being the
/// source from there to the end of its line.
fn lex_error(rest_of_line: &str, span: Range<usize>) -> Diagnostic {
    let (message, help) = hints::unreadable(rest_of_line);
    Diagnostic::at(message, span.into(), "unrecognized").with_help(help)
}
