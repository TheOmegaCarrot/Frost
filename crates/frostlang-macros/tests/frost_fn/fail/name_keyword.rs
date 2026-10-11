// A base name is a Frost identifier, not a keyword, whether it is the
// function's own or given.

use frostlang::frost_fn;

#[frost_fn]
fn map() {}

#[frost_fn(name = "if")]
fn condition() {}

#[frost_fn(name = "not-an-identifier")]
fn dashed() {}

// Braces are not format arguments in the message.
#[frost_fn(name = "a{b}")]
fn braced() {}

fn main() {}
