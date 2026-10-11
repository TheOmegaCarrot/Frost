// The return type must be an `IntoNativeResult`, reported at the type.

use frostlang::frost_fn;

struct Opaque;

#[frost_fn(name = "f")]
fn f() -> Opaque {
    Opaque
}

fn main() {}
