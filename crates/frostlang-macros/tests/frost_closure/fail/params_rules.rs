// Parameters that break the rules of `Params` fail its const evaluation.

use frostlang::frost_closure;
use frostlang::native::{Optional, Rest};

fn main() {
    let _ = frost_closure!("f", |_first: Optional<i64>, _rest: Rest<i64>| 0);
}
