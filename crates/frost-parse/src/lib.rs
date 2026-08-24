//! Frost's parser.
//!
//! The API is a single function, [`parse_program`];
//! the real surface of this crate is the [`ast`] it produces.
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

pub mod ast;
mod lex;
mod parse;

pub use parse::{Label, ParseError, parse_program};
