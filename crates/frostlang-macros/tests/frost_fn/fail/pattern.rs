// Each parameter but the context is a plain name, which names it in errors.

use frostlang::frost_fn;

#[frost_fn(name = "tuple")]
fn tuple((_a, _b): (i64, i64)) {}

#[frost_fn(name = "wildcard")]
fn wildcard(_: i64) {}

#[frost_fn(name = "by_ref")]
fn by_ref(ref _value: i64) {}

#[frost_fn(name = "bound")]
fn bound(_value @ _: i64) {}

fn main() {}
