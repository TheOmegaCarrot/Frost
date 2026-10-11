// One context, marked once.

use frostlang::{NativeCtx, frost_fn};

#[frost_fn(name = "two_params")]
fn two_params(#[frost(ctx)] _a: &mut NativeCtx<'_>, #[frost(ctx)] _b: &mut NativeCtx<'_>) {}

#[frost_fn(name = "marked_twice")]
fn marked_twice(#[frost(ctx, ctx)] _ctx: &mut NativeCtx<'_>) {}

fn main() {}
