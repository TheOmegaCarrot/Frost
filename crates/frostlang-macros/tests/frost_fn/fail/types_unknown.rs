// A `types` constraint names Frost types and categories, spelled as `is`
// spells them.

use frostlang::{Value, frost_fn};

#[frost_fn(name = "misspelled")]
fn misspelled(#[frost(types = Strng | Numeric | Mapp)] _value: Value) {}

#[frost_fn(name = "lowercase")]
fn lowercase(#[frost(types = string)] _value: Value) {}

// Any is no constraint: an unconstrained `Value` already accepts any type.
#[frost_fn(name = "any")]
fn any(#[frost(types = Any)] _value: Value) {}

fn main() {}
