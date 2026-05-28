use std::ops::Range;

use crate::lex::Token;
use crate::parse::{ParseError, ParseResult};

use logos::Logos;
use miette::{LabeledSpan, NamedSource, miette};

#[derive(Debug)]
pub struct ParseCtx<'src, 'f> {
    /// The original full source, preserved for diagnostics.
    full_source: &'src str,

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

            let Ok(token) = token else {
                let shifted = (span.start + base_offset)..(span.end + base_offset);
                return Err(lex_error(filename, src, shifted));
            };

            let shifted = (span.start + base_offset)..(span.end + base_offset);
            input.push(SrcToken {
                token,
                span: shifted,
            });
        }

        Ok(Self {
            full_source: src,
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
            return Err(miette!(
                labels = vec![LabeledSpan::at(
                    current.span.clone(),
                    format!("Expected {}, but got {}.", &token, &current.token)
                )],
                ""
            )
            .with_source_code(self.named_source())
            .into());
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

    pub fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    pub fn here(&self) -> usize {
        self.state.pos
    }

    pub fn get(&self, pos: usize) -> Option<&SrcToken<'src>> {
        self.input.get(pos)
    }

    pub fn unexpected_token(&self, token: &SrcToken, tried_to_parse: &str) -> ParseError {
        miette!(
            labels = vec![LabeledSpan::at(token.span.clone(), "unexpected")],
            "unexpected {} while parsing {tried_to_parse}",
            token.token
        )
        .with_source_code(self.named_source())
        .into()
    }

    pub fn unexpected_eof(&self, tried_to_parse: &str) -> ParseError {
        let end = self.full_source.len();
        miette!(
            labels = vec![LabeledSpan::at(end..end, "end of input")],
            "unexpected end of input while parsing {tried_to_parse}"
        )
        .with_source_code(self.named_source())
        .into()
    }

    /// Expensive: only called in the error case
    pub fn named_source(&self) -> NamedSource<String> {
        NamedSource::new(self.filename, self.full_source.to_owned())
    }
}

fn lex_error(filename: &str, src: &str, span: Range<usize>) -> ParseError {
    let source = NamedSource::new(filename, src.to_owned());

    miette!(
        labels = vec![LabeledSpan::at(span, "unrecognized")],
        "unexpected character"
    )
    .with_source_code(source)
    .into()
}
