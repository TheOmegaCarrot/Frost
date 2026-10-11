// `#[frost_fn]` as an extension uses it: through `frostlang` without its
// default features, so without the compiler. Every kind of parameter, the
// context, and a result.

use frostlang::native::{Function, Nullable, Optional, Rest};
use frostlang::{FrostResult, NativeCtx, Value, frost_fn};

#[frost_fn(name = "ext.label")]
pub fn label(text: String, prefix: Optional<Nullable<String>>) -> String {
    match prefix {
        Optional(Some(Nullable(Some(prefix)))) => format!("{prefix}: {text}"),
        Optional(_) => text,
    }
}

#[frost_fn(name = "ext.call_each")]
fn call_each(#[frost(ctx)] ctx: &mut NativeCtx<'_>, functions: Rest<Function>) -> FrostResult {
    let mut last = Value::Null;
    for function in &functions.0 {
        last = ctx.invoke(function, [])?;
    }
    Ok(last)
}

fn main() {
    let _: Value = label_frost();
    let _: Value = call_each_frost();
    assert_eq!(label("text".to_string(), Optional(None)), "text");
}
