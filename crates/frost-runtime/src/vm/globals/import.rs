use std::sync::Arc;

use crate::{
    Arity, Bytecode, Closure, CompiledFunction, FormatVersion, FrostType, Param, Params, Value,
};

/// The `imported()` global: `true` when the running script is being imported by
/// another module, `false` when it is run directly. It reads the running Vm's
/// import depth, so it is never a compile-time constant.
pub(super) fn imported_global() -> Value {
    Value::native("imported", Arity::Exact(0), |ctx, _args| {
        Ok(Value::Bool(ctx.vm.is_imported()))
    })
}

pub(super) fn import_global() -> Value {
    Value::Closure(Arc::new(Closure {
        captures: Vec::new(),
        function: Arc::new(CompiledFunction {
            version: FormatVersion,
            name: "import".to_string(),
            arity: Arity::Exact(1),
            num_captures: 0,
            name_table: Vec::new(),
            constants: vec![import_type_check()],
            key_constants: Vec::new(),
            child_fns: Vec::new(),
            code: vec![
                Bytecode::DropBelow(1), // 0: drop the function itself -> ( spec )
                // Type-check the spec: when it is a String, skip to idx 8.
                Bytecode::PeekDown(0),                 // 1: ( spec spec )
                Bytecode::TypeTest(FrostType::STRING), // 2
                Bytecode::JumpIfTrue(4),               // 3: a String -> idx 8
                // A wrong type: the check raises its error, naming `import`.
                Bytecode::LoadConst(0), // 4: ( spec check )
                Bytecode::PeekDown(1),  // 5: ( spec check spec )
                Bytecode::Call(1),      // 6: raises
                Bytecode::Pop,          // 7
                Bytecode::Import,       // 8
            ],
        }),
    }))
}

/// A native named `import` whose only work is checking `import`'s argument
/// type, so a wrong type raises as a native's type check does. `import` runs it
/// only once its own fast check has failed.
fn import_type_check() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::STRING)]);
    Value::checked_native("import", PARAMS, |_, _| Ok(Value::Null))
}
