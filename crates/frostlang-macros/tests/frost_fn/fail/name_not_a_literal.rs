// A given base name is a string literal.

use frostlang::frost_fn;

const NAME: &str = "area";

#[frost_fn(name = NAME)]
fn area() {}

fn main() {}
