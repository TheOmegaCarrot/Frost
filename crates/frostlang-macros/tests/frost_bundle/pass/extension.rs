// The bundle macros as an extension uses them: through `frostlang` without
// its default features, so without the compiler. Every way a function takes
// the state, groups, and an unnamed parameter.

use frostlang::native::{Bundle, BundleCtx, MutableBundle, MutableBundleCtx};
use frostlang::{FrostResult, NativeCtx, Value, frost_bundle, frost_mutable_bundle};

struct Prefix(String);

#[frost_bundle(prefix = "prefix")]
impl Prefix {
    fn apply(&self, #[frost(unnamed)] text: String) -> String {
        format!("{}{text}", self.0)
    }

    fn call(&self, #[frost(ctx)] ctx: &mut NativeCtx<'_>, function: Value) -> FrostResult {
        ctx.invoke(&function, [Value::from(self.0.as_str())])
    }

    fn whole(#[frost(ctx)] ctx: &mut BundleCtx<'_, Self>) -> String {
        ctx.state().0.clone()
    }

    fn version() -> i64 {
        1
    }
}

struct Log(Vec<String>);

#[frost_mutable_bundle(prefix = "log")]
impl Log {
    fn push(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>, line: String) {
        self.0.push(format!("{}: {line}", ctx.name()));
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn push_from(
        #[frost(ctx)] ctx: &mut MutableBundleCtx<'_, Self>,
        function: Value,
    ) -> FrostResult {
        let line = ctx.native().invoke(&function, [])?;
        ctx.lock()?.0.push(line.to_frost_string());
        Ok(Value::Null)
    }
}

#[frost_mutable_bundle(prefix = "log", group = clear_functions)]
impl Log {
    fn clear(&mut self) {
        self.0.clear();
    }
}

fn main() {
    let _: Value = Prefix::frost_functions(Bundle::new(Prefix(String::from("> ")))).build();
    let log = Log::frost_functions(MutableBundle::new(Log(Vec::new())));
    let _: Value = Log::clear_functions(log).build();
}
