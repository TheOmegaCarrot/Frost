//! Which optimizations the compiler applies, and how tools name and set them.

use std::fmt;

/// Which optimizations the compiler applies. None changes what a program
/// computes, only how it computes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptimizationOptions {
    /// Evaluate an expression built only from compile-time-known values and pure
    /// operations at compile time, emitting just its value.
    pub constant_fold: bool,
    /// Propagate a binding whose value is compile-time known: a lookup of it loads
    /// the value directly, so it is itself fold-eligible.
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
        consume_locals: false,
        deduplicate_constants: false,
    };

    /// Every optimization on.
    pub const ALL: Self = Self {
        constant_fold: true,
        constant_propagate: true,
        branch_eliminate: true,
        capture_hoist: true,
        consume_locals: true,
        deduplicate_constants: true,
    };

    /// Whether `optimization` is on.
    pub fn get(self, optimization: Optimization) -> bool {
        let mut options = self;
        *options.flag(optimization)
    }

    /// Turn `optimization` on or off.
    pub fn set(&mut self, optimization: Optimization, on: bool) {
        *self.flag(optimization) = on;
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

    fn flag(&mut self, optimization: Optimization) -> &mut bool {
        match optimization {
            Optimization::ConstantFold => &mut self.constant_fold,
            Optimization::ConstantPropagate => &mut self.constant_propagate,
            Optimization::BranchEliminate => &mut self.branch_eliminate,
            Optimization::CaptureHoist => &mut self.capture_hoist,
            Optimization::ConsumeLocals => &mut self.consume_locals,
            Optimization::DeduplicateConstants => &mut self.deduplicate_constants,
        }
    }
}

/// One of the optimizations [`OptimizationOptions`] turns on or off, each
/// described on its field there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Optimization {
    /// [`OptimizationOptions::constant_fold`].
    ConstantFold,
    /// [`OptimizationOptions::constant_propagate`].
    ConstantPropagate,
    /// [`OptimizationOptions::branch_eliminate`].
    BranchEliminate,
    /// [`OptimizationOptions::capture_hoist`].
    CaptureHoist,
    /// [`OptimizationOptions::consume_locals`].
    ConsumeLocals,
    /// [`OptimizationOptions::deduplicate_constants`].
    DeduplicateConstants,
}

impl Optimization {
    /// Every optimization, in the order [`OptimizationOptions`] declares them.
    pub const ALL: [Self; 6] = [
        Self::ConstantFold,
        Self::ConstantPropagate,
        Self::BranchEliminate,
        Self::CaptureHoist,
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
            Self::ConsumeLocals => "consume-locals",
            Self::DeduplicateConstants => "deduplicate-constants",
        }
    }

    /// The optimization with the [`name`](Self::name) `name`, if any.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
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
                let names: Vec<&str> = Optimization::ALL.map(Optimization::name).to_vec();
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
