// Each parameter but the context is a plain name, typed or not.

use frostlang::frost_closure;

fn main() {
    let _ = frost_closure!("tuple", |(_a, _b): (i64, i64)| 0);
    let _ = frost_closure!("wildcard", |_| 0);
}
