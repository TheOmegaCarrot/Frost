// The context takes no argument, so no `types` constraint.

use frostlang::{NativeCtx, frost_fn};

#[frost_fn(name = "constrained_ctx")]
fn constrained_ctx(#[frost(ctx, types = Map)] _ctx: &mut NativeCtx<'_>) {}

fn main() {}
