use std::ops::Range;

use crate::ast;
use crate::lex::Token;
use crate::parse::ParseError;

use logos::Logos;
use miette::{miette, LabeledSpan, NamedSource};

#[derive(Debug)]
pub struct ParseCtx<'src, 'f> {
    /// The original full source, preserved for diagnostics.
    full_source: &'src str,

    filename: &'f str,

    /// The full lexer result.
    input: Vec<Spanned<'src>>,

    state: ParseState,
}

#[derive(Debug)]
pub struct Spanned<'src> {
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

            input.push(Spanned {
                token,
                span,
            });
        }

        Ok(Self {
            full_source: src,
            filename,
            input,
            state: ParseState::default(),
        })
    }
}

fn lex_error(filename: &str, src: &str, span: Range<usize>) -> ParseError {

    let source = NamedSource::new(filename, src.to_owned());

    let report = miette!(
        labels = vec![LabeledSpan::at(span, "unrecognized")],
        "unexpected character"
    )
    .with_source_code(source);

    ParseError(format!("{:?}", report))
}
