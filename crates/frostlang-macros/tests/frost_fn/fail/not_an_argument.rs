// A parameter type must be a `FrostArg`, reported at the type.

use frostlang::frost_fn;

#[frost_fn(name = "f")]
fn f(_text: &'static str) {}

fn main() {}
