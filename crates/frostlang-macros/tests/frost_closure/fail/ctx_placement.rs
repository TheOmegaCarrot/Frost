// One context, first, marked once; `#[frost(ctx)]` the only marker.

use frostlang::frost_closure;

fn main() {
    let _ = frost_closure!("not_first", |_value: i64, #[frost(ctx)] _ctx| 0);
    let _ = frost_closure!("twice", |#[frost(ctx)] _a, #[frost(ctx)] _b| 0);
    let _ = frost_closure!("unknown", |#[frost(skip)] _value: i64| 0);
}
