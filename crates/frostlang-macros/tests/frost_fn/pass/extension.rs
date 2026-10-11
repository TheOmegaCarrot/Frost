// `#[frost_fn]` as an extension uses it: through `frostlang` without its
// default features, so without the compiler. Every kind of parameter, a
// `types` constraint, the context, a result, a base name of the function's
// own and a given one, and placement in an extension.

use frostlang::native::{Function, NativeDef, Nullable, Optional, Rest};
use frostlang::{Extension, FrostResult, NativeCtx, Value, frost_fn};

#[frost_fn]
pub fn label(text: String, prefix: Optional<Nullable<String>>) -> String {
    match prefix {
        Optional(Some(Nullable(Some(prefix)))) => format!("{prefix}: {text}"),
        Optional(_) => text,
    }
}

#[frost_fn]
fn call_each(#[frost(ctx)] ctx: &mut NativeCtx<'_>, functions: Rest<Function>) -> FrostResult {
    let mut last = Value::Null;
    for function in &functions.0 {
        last = ctx.invoke(function, [])?;
    }
    Ok(last)
}

#[frost_fn(name = "is_text")]
fn is_textual(#[frost(unnamed, types = Flat | Array)] sequence: Value) -> bool {
    !sequence.is_array()
}

#[frost_fn]
fn r#struct() {}

const LABEL: NativeDef = label_frost();

fn main() {
    assert_eq!(LABEL.base_name(), "label");
    assert_eq!(is_textual_frost().base_name(), "is_text");
    assert_eq!(struct_frost().base_name(), "struct");
    let _: Value = call_each_frost().build();
    let _: Value = is_textual_frost().build_named("text.is_text");
    let extension: Extension = Extension::builder("tools")
        .function(LABEL)
        .module("calls", |calls| calls.function(call_each_frost()))
        .build();
    assert_eq!(extension.name(), "tools");
    assert_eq!(label("text".to_string(), Optional(None)), "text");
}
