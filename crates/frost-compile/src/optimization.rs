//! Which optimizations the compiler applies, and how tools name and set them.

#[cfg(test)]
mod tests;

use std::fmt;

/// Which optimizations the compiler applies. None changes what a program
/// computes, only how it computes it.
///
/// Start from a preset, [`ALL`](Self::ALL) or [`NONE`](Self::NONE), and adjust
/// it with [`with`](Self::with), [`set`](Self::set), or
/// [`with_settings`](Self::with_settings). More optimizations may be added, so
/// these options cannot be built field by field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OptimizationOptions {
    /// Evaluate an expression built only from compile-time-known values and pure
    /// operations at compile time, emitting just its value.
    pub constant_fold: bool,
    /// Propagate a binding whose value is compile-time known, so a lookup of it
    /// is itself fold-eligible. The lookup loads the value directly, unless it
    /// cannot be a constant, as a function cannot: then the lookup still loads
    /// the binding, but a [constant fold](Self::constant_fold) can use the
    /// value, as by calling the function. A value that only a fold can compute,
    /// rather than a literal, is known only alongside
    /// [`constant_fold`](Self::constant_fold).
    pub constant_propagate: bool,
    /// Resolve a branch whose condition is compile-time known, emitting only the
    /// path taken.
    pub branch_eliminate: bool,
    /// Build a captured value that is compile-time known into the capturing
    /// function itself, rather than passing it in each time a closure is
    /// created. A captured binding's value is known only through
    /// [`constant_propagate`](Self::constant_propagate), so this has effect only
    /// alongside it.
    pub capture_hoist: bool,
    /// Discard a binding's value rather than store it, when nothing reads the
    /// binding afterward. The value is still computed; with
    /// [`discard_eliminate`](Self::discard_eliminate), one that takes no
    /// computation, such as a constant, is not. An exported binding is always
    /// stored.
    ///
    /// A binding the script does read can still go unread once compiled: with
    /// [`constant_propagate`](Self::constant_propagate), a lookup of a known
    /// constant loads the constant instead, and with
    /// [`constant_fold`](Self::constant_fold) as well, an expression using a
    /// known value, such as a call to a known function, becomes its result.
    pub dead_store_eliminate: bool,
    /// Skip loading a value that would only be discarded, when loading it takes
    /// no computation: a constant, a binding's value, or a function that
    /// captures nothing. Such a value is discarded when a statement's value goes
    /// unused, or, with [`dead_store_eliminate`](Self::dead_store_eliminate),
    /// when a binding is never read.
    pub discard_eliminate: bool,
    /// Move a local's value out on its last use, rather than copying it, so a
    /// structure held only by that local can be updated in place.
    pub consume_locals: bool,
    /// Store identical constants once per function, however many places use
    /// them, making the compiled program smaller.
    pub deduplicate_constants: bool,
}

impl OptimizationOptions {
    /// Every optimization off: the program compiles as written.
    pub const NONE: Self = Self {
        constant_fold: false,
        constant_propagate: false,
        branch_eliminate: false,
        capture_hoist: false,
        dead_store_eliminate: false,
        discard_eliminate: false,
        consume_locals: false,
        deduplicate_constants: false,
    };

    /// Every optimization on.
    pub const ALL: Self = Self {
        constant_fold: true,
        constant_propagate: true,
        branch_eliminate: true,
        capture_hoist: true,
        dead_store_eliminate: true,
        discard_eliminate: true,
        consume_locals: true,
        deduplicate_constants: true,
    };

    /// Whether `optimization` is on.
    pub const fn get(self, optimization: Optimization) -> bool {
        let mut options = self;
        *options.flag(optimization)
    }

    /// Turn `optimization` on or off.
    pub const fn set(&mut self, optimization: Optimization, on: bool) {
        *self.flag(optimization) = on;
    }

    /// These options with `optimization` turned on or off.
    ///
    /// ```
    /// use frost_compile::{Optimization, OptimizationOptions};
    ///
    /// const FOLD_ONLY: OptimizationOptions =
    ///     OptimizationOptions::NONE.with(Optimization::ConstantFold, true);
    /// assert!(FOLD_ONLY.get(Optimization::ConstantFold));
    /// assert!(!FOLD_ONLY.get(Optimization::ConsumeLocals));
    /// ```
    #[must_use]
    pub const fn with(mut self, optimization: Optimization, on: bool) -> Self {
        self.set(optimization, on);
        self
    }

    /// These options with `settings` applied, as tools let a person write
    /// them: comma-separated, applied left to right, each one of
    ///
    /// - `<name>=true` or `<name>=false`, for the optimization with that
    ///   [name](Optimization::name);
    /// - `preset=all` or `preset=none`, or just `all` or `none`, for
    ///   [`ALL`](Self::ALL) or [`NONE`](Self::NONE).
    ///
    /// Spaces around each part are ignored. If any setting is invalid, none is
    /// applied.
    ///
    /// ```
    /// use frost_compile::{Optimization, OptimizationOptions};
    ///
    /// let options = OptimizationOptions::ALL.with_settings("none, constant-fold=true").unwrap();
    /// assert!(options.get(Optimization::ConstantFold));
    /// assert!(!options.get(Optimization::ConsumeLocals));
    /// ```
    pub fn with_settings(self, settings: &str) -> Result<Self, InvalidOptimizationSetting> {
        settings.split(',').try_fold(self, |mut options, setting| {
            let setting = setting.trim();
            let Some((name, value)) = setting.split_once('=') else {
                return match setting {
                    "all" => Ok(Self::ALL),
                    "none" => Ok(Self::NONE),
                    _ => Err(InvalidOptimizationSetting::Malformed(setting.to_string())),
                };
            };
            let (name, value) = (name.trim(), value.trim());
            if name == "preset" {
                return match value {
                    "all" => Ok(Self::ALL),
                    "none" => Ok(Self::NONE),
                    _ => Err(InvalidOptimizationSetting::UnknownPreset(value.to_string())),
                };
            }
            let optimization = Optimization::from_name(name)
                .ok_or_else(|| InvalidOptimizationSetting::UnknownOptimization(name.to_string()))?;
            let on = match value {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(InvalidOptimizationSetting::InvalidValue {
                        optimization,
                        value: value.to_string(),
                    });
                }
            };
            options.set(optimization, on);
            Ok(options)
        })
    }

    const fn flag(&mut self, optimization: Optimization) -> &mut bool {
        match optimization {
            Optimization::ConstantFold => &mut self.constant_fold,
            Optimization::ConstantPropagate => &mut self.constant_propagate,
            Optimization::BranchEliminate => &mut self.branch_eliminate,
            Optimization::CaptureHoist => &mut self.capture_hoist,
            Optimization::DeadStoreEliminate => &mut self.dead_store_eliminate,
            Optimization::DiscardEliminate => &mut self.discard_eliminate,
            Optimization::ConsumeLocals => &mut self.consume_locals,
            Optimization::DeduplicateConstants => &mut self.deduplicate_constants,
        }
    }
}

/// One of the optimizations [`OptimizationOptions`] turns on or off, each
/// described on its field there. More may be added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Optimization {
    /// [`OptimizationOptions::constant_fold`].
    ConstantFold,
    /// [`OptimizationOptions::constant_propagate`].
    ConstantPropagate,
    /// [`OptimizationOptions::branch_eliminate`].
    BranchEliminate,
    /// [`OptimizationOptions::capture_hoist`].
    CaptureHoist,
    /// [`OptimizationOptions::dead_store_eliminate`].
    DeadStoreEliminate,
    /// [`OptimizationOptions::discard_eliminate`].
    DiscardEliminate,
    /// [`OptimizationOptions::consume_locals`].
    ConsumeLocals,
    /// [`OptimizationOptions::deduplicate_constants`].
    DeduplicateConstants,
}

impl Optimization {
    /// Every optimization, in the order [`OptimizationOptions`] declares them.
    pub const ALL: &[Self] = &[
        Self::ConstantFold,
        Self::ConstantPropagate,
        Self::BranchEliminate,
        Self::CaptureHoist,
        Self::DeadStoreEliminate,
        Self::DiscardEliminate,
        Self::ConsumeLocals,
        Self::DeduplicateConstants,
    ];

    /// The name tools give it: its field's name in kebab-case, as
    /// `constant-fold`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ConstantFold => "constant-fold",
            Self::ConstantPropagate => "constant-propagate",
            Self::BranchEliminate => "branch-eliminate",
            Self::CaptureHoist => "capture-hoist",
            Self::DeadStoreEliminate => "dead-store-eliminate",
            Self::DiscardEliminate => "discard-eliminate",
            Self::ConsumeLocals => "consume-locals",
            Self::DeduplicateConstants => "deduplicate-constants",
        }
    }

    /// The optimization with the [`name`](Self::name) `name`, if any.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|optimization| optimization.name() == name)
    }
}

/// A setting [`OptimizationOptions::with_settings`] refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidOptimizationSetting {
    /// Neither `<name>=<value>`, `all`, nor `none`.
    Malformed(String),
    /// No optimization has this name.
    UnknownOptimization(String),
    /// An optimization set to something other than `true` or `false`.
    InvalidValue {
        /// The optimization.
        optimization: Optimization,
        /// What it was set to.
        value: String,
    },
    /// `preset` set to something other than `all` or `none`.
    UnknownPreset(String),
}

impl fmt::Display for InvalidOptimizationSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(setting) => write!(
                f,
                "`{setting}` should be `<optimization>=true|false`, `preset=all|none`, `all`, or `none`"
            ),
            Self::UnknownOptimization(name) => {
                let names: Vec<&str> = Optimization::ALL.iter().map(|o| o.name()).collect();
                write!(
                    f,
                    "there is no optimization `{name}`; there are {}",
                    names.join(", ")
                )
            }
            Self::InvalidValue {
                optimization,
                value,
            } => write!(
                f,
                "`{}` is `true` or `false`, not `{value}`",
                optimization.name()
            ),
            Self::UnknownPreset(value) => write!(f, "`preset` is `all` or `none`, not `{value}`"),
        }
    }
}

impl std::error::Error for InvalidOptimizationSetting {}
