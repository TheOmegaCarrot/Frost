#![allow(unused)]

pub mod ast;
pub mod lex;
mod parse;

pub use parse::{parse_program, ParseError};
