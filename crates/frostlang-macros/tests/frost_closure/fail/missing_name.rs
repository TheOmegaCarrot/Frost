// The Frost function's name comes first.

use frostlang::frost_closure;

fn main() {
    let _ = frost_closure!(|n: i64| n);
}
