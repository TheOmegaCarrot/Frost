// One compile reports every error in a declaration, not just the first.

use frostlang::frost_fn;

#[frost_fn(strict = true)]
async fn f<T>((_a, _b): (T, T), #[frost(skip)] _c: i64) {}

fn main() {}
