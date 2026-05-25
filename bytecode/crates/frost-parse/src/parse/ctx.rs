use std::ops::Range;

use crate::ast;
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
}

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub fn checkpoint(&self) -> ParseState {
        self.state.clone()
    }

    pub fn restore(&mut self, state: ParseState) {
        self.state = state;
    }

    pub fn new(filename: &'f str, src: &'src str) -> ParseResult<Self> {
        let mut lexer = Token::lexer(src);
        let mut input = Vec::new();

        while let Some(token) = lexer.next() {
            let span = lexer.span();

            let Ok(token) = token else {
                return Err(lex_error(filename, src, span));
            };

            input.push(SrcToken { token, span });
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

    pub fn skip_nl(&mut self) -> &mut Self {
        while let Some(SrcToken {
            token: Token::Newline,
            ..
        }) = self.peek()
        {
            self.advance(1);
        }
        self
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
