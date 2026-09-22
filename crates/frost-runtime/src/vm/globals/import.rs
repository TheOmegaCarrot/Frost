use std::sync::Arc;

use crate::{Arity, Bytecode, Closure, CompiledFunction, FormatVersion, Value};

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
            constants: Vec::new(),
            key_constants: Vec::new(),
            child_fns: Vec::new(),
            code: vec![
                Bytecode::DropBelow(1), // Drop the function itself
                Bytecode::Import,
            ],
        }),
    }))
}
