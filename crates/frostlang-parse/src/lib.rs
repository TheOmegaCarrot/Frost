//! Frost's parser.
//!
//! The API is [`parse_program`], and the real surface of this crate is the
//! [`ast`] it produces. For tools that work below the level of syntax, such as
//! highlighting, [`tokens`] exposes the lexer.
//! Parsing yields either a complete [`ast::Program`] or a single [`ParseError`]:
//! the parser stops at the first error and never attempts recovery,
//! so there is never more than one diagnostic.
//!
//! # Span invariants
//!
//! Every AST node arrives wrapped in [`ast::Spanned`], whose span obeys:
//!
//! - Spans are byte offsets into the source string given to [`parse_program`],
//!   half-open: `start` inclusive, `end` exclusive.
//! - Spans are absolute, including inside format-string interpolations:
//!   positions always index the whole source, never a substring of it.
//! - Containment: a node's span encloses the spans of all its children.
//! - Equality ignores spans (see [`ast::Spanned`]).
//!
//! The labeled spans on a [`ParseError`] (see [`Label`]) follow the same
//! byte-offset conventions.

/// The syntax tree [`parse_program`] produces, rooted at [`Program`](ast::Program).
pub mod ast;
mod lex;
mod parse;

pub use lex::{LexError, Token, tokens};
pub use parse::{Label, ParseError, parse_program};
