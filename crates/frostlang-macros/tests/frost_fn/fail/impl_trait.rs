// `impl Trait` in a parameter is generic too, wherever it appears in the type.

use frostlang::frost_fn;
use frostlang::native::Rest;

#[frost_fn(name = "outer")]
fn outer(_value: impl Into<i64>) {}

#[frost_fn(name = "nested")]
fn nested(_values: Rest<impl Into<i64>>) {}

fn main() {}
