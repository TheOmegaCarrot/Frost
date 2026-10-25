// A result is never absent, so an `Option` is no result: the error points to
// `Nullable`, for a result that may be Null.

use frostlang::frost_fn;

#[frost_fn(name = "f")]
fn f(some: bool) -> Option<i64> {
    some.then_some(1)
}

fn main() {}
