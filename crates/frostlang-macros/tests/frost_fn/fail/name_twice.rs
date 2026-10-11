// `name` is given once.

use frostlang::frost_fn;

#[frost_fn(name = "f", name = "g")]
fn f() {}

fn main() {}
