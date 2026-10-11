// A `types` constraint stays within the types the parameter's Rust type
// accepts, failing const evaluation otherwise, reported at the marker.

use frostlang::frost_fn;
use frostlang::native::{Optional, Rest};

#[frost_fn(name = "disjoint")]
fn disjoint(#[frost(types = String)] _n: i64) {}

#[frost_fn(name = "overlapping")]
fn overlapping(#[frost(types = Null | Int)] _n: Optional<i64>) {}

#[frost_fn(name = "rest")]
fn rest(#[frost(types = Numeric)] _ns: Rest<i64>) {}

fn main() {}
