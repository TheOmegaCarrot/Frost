// The state is lent, never given: no `self` by value, nor a typed receiver.

use frostlang::{frost_bundle, frost_mutable_bundle};

struct Owned;

#[frost_bundle(prefix = "owned")]
impl Owned {
    fn by_value(self) {}
}

struct Boxed;

#[frost_mutable_bundle(prefix = "boxed")]
impl Boxed {
    fn typed(self: Box<Self>) {}

    fn mutable_value(mut self) {}
}

fn main() {}
