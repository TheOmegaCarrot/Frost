//! Declarative argument specs for native functions.
//!
//! A [`Params`] is a validated sequence of [`Param`]s describing a native's
//! parameters: the [`Arity`] they imply, and the types each argument may have.
//!
//! Functions whose validity can't be expressed per-parameter (e.g. operators,
//! where `Int + Int` is fine but `Int + String` is not) skip this and validate in
//! their own body instead.

use std::borrow::Cow;
use std::fmt;

use enumset::EnumSet;

use crate::{Arity, FrostType, Value};

/// One parameter of a native function.
#[derive(Clone, Copy, Debug)]
pub struct Param {
    /// Surfaced in the error as `argument N (name)` when present; most params omit it.
    pub(crate) name: Option<&'static str>,
    /// Accepted types. [`FrostType::ANY`] accepts any value.
    types: EnumSet<FrostType>,
    optional: bool,
}

impl Param {
    /// A required parameter accepting the types in `types`: a single type
    /// (`FrostType::Function.into()`), a `|`-built set, or a named category.
    /// The set must be nonempty: a parameter that accepts no type fails every call,
    /// and [`Params::try_new`] rejects such a spec.
    /// Use [`Param::any`] instead to construct a `Param` that accepts any type,
    /// but passing [`FrostType::NONNULL`] to this constructor may more often be what you want.
    pub const fn of(types: EnumSet<FrostType>) -> Self {
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
            types: FrostType::ANY,
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
    /// An optional parameter may only appear before another optional parameter, or as the last
    /// parameter.
    pub const fn optional(self) -> Self {
        Self {
            name: self.name,
            types: self.types,
            optional: true,
        }
    }

    pub(crate) fn accepts(&self, value: &Value) -> bool {
        value.fits(self.types)
    }

    /// The accepted types for the error message: a named category (`Any`, `Numeric`, ...)
    /// when the set is one, else the list of type names, e.g. `Array or String`.
    pub(crate) fn expected(&self) -> String {
        match self.types {
            t if t == FrostType::ANY => "Any".to_string(),
            t if t == FrostType::NUMERIC => "Numeric".to_string(),
            t if t == FrostType::PRIMITIVE => "Primitive".to_string(),
            t if t == FrostType::STRUCTURED => "Structured".to_string(),
            t if t == FrostType::NONNULL => "Nonnull".to_string(),
            t => t.iter().map(|t| t.name()).collect::<Vec<_>>().join(" or "),
        }
    }
}

/// A complete parameter spec.
/// 
/// Construct with [`Params::new`] for specs written as literals, or
/// [`Params::try_new`] for specs built from runtime data.
#[derive(Clone, Debug)]
pub struct Params {
    params: Cow<'static, [Param]>,
    arity: Arity,
}

/// Why a parameter spec was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidParams {
    /// A required parameter appears after an optional one, so it cannot be
    /// matched positionally (which argument would fill it?).
    RequiredAfterOptional {
        /// Position of the offending required parameter.
        index: usize,
    },
    /// A parameter's type set is empty: it accepts no value, so every call fails.
    EmptyTypeSet {
        /// Position of the offending parameter.
        index: usize,
    },
}

impl fmt::Display for InvalidParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiredAfterOptional { index } => write!(
                f,
                "invalid param spec: required parameter at index {index} follows an optional one (optionals must be trailing)",
            ),
            Self::EmptyTypeSet { index } => write!(
                f,
                "invalid param spec: parameter at index {index} has an empty type set and accepts no value",
            ),
        }
    }
}

impl std::error::Error for InvalidParams {}

impl Params {
    /// Builds a spec from a `'static` slice, panicking if it is invalid
    /// (a required parameter follows an optional one, or a type set is empty).
    ///
    /// Being `const`, this can validate at compile time when it initializes a
    /// `const` item, turning an invalid spec into a compile error:
    ///
    /// ```
    /// # use frost_runtime::{Param, Params};
    /// const PARAMS: Params = Params::new(&[Param::any(), Param::any().optional()]);
    /// ```
    ///
    /// ```compile_fail
    /// # use frost_runtime::{Param, Params};
    /// // A required parameter after an optional one fails const evaluation.
    /// const PARAMS: Params = Params::new(&[Param::any().optional(), Param::any()]);
    /// ```
    ///
    /// ```compile_fail
    /// # use frost_runtime::{EnumSet, Param, Params};
    /// // An empty type set (a parameter accepting no value) fails const evaluation.
    /// const PARAMS: Params = Params::new(&[Param::of(EnumSet::empty())]);
    /// ```
    pub const fn new(params: &'static [Param]) -> Self {
        match Self::derive_arity(params) {
            Ok(arity) => Self {
                params: Cow::Borrowed(params),
                arity,
            },
            Err(InvalidParams::RequiredAfterOptional { .. }) => panic!(
                "invalid param spec: a required parameter follows an optional one (optionals must be trailing)"
            ),
            Err(InvalidParams::EmptyTypeSet { .. }) => {
                panic!("invalid param spec: a parameter's type set is empty and accepts no value")
            }
        }
    }

    /// Builds a spec from runtime-supplied parameters, rejecting an invalid one
    /// with the reason.
    ///
    /// Prefer [`Params::new`] for specs written as literals; this constructor is
    /// for specs assembled from runtime data, where rejection is a handleable
    /// [`InvalidParams`] rather than a panic.
    pub fn try_new(params: impl IntoIterator<Item = Param>) -> Result<Self, InvalidParams> {
        let params: Vec<Param> = params.into_iter().collect();
        let arity = Self::derive_arity(&params)?;
        Ok(Self {
            params: Cow::Owned(params),
            arity,
        })
    }

    /// The function arity this spec implies: `Exact` when all parameters are
    /// required, `Between` when some are optional.
    pub const fn arity(&self) -> Arity {
        self.arity
    }

    /// The parameters, in order.
    pub fn as_slice(&self) -> &[Param] {
        &self.params
    }

    /// Validation core shared by both constructors: every type set must be
    /// nonempty, and optionals must trail the required parameters.
    /// Returns the derived arity.
    const fn derive_arity(params: &[Param]) -> Result<Arity, InvalidParams> {
        let mut required = 0;
        let mut seen_optional = false;
        let mut i = 0;
        while i < params.len() {
            // Emptiness via the raw repr: the const-compatible spelling of
            // `is_empty` (which is not a const fn).
            if params[i].types.as_repr() == 0 {
                return Err(InvalidParams::EmptyTypeSet { index: i });
            }
            if params[i].optional {
                seen_optional = true;
            } else {
                if seen_optional {
                    return Err(InvalidParams::RequiredAfterOptional { index: i });
                }
                required += 1;
            }
            i += 1;
        }
        if required == params.len() {
            Ok(Arity::Exact(required))
        } else {
            Ok(Arity::Between(required, params.len()))
        }
    }
}
