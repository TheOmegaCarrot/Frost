// A parameter type must be a `FrostArg`, reported at the type.

use frostlang::frost_closure;

fn main() {
    let _ = frost_closure!("f", |_text: &'static str| 0);
}
