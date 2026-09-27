//! Printing and message formatting.

use crate::{Arity, Value};

/// `print(value)`: hand the value, as `to_string` renders it, to the Vm's print
/// sink. Returns Null.
pub(super) fn print_global() -> Value {
    Value::native("print", Arity::Exact(1), |ctx, args| {
        ctx.vm.config.print_sink.print(&args[0].to_frost_string());
        Ok(Value::Null)
    })
}

pub(super) fn mformat_global() -> Value {
    super::stub("mformat")
}

pub(super) fn mprint_global() -> Value {
    super::stub("mprint")
}
