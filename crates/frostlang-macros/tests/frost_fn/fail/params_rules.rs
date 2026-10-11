// Parameters that break the rules of `Params` fail its const evaluation,
// reported at the parameter list.

use frostlang::frost_fn;
use frostlang::native::{Optional, Rest};

#[frost_fn(name = "f")]
fn f(_first: Optional<i64>, _rest: Rest<i64>) {}

fn main() {}
