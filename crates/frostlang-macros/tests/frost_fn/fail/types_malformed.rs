// A `types` constraint is given once, as type names joined by `|`.

use frostlang::{Value, frost_fn};

#[frost_fn(name = "no_value")]
fn no_value(#[frost(types)] _value: Value) {}

#[frost_fn(name = "empty")]
fn empty(#[frost(types = )] _value: Value) {}

#[frost_fn(name = "plus")]
fn plus(#[frost(types = String + Bytes)] _value: Value) {}

#[frost_fn(name = "quoted")]
fn quoted(#[frost(types = "String")] _value: Value) {}

#[frost_fn(name = "trailing")]
fn trailing(#[frost(types = String |)] _value: Value) {}

#[frost_fn(name = "twice")]
fn twice(#[frost(types = String, types = Bytes)] _value: Value) {}

fn main() {}
