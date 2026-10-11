// A plain Rust function only.

use frostlang::frost_fn;

#[frost_fn(name = "f")]
extern "C" fn f() {}

fn main() {}
