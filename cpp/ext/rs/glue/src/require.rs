//! Runtime argument validation for Rust closures, producing error messages
//! matching Frost's C++ REQUIRE_ARGS format.
//!
//! Usage:
//! ```ignore
//! use frost_glue::require::{require_args, Param, Type};
//!
//! require_args("csv.write.row", args, &[
//!     Param::required("row", &[Type::Array]),
//! ])?;
//! ```

use crate::FrostValue;

/// Frost type tags for validation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Null,
    Int,
    Float,
    Bool,
    String,
    Array,
    Map,
    Function,
}

impl Type {
    fn name(self) -> &'static str {
        match self {
            Type::Null => "Null",
            Type::Int => "Int",
            Type::Float => "Float",
            Type::Bool => "Bool",
            Type::String => "String",
            Type::Array => "Array",
            Type::Map => "Map",
            Type::Function => "Function",
        }
    }

    fn matches(self, val: &FrostValue) -> bool {
        match self {
            Type::Null => val.is_null(),
            Type::Int => val.is_int(),
            Type::Float => val.is_float(),
            Type::Bool => val.is_bool(),
            Type::String => val.is_string(),
            Type::Array => val.is_array(),
            Type::Map => val.is_map(),
            Type::Function => val.is_function(),
        }
    }
}

/// A parameter specification.
pub struct Param {
    pub label: &'static str,
    pub types: &'static [Type],
    pub optional: bool,
}

impl Param {
    pub const fn required(label: &'static str, types: &'static [Type]) -> Self {
        Self { label, types, optional: false }
    }

    pub const fn optional(label: &'static str, types: &'static [Type]) -> Self {
        Self { label, types, optional: true }
    }

    /// Special: matches any type.
    pub const fn any(label: &'static str) -> Self {
        Self { label, types: &[], optional: false }
    }

    /// Special: optional, matches any type.
    pub const fn optional_any(label: &'static str) -> Self {
        Self { label, types: &[], optional: true }
    }

    fn accepts_any(&self) -> bool {
        self.types.is_empty()
    }

    fn matches(&self, val: &FrostValue) -> bool {
        if self.accepts_any() {
            return true;
        }
        self.types.iter().any(|t| t.matches(val))
    }

    fn expected_list(&self) -> std::string::String {
        if self.accepts_any() {
            return "Any".to_owned();
        }
        self.types.iter()
            .map(|t| t.name())
            .collect::<Vec<_>>()
            .join(" or ")
    }
}

pub fn require_nullary(fn_name: &str, args: &[FrostValue]) -> Result<(), String> {
    require_args(fn_name, args, &[])
}

/// Validate arguments against a parameter spec, producing REQUIRE_ARGS-style
/// error messages on failure.
///
/// - Checks arity (too few / too many)
/// - Checks each argument's type against the spec
/// - Optional params are skipped if not provided
pub fn require_args(fn_name: &str, args: &[FrostValue], params: &[Param]) -> Result<(), String> {
    let min_args = params.iter().filter(|p| !p.optional).count();
    let max_args = params.len();

    if args.len() < min_args {
        return Err(format!(
            "Function {} called with insufficient arguments. \
             Called with {} but requires at least {}.",
            fn_name, args.len(), min_args
        ));
    }

    if args.len() > max_args {
        return Err(format!(
            "Function {} called with too many arguments. \
             Called with {} but accepts no more than {}.",
            fn_name, args.len(), max_args
        ));
    }

    for (i, param) in params.iter().enumerate() {
        if i >= args.len() {
            break;
        }
        if !param.matches(&args[i]) {
            let label_part = if param.label.is_empty() {
                format!("argument {}", i + 1)
            } else {
                format!("argument {} ({})", i + 1, param.label)
            };
            return Err(format!(
                "Function {} requires {} as {}, got {}",
                fn_name,
                param.expected_list(),
                label_part,
                args[i].type_name()
            ));
        }
    }

    Ok(())
}
