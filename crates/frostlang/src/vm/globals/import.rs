use std::sync::Arc;

use crate::bytecode::{Bytecode, CompiledFunction, FormatVersion};
use crate::{Arity, Closure, FrostType, NativeCtx, Param, Params, Value, frost_fn};

/// The `imported()` global: `true` when the running script is being imported by
/// another module, `false` when it is run directly. It reads the running Vm's
/// import depth, so it is never a compile-time constant.
#[frost_fn]
pub(super) fn imported(#[frost(ctx)] ctx: &mut NativeCtx<'_>) -> bool {
    ctx.vm.is_imported()
}

pub(super) fn import_global() -> Value {
    Value::Closure(Arc::new(Closure {
        captures: Vec::new(),
        function: Arc::new(CompiledFunction {
            version: FormatVersion,
            name: "import".to_string(),
            origin: None,
            arity: Arity::Exact(1),
            num_captures: 0,
            name_table: Vec::new(),
            constants: vec![import_type_error()],
            key_constants: Vec::new(),
            child_fns: Vec::new(),
            debug_info: None,
            code: vec![
                Bytecode::DropBelow(1), // 0: drop the function itself -> ( spec )
                // Type-check the spec: when it is a String, skip to idx 8.
                Bytecode::PeekDown(0),                 // 1: ( spec spec )
                Bytecode::TypeTest(FrostType::STRING), // 2
                Bytecode::JumpIfTrue(4),               // 3: a String -> idx 8
                // A wrong type: raise the error naming `import` from this frame.
                Bytecode::LoadConst(0), // 4: ( spec type_error )
                Bytecode::PeekDown(1),  // 5: ( spec type_error spec )
                Bytecode::Call(1),      // 6: ( spec error )
                Bytecode::ProduceError, // 7: raises
                Bytecode::Import,       // 8
            ],
        }),
    }))
}

/// The native giving the error for `import`'s argument having the wrong type
/// (see `type_error_native`).
fn import_type_error() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::STRING)]);
    crate::vm::globals::type_error_native("import", PARAMS)
}
