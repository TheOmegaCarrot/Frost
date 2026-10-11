//! [`NativeDef`]: a native function defined, not yet built.

use std::borrow::Cow;
use std::fmt;

use crate::Value;

/// A native function that [`#[frost_fn]`](crate::frost_fn) defines, built into
/// a [`Value`] once its name is known.
///
/// [`build`](Self::build) names it by its [`base_name`](Self::base_name), and
/// [`build_named`](Self::build_named) by any name.
/// [`ComponentBuilder::function`](crate::ComponentBuilder::function) names it by
/// where it is placed. Each build makes a new function.
///
/// ```
/// use frostlang::frost_fn;
///
/// #[frost_fn]
/// fn double(x: i64) -> i64 {
///     x * 2
/// }
///
/// let definition = double_frost();
/// assert_eq!(definition.base_name(), "double");
/// let double = definition.build();
/// let also_double = definition.build_named("tools.double");
/// ```
#[derive(Clone, Copy)]
pub struct NativeDef {
    base_name: &'static str,
    make: fn(Cow<'static, str>) -> Value,
}

impl NativeDef {
    /// A definition whose function `make` builds under a given name.
    pub(crate) const fn new(base_name: &'static str, make: fn(Cow<'static, str>) -> Value) -> Self {
        Self { base_name, make }
    }

    /// The name the function goes by on its own, and its key where it is
    /// placed: a Frost identifier, not a keyword.
    pub const fn base_name(&self) -> &'static str {
        self.base_name
    }

    /// The function, named by its [`base_name`](Self::base_name).
    pub fn build(self) -> Value {
        (self.make)(Cow::Borrowed(self.base_name))
    }

    /// The function, named `name`, as reported in errors and backtraces.
    pub fn build_named(self, name: impl Into<Cow<'static, str>>) -> Value {
        (self.make)(name.into())
    }
}

impl fmt::Debug for NativeDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeDef")
            .field("base_name", &self.base_name)
            .finish_non_exhaustive()
    }
}
