// Nothing could uphold an `unsafe` function's requirements when Frost calls it.

use frostlang::frost_fn;

#[frost_fn(name = "f")]
unsafe fn f() {}

fn main() {}
