// The context comes first.

use frostlang::{NativeCtx, frost_fn};

#[frost_fn(name = "f")]
fn f(_value: i64, #[frost(ctx)] _ctx: &mut NativeCtx<'_>) {}

fn main() {}
