use std::ops::Range;

use crate::ast;
use crate::lex::Token;
use crate::parse::ParseError;

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

    pub fn new(filename: &'f str, src: &'src str) -> Result<Self, ParseError> {
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

    pub fn expect(&mut self, token: Token) -> Result<&mut Self, ParseError> {
        let Some(current) = self.peek() else {
            return Err(miette!(format!("Expected {token}, but got end of input.")).into());
        };
        if current.token == token {
            self.advance(1);
        } else {
            return Err(miette!(
                labels = vec![LabeledSpan::at(
                    current.span.clone(),
                    format!("Expected {}, but got {}.", &token, &current.token)
                )],
                ""
            )
            .with_source_code(self.named_source())
            .into());
        };

        Ok(self)
    }

    pub fn skip_nl(&mut self) -> &mut Self {
        while let Some(SrcToken{token: Token::Newline, ..}) = self.peek() {
            self.advance(1);
        }
        self
    }

    pub fn at_end(&self) -> bool {
        self.peek().is_none()
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
