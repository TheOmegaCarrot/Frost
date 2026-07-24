use std::ops::Range;

use crate::lex::Token;
use crate::parse::{Diagnostic, ParseResult};

use logos::Logos;

#[derive(Debug)]
pub struct ParseCtx<'src, 'f> {
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
pub struct SrcToken<'src> {
    pub token: Token<'src>,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct ParseState {
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
pub struct DollarUsage {
    /// `used[i]` == whether `$(i+1)` was referenced (`$` counts as `$1`).
    /// The length is the highest positional referenced; empty if none.
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
    pub fn checkpoint(&self) -> ParseState {
        self.state.clone()
    }

    pub fn restore(&mut self, state: ParseState) {
        self.state = state;
    }

    pub fn new(filename: &'f str, src: &'src str) -> ParseResult<Self> {
        Self::new_with_offset(filename, src, 0)
    }

    pub fn new_with_offset(
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
    pub fn peek(&self) -> Option<&SrcToken<'src>> {
        self.input.get(self.state.pos)
    }

    pub fn must_peek(&self, context: &str) -> ParseResult<&SrcToken<'src>> {
        self.peek().ok_or_else(|| self.unexpected_eof(context))
    }

    /// Get the current token, and advance the state.
    pub fn next(&mut self) -> Option<&SrcToken<'src>> {
        self.advance(1);
        self.input.get(self.state.pos - 1)
    }

    /// Advance the state by n tokens.
    pub fn advance(&mut self, n: usize) -> &mut Self {
        self.state.pos += n;
        self
    }

    pub fn expect(&mut self, token: Token) -> ParseResult<&SrcToken<'src>> {
        let Some(current) = self.peek() else {
            return Err(self.unexpected_eof(format!("{token}").as_str()));
        };
        if current.token != token {
            return Err(Diagnostic::at(
                format!("expected {token}, but found {}", current.token),
                current.span.clone().into(),
                "unexpected",
            ));
        }

        self.advance(1);
        Ok(&self.input[self.state.pos - 1])
    }

    pub fn enter_nl_context(&mut self) -> &mut Self {
        self.state.nl_depth += 1;
        self
    }

    pub fn exit_nl_context(&mut self) -> &mut Self {
        self.state.nl_depth -= 1;
        self
    }

    /// Skip newlines only when inside delimiters (nl_depth > 0).
    pub fn maybe_skip_nl(&mut self) -> &mut Self {
        if self.state.nl_depth > 0 {
            while matches!(self.peek().map(|t| &t.token), Some(Token::Newline)) {
                self.advance(1);
            }
        }
        self
    }

    /// Peek the next significant token, skipping any newlines, without
    /// advancing. Like `peek`, but sees past line breaks.
    pub fn peek_past_nl(&self) -> Option<&SrcToken<'src>> {
        let mut pos = self.state.pos;
        while matches!(self.input.get(pos).map(|t| &t.token), Some(Token::Newline)) {
            pos += 1;
        }
        self.input.get(pos)
    }

    /// Advance past any run of newlines, regardless of `nl_depth`. The
    /// unconditional companion to `maybe_skip_nl`, used to commit a
    /// continuation after `peek_past_nl` confirms the following token.
    pub fn skip_nl(&mut self) -> &mut Self {
        while matches!(self.peek().map(|t| &t.token), Some(Token::Newline)) {
            self.advance(1);
        }
        self
    }

    /// Parse a comma-separated list of items inside delimiters.
    /// The opening delimiter must already be consumed and nl context entered.
    /// Consumes the closing delimiter and exits nl context.
    pub fn parse_comma_separated<T>(
        &mut self,
        close: Token,
        context: &str,
        mut parse_item: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<(Vec<T>, &SrcToken<'src>)> {
        self.maybe_skip_nl();

        let mut items = Vec::new();

        if !matches!(self.peek().map(|t| &t.token), Some(t) if *t == close) {
            loop {
                self.maybe_skip_nl();
                items.push(parse_item(self)?);
                self.maybe_skip_nl();

                let peek = self.must_peek(context)?;
                if peek.token == close {
                    break;
                }
                match peek.token {
                    Token::Comma => {
                        self.expect(Token::Comma)?;
                    }
                    _ => return Err(self.unexpected_token(peek, context)),
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

    pub fn filename(&self) -> &'f str {
        self.filename
    }

    pub fn in_abbreviated_lambda(&self) -> bool {
        !self.state.abbrev_lambdas.is_empty()
    }

    pub fn enter_abbreviated_lambda(&mut self) -> &mut Self {
        self.state.abbrev_lambdas.push(DollarUsage::default());
        self
    }

    /// Ends the innermost abbreviated lambda, yielding the dollar-identifier
    /// usage its body recorded.
    pub fn exit_abbreviated_lambda(&mut self) -> DollarUsage {
        self.state
            .abbrev_lambdas
            .pop()
            .expect("IMPOSSIBLE: exit_abbreviated_lambda without a matching enter")
    }

    /// Records a dollar identifier against the innermost abbreviated lambda.
    /// `name` is the token text: `$`, `$1`..`$9`, or `$$`.
    pub fn record_dollar(&mut self, name: &str) {
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

    pub fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    pub fn here(&self) -> usize {
        self.state.pos
    }

    pub fn get(&self, pos: usize) -> Option<&SrcToken<'src>> {
        self.input.get(pos)
    }

    pub fn unexpected_token(&self, token: &SrcToken, tried_to_parse: &str) -> Diagnostic {
        Diagnostic::at(
            format!("unexpected {} while parsing {tried_to_parse}", token.token),
            token.span.clone().into(),
            "unexpected",
        )
    }

    pub fn unexpected_eof(&self, tried_to_parse: &str) -> Diagnostic {
        // End of this context's tokens, in whole-source coordinates.
        let end = self.base_offset + self.full_source.len();
        Diagnostic::at(
            format!("unexpected end of input while parsing {tried_to_parse}"),
            (end..end).into(),
            "end of input",
        )
    }
}

fn lex_error(span: Range<usize>) -> Diagnostic {
    Diagnostic::at("unexpected character", span.into(), "unrecognized")
}
