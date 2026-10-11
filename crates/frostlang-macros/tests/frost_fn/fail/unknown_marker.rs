// A parameter takes only the known markers, and `ctx` takes no value.

use frostlang::{NativeCtx, frost_fn};

#[frost_fn(name = "unknown")]
fn unknown(#[frost(skip)] _value: i64) {}

#[frost_fn(name = "valued")]
fn valued(#[frost(ctx = true)] _ctx: &mut NativeCtx<'_>) {}

#[frost_fn(name = "bare")]
fn bare(#[frost] _value: i64) {}

fn main() {}
