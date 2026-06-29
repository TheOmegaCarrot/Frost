//! Type checking, conversion, and value serialization.

use std::sync::Arc;

use crate::{Arity, NativeFunction, Value};

// `unwrap` is safer than usual in a Frost nativeFunction,
// because once control enters the implementation, arity is already checked.

pub(super) fn is_null_global() -> Value {
    Value::NativeFunction(Arc::new (NativeFunction{
        arity: Arity::Exact(1),
        name: "is_null",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_null().into()) )
    }))
}

pub(super) fn is_int_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_int",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_int().into())),
    }))
}

pub(super) fn is_float_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_float",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_float().into())),
    }))
}

pub(super) fn is_bool_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_bool",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_bool().into())),
    }))
}

pub(super) fn is_string_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_string",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_string().into())),
    }))
}

pub(super) fn is_array_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_array",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_array().into())),
    }))
}

pub(super) fn is_map_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_map",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_map().into())),
    }))
}

pub(super) fn is_function_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_function",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_function().into())),
    }))
}

pub(super) fn is_nonnull_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_nonnull",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_nonnull().into())),
    }))
}

pub(super) fn is_numeric_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_numeric",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_numeric().into())),
    }))
}

pub(super) fn is_primitive_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_primitive",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_primitive().into())),
    }))
}

pub(super) fn is_structured_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "is_structured",
        function: Box::new(|_, args| Ok(args.first().unwrap().is_structured().into())),
    }))
}

pub(super) fn type_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "type",
        function: Box::new(|_, args| Ok(Value::from(args.first().unwrap().type_name()))),
    }))
}

pub(super) fn to_string_global() -> Value {
    super::stub("to_string")
}

pub(super) fn pretty_global() -> Value {
    super::stub("pretty")
}

pub(super) fn to_int_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "to_int",
        function: Box::new(|_, args| Ok(args.first().unwrap().to_frost_int())),
    }))
}

pub(super) fn to_float_global() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction {
        arity: Arity::Exact(1),
        name: "to_float",
        function: Box::new(|_, args| Ok(args.first().unwrap().to_frost_float())),
    }))
}
