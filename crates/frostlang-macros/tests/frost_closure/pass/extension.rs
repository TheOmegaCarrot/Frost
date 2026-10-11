// `frost_closure!` as an extension uses it: through `frostlang` without its
// default features, so without the compiler. Captures, a runtime name, a
// generic factory, untyped parameters, and an untyped context.

use frostlang::native::{FromArg, IntoNativeResult, Optional, Rest};
use frostlang::{Value, frost_closure};

fn scaler(name: &'static str, factor: i64) -> Value {
    frost_closure!(name, move |offset: Optional<i64>| {
        frost_closure!("ext.scaled", move |ns: Rest<i64>| {
            ns.0.iter().map(|n| n * factor + offset.0.unwrap_or(0)).collect::<Vec<_>>()
        })
    })
}

fn first<T: FromArg + IntoNativeResult + 'static>(name: &'static str) -> Value {
    frost_closure!(name, |items: Vec<T>| items.into_iter().next())
}

fn main() {
    let _: Value = scaler("ext.double", 2);
    let _: Value = first::<String>("ext.first");
    let _: Value = frost_closure!("ext.call", |#[frost(ctx)] ctx, function, argument| {
        ctx.invoke(&function, [argument])
    });
}
