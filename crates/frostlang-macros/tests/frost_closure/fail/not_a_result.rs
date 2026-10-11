// The result must be an `IntoNativeResult`, reported at the return type, or
// at the body when there is none.

use frostlang::frost_closure;

struct Opaque;

fn main() {
    let _ = frost_closure!("typed", || -> Opaque { Opaque });
    let _ = frost_closure!("untyped", || Opaque);
}
