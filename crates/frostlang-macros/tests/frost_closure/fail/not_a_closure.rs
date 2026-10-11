// A closure only: a function takes `#[frost_fn]`.

use frostlang::frost_closure;

fn double(n: i64) -> i64 {
    n * 2
}

fn main() {
    let _ = frost_closure!("double", double);
}
