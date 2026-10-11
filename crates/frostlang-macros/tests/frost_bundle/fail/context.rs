// The context has no name to leave out; and holding the state, a mutable
// bundle's function gets `&NativeCtx`, which cannot call back into Frost.

use frostlang::{NativeCtx, frost_mutable_bundle};

struct Counter(i64);

#[frost_mutable_bundle(prefix = "counter")]
impl Counter {
    fn unnamed(&self, #[frost(ctx, unnamed)] _ctx: &NativeCtx<'_>) {}
}

struct Holder(i64);

#[frost_mutable_bundle(prefix = "holder")]
impl Holder {
    fn calls_back(&mut self, #[frost(ctx)] _ctx: &mut NativeCtx<'_>) {}
}

fn main() {}
