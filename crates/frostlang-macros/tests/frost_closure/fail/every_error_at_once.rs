// One compile reports every error in a closure, not just the first.

use frostlang::frost_closure;

fn main() {
    let _ = frost_closure!("f", async |(_a, _b): (i64, i64), #[frost(skip)] _c| 0);
}
