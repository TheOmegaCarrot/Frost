// A plain closure only: not `async`, `const`, or with a `for<...>` binder.

use frostlang::frost_closure;

fn main() {
    let _ = frost_closure!("asynchronous", async |n: i64| n);
    let _ = frost_closure!("constant", const |n: i64| n);
    let _ = frost_closure!("binder", for<'a> |n: i64| n);
}
