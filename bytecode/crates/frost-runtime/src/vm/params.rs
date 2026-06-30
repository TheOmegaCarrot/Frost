//! Declarative argument specs for native functions.
//!
//! A `&'static [Param]` describes a native's parameters; from it we derive the
//! function's [`Arity`] ([`ParamSpec::arity`]) and type-check its arguments
//! ([`check_args`]), producing Frost's standard
//! `requires {types} as argument {N}{ (name)}, got {Type}` message.
//!
//! Functions whose validity can't be expressed per-parameter (e.g. operators,
//! where `Int + Int` is fine but `Int + String` is not) skip this and validate in
//! their own body instead.

use crate::{Arity, FrostType, Value};

/// One parameter of a native function.
pub struct Param {
    /// Surfaced in the error as `argument N (name)` when present; most params omit it.
    pub(crate) name: Option<&'static str>,
    /// Accepted types. An empty slice accepts any value (`Any`).
    types: &'static [FrostType],
    optional: bool,
}

impl Param {
    /// A required parameter accepting any of `types`.
    pub const fn of(types: &'static [FrostType]) -> Self {
        Self {
            name: None,
            types,
            optional: false,
        }
    }

    /// A required parameter accepting any value.
    pub const fn any() -> Self {
        Self {
            name: None,
            types: &[],
            optional: false,
        }
    }

    /// Attach a name, surfaced in the error message.
    pub const fn named(self, name: &'static str) -> Self {
        Self {
            name: Some(name),
            types: self.types,
            optional: self.optional,
        }
    }

    /// Mark this parameter optional (may be omitted).
    pub const fn optional(self) -> Self {
        Self {
            name: self.name,
            types: self.types,
            optional: true,
        }
    }

    pub(crate) fn accepts(&self, value: &Value) -> bool {
        self.types.is_empty() || self.types.contains(&value.frost_type())
    }

    /// The accepted-types list for the error message, e.g. `Array or String`.
    pub(crate) fn expected(&self) -> String {
        if self.types.is_empty() {
            return "Any".to_string();
        }
        self.types
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>()
            .join(" or ")
    }
}

/// Derive a native function's [`Arity`] from its parameter spec.
pub trait ParamSpec {
    fn arity(&self) -> Arity;
}

impl ParamSpec for [Param] {
    fn arity(&self) -> Arity {
        // Optionals must be trailing: a required param after an optional one can't be
        // matched positionally (which argument filled it?), so reject such specs.
        let required = self.iter().take_while(|p| !p.optional).count();
        debug_assert!(
            self[required..].iter().all(|p| p.optional),
            "invalid param spec: a required parameter follows an optional one (optionals must be trailing)",
        );
        let total = self.len();
        if required == total {
            Arity::Exact(total)
        } else {
            Arity::Between(required as u32, total as u32)
        }
    }
}
