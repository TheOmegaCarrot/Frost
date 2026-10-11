// A Frost function cannot await.

use frostlang::frost_fn;

#[frost_fn(name = "f")]
async fn f() {}

fn main() {}
