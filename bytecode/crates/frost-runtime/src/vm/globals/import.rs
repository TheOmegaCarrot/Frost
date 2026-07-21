use std::sync::Arc;

use crate::{Arity, Bytecode, Closure, CompiledFunction, FormatVersion, Value};

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
            child_fns: Vec::new(),
            code: vec![
                Bytecode::DropBelow(1), // Drop the function itself
                Bytecode::Import,
            ],
        }),
    }))
}
