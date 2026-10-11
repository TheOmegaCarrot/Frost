// Frost calls the closure through a shared reference, from any thread, for
// as long as the function lives: rustc's own errors report a closure that
// mutates what it captures, or borrows from its surroundings.

use frostlang::frost_closure;

fn main() {
    let mut count = 0;
    let _ = frost_closure!("mutating", move || {
        count += 1;
        count
    });

    let local = String::from("local");
    let _ = frost_closure!("borrowing", || local.len());
}
