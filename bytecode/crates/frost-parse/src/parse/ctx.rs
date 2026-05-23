use std::ops::Range;

use crate::ast;
use crate::lex::Token;
use crate::parse::ParseError;

use logos::Logos;

pub struct ParseCtx<'src> {
    /// The original full source, preserved for diagnostics.
    full_source: &'src str,

    /// The full lexer result.
    input: Vec<Spanned<'src>>,

    state: ParseState,
}

pub struct Spanned<'src> {
    pub token: Token<'src>,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct ParseState {
    /// The current offset into `input`.
    pub pos: usize,

    /// Current line number.
    pub line: usize,
}

impl<'src> ParseCtx<'src> {
    pub fn checkpoint(&self) -> ParseState {
        self.state.clone()
    }

    pub fn restore(&mut self, state: ParseState) {
        self.state = state;
    }

    pub fn new(src: &'src str) -> Result<Self, ParseError> {
        let mut lexer = Token::lexer(src);
        let mut input = Vec::new();

        while let Some(token) = lexer.next() {

            let span = lexer.span();

            let Ok(token) = token else {
                return Err("placeholder for a proper miette error".into());
            };

            input.push(Spanned {
                token,
                span,
            });
        }

        Ok(Self {
            full_source: src,
            input,
            state: ParseState::default(),
        })
    }
}
