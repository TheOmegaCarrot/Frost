// Frost reaches each function as `.key`, so the prefix and keys are Frost
// identifiers, not keywords; checked at compile time.

use frostlang::{frost_bundle, frost_mutable_bundle};

struct Keyword;

#[frost_bundle(prefix = "match")]
impl Keyword {
    fn def(&self) {}
}

struct Dotted;

#[frost_mutable_bundle(prefix = "a.b")]
impl Dotted {
    fn fine(&self) {}
}

fn main() {}
