// A base name is one segment: where the function is placed gives its path.

use frostlang::frost_fn;

#[frost_fn(name = "geometry.area")]
fn area() {}

fn main() {}
