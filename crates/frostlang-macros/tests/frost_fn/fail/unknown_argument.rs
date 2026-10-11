// `name` is the attribute's only argument.

use frostlang::frost_fn;

#[frost_fn(name = "f", strict = true)]
fn f() {}

fn main() {}
