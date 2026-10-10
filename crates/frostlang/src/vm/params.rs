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
use std::ops::Range;

use enumset::EnumSet;

use crate::{Arity, FrostType, Value};

/// One parameter of a native function.
#[derive(Clone, Copy, Debug)]
pub struct Param {
    /// Surfaced in the error as `argument N (name)` when present; most params omit it.
    pub(crate) name: Option<&'static str>,
    /// Accepted types. [`FrostType::ANY`] accepts any value.
    types: EnumSet<FrostType>,
    kind: Kind,
}

/// How many arguments a parameter takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Exactly one.
    Required,
    /// One, or none when the call omits it.
    Optional,
    /// Every remaining argument, possibly none.
    Rest,
}

impl Param {
    /// A required parameter accepting the types in `types`: a single type
    /// (`FrostType::Function.into()`), a `|`-built set, or a named category.
    /// The set must be nonempty; [`Params`] construction rejects an empty one.
    /// Use [`Param::any`] instead to construct a `Param` that accepts any type,
    /// but passing [`FrostType::NONNULL`] to this constructor may more often be what you want.
    pub const fn of(types: EnumSet<FrostType>) -> Self {
        Self {
            name: None,
            types,
            kind: Kind::Required,
        }
    }

    /// A required parameter accepting any value.
    pub const fn any() -> Self {
        Self::of(FrostType::ANY)
    }

    /// Attach a name, surfaced in the error message.
    pub const fn named(self, name: &'static str) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }

    /// Make this parameter optional: a call may omit it.
    /// See [`Params`] for where optional parameters may appear, and which
    /// receive arguments.
    pub const fn optional(self) -> Self {
        Self {
            kind: Kind::Optional,
            ..self
        }
    }

    /// Make this a rest parameter: it takes every argument after those of the
    /// parameters before it, each of its types, and possibly none.
    /// A rest parameter must be the last.
    pub const fn rest(self) -> Self {
        Self {
            kind: Kind::Rest,
            ..self
        }
    }

    /// The parameter's name, when it has one.
    pub const fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// The set of types this parameter accepts.
    pub const fn types(&self) -> EnumSet<FrostType> {
        self.types
    }

    /// Whether this parameter may be omitted.
    pub const fn is_optional(&self) -> bool {
        matches!(self.kind, Kind::Optional)
    }

    /// Whether this is a rest parameter.
    pub const fn is_rest(&self) -> bool {
        matches!(self.kind, Kind::Rest)
    }

    pub(crate) fn accepts(&self, value: &Value) -> bool {
        value.fits(self.types)
    }
}

/// `types` for an error message: a named category (`Any`, `Numeric`, ...)
/// when the set is one, else the list of type names, e.g. `String or Array`.
pub(crate) fn expected_types(types: EnumSet<FrostType>) -> String {
    match types {
        t if t == FrostType::ANY => "Any".to_string(),
        t if t == FrostType::NUMERIC => "Numeric".to_string(),
        t if t == FrostType::PRIMITIVE => "Primitive".to_string(),
        t if t == FrostType::STRUCTURED => "Structured".to_string(),
        t if t == FrostType::NONNULL => "Nonnull".to_string(),
        t => t.iter().map(|t| t.name()).collect::<Vec<_>>().join(" or "),
    }
}

/// A complete parameter spec.
///
/// Optional parameters form one contiguous group, anywhere in the spec, such
/// as `b` and `c` in `(a, b?, c?, d)`. Which of them receive arguments
/// depends on the number of arguments alone, never on their types: those
/// beyond the required parameters' fill the group strictly left to right, so
/// with three arguments, `b` receives one and `c` is omitted.
///
/// A spec may instead end in one rest parameter, which takes every argument
/// after the others'. A spec does not have both optionals and a rest.
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
#[non_exhaustive]
pub enum InvalidParams {
    /// A parameter's type set is empty: it accepts no value, so every call fails.
    EmptyTypeSet {
        /// Position of the offending parameter.
        index: usize,
    },
    /// An optional parameter is apart from the earlier ones, so which
    /// receives an argument would be unclear.
    OptionalsNotContiguous {
        /// Position of the first optional parameter of the second group.
        index: usize,
    },
    /// A rest parameter is not the last parameter.
    RestNotLast {
        /// Position of the rest parameter.
        index: usize,
    },
    /// The spec has both optional parameters and a rest parameter, so which
    /// takes an argument would be unclear.
    OptionalsAndRest {
        /// Position of the rest parameter.
        index: usize,
    },
}

impl fmt::Display for InvalidParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTypeSet { index } => write!(
                f,
                "invalid param spec: parameter at index {index} has an empty type set and accepts no value",
            ),
            Self::OptionalsNotContiguous { index } => write!(
                f,
                "invalid param spec: optional parameter at index {index} is apart from the earlier optionals (optionals must be contiguous)",
            ),
            Self::RestNotLast { index } => write!(
                f,
                "invalid param spec: rest parameter at index {index} is not the last parameter",
            ),
            Self::OptionalsAndRest { index } => write!(
                f,
                "invalid param spec: rest parameter at index {index} follows optional parameters (a spec has optionals or a rest, not both)",
            ),
        }
    }
}

impl std::error::Error for InvalidParams {}

impl Params {
    /// Builds a spec from a `'static` slice, panicking if it is invalid
    /// (see [`InvalidParams`]).
    ///
    /// Being `const`, this can validate at compile time when it initializes a
    /// `const` item, turning an invalid spec into a compile error:
    ///
    /// ```
    /// # use frostlang::{FrostType, Param, Params};
    /// const PARAMS: Params = Params::new(&[
    ///     Param::any(),
    ///     Param::any().optional(),
    ///     Param::of(FrostType::FUNCTION),
    /// ]);
    /// const VARIADIC: Params = Params::new(&[Param::any(), Param::any().rest()]);
    /// ```
    ///
    /// ```compile_fail
    /// # use frostlang::{EnumSet, Param, Params};
    /// // An empty type set (a parameter accepting no value) fails const evaluation.
    /// const PARAMS: Params = Params::new(&[Param::of(EnumSet::empty())]);
    /// ```
    ///
    /// ```compile_fail
    /// # use frostlang::{Param, Params};
    /// // Two separate groups of optionals fail const evaluation.
    /// const PARAMS: Params =
    ///     Params::new(&[Param::any().optional(), Param::any(), Param::any().optional()]);
    /// ```
    ///
    /// ```compile_fail
    /// # use frostlang::{Param, Params};
    /// // A rest parameter before another fails const evaluation.
    /// const PARAMS: Params = Params::new(&[Param::any().rest(), Param::any()]);
    /// ```
    ///
    /// ```compile_fail
    /// # use frostlang::{Param, Params};
    /// // Optionals with a rest fail const evaluation.
    /// const PARAMS: Params = Params::new(&[Param::any().optional(), Param::any().rest()]);
    /// ```
    pub const fn new(params: &'static [Param]) -> Self {
        match Self::derive_arity(params) {
            Ok(arity) => Self {
                params: Cow::Borrowed(params),
                arity,
            },
            Err(InvalidParams::EmptyTypeSet { .. }) => {
                panic!("invalid param spec: a parameter's type set is empty and accepts no value")
            }
            Err(InvalidParams::OptionalsNotContiguous { .. }) => {
                panic!("invalid param spec: optional parameters must be contiguous")
            }
            Err(InvalidParams::RestNotLast { .. }) => {
                panic!("invalid param spec: a rest parameter must be the last parameter")
            }
            Err(InvalidParams::OptionalsAndRest { .. }) => {
                panic!("invalid param spec: a spec has optionals or a rest, not both")
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
    /// required, `Between` when some are optional, and `AtLeast` when it ends
    /// in a rest parameter.
    pub const fn arity(&self) -> Arity {
        self.arity
    }

    /// The parameters, in order.
    pub fn as_slice(&self) -> &[Param] {
        &self.params
    }

    /// The number of required parameters.
    pub(crate) const fn required(&self) -> usize {
        match self.arity {
            Arity::Exact(required) | Arity::Between(required, _) | Arity::AtLeast(required) => {
                required
            }
        }
    }

    /// Validates the spec and derives its arity.
    const fn derive_arity(params: &[Param]) -> Result<Arity, InvalidParams> {
        let mut required = 0;
        let mut optionals = 0;
        // Whether a required parameter has followed the optionals seen so far,
        // which closes their group.
        let mut optionals_closed = false;
        let mut i = 0;
        while i < params.len() {
            // Emptiness via the raw repr: `is_empty` is not a const fn.
            if params[i].types.as_repr() == 0 {
                return Err(InvalidParams::EmptyTypeSet { index: i });
            }
            match params[i].kind {
                Kind::Required => {
                    required += 1;
                    optionals_closed = optionals > 0;
                }
                Kind::Optional => {
                    if optionals_closed {
                        return Err(InvalidParams::OptionalsNotContiguous { index: i });
                    }
                    optionals += 1;
                }
                Kind::Rest => {
                    if i + 1 != params.len() {
                        return Err(InvalidParams::RestNotLast { index: i });
                    }
                    if optionals > 0 {
                        return Err(InvalidParams::OptionalsAndRest { index: i });
                    }
                    return Ok(Arity::AtLeast(required));
                }
            }
            i += 1;
        }
        if optionals == 0 {
            Ok(Arity::Exact(required))
        } else {
            Ok(Arity::Between(required, required + optionals))
        }
    }
}

/// Which arguments of a call each parameter of a spec takes, parameter by
/// parameter, by the rules on [`Params`].
#[derive(Debug)]
pub(crate) struct Binder {
    argc: usize,
    /// The index of the next argument to bind.
    next: usize,
    /// How many of the optional parameters still to bind receive an argument.
    optionals_filled: usize,
}

impl Binder {
    /// A binder for a call to a native with `params`, given `argc` arguments.
    pub(crate) fn new(params: &Params, argc: usize) -> Self {
        Self {
            argc,
            next: 0,
            optionals_filled: argc.saturating_sub(params.required()),
        }
    }

    /// The indices of the arguments `param`, the next parameter of the spec,
    /// takes. A required parameter's index may be past the arguments when the
    /// call's arity has not been checked.
    pub(crate) fn bind(&mut self, param: &Param) -> Range<usize> {
        let start = self.next;
        self.next = match param.kind {
            Kind::Required => start + 1,
            Kind::Optional if self.optionals_filled > 0 => {
                self.optionals_filled -= 1;
                start + 1
            }
            Kind::Optional => start,
            Kind::Rest => self.argc.max(start),
        };
        start..self.next
    }
}
