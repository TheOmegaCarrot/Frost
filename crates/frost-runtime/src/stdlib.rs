//! The modules of Frost's standard library, one constructor each, and the
//! [`Stdlib`] presets that gather them.
//!
//! A host chooses which modules its scripts may import. A preset is the usual
//! start, and modules may still be added to it:
//!
//! ```
//! use frost_runtime::stdlib::RandomConfig;
//! use frost_runtime::{ImporterBuilder, Stdlib};
//!
//! let stdlib = Stdlib::contained(RandomConfig::default());
//! let importer = ImporterBuilder::new().with_stdlib(stdlib).build();
//! ```
//!
//! Or the modules can be picked one by one:
//!
//! ```
//! use frost_runtime::{ImporterBuilder, Stdlib, stdlib};
//!
//! let stdlib = Stdlib::new().with_module(stdlib::encoding())?;
//! let importer = ImporterBuilder::new().with_stdlib(stdlib).build();
//! # Ok::<(), frost_runtime::StdlibError>(())
//! ```

mod encoding;
mod math;
mod os;
mod random;
mod regex;
mod string;

pub use encoding::encoding;
pub use math::math;
pub use os::os;
pub use random::{RandomConfig, random};
pub use regex::regex;
pub use string::string;

use crate::{Stdlib, StdlibModule};

impl Stdlib {
    /// Every module contained within the script: nothing a script does with
    /// them reads or changes anything outside it, such as files, the
    /// environment, other processes, or the clock. Each configurable module is
    /// configured by its argument.
    ///
    /// Containment is not a security boundary: a contained script can still
    /// exhaust memory or run forever.
    ///
    /// Includes [`encoding`], [`math`], [`random`], [`regex`], and [`string`].
    pub fn contained(random_config: RandomConfig) -> Self {
        Self::new().with_modules([encoding(), math(), random(random_config), regex(), string()])
    }

    /// Every module, including those that reach outside the script. Each
    /// configurable module is configured by its argument.
    ///
    /// Includes everything in [`contained`](Self::contained), and [`os`].
    pub fn complete(random_config: RandomConfig) -> Self {
        Self::contained(random_config).with_modules([os()])
    }

    /// This library with `modules` added, whose names must be new to it.
    fn with_modules(self, modules: impl IntoIterator<Item = StdlibModule>) -> Self {
        modules.into_iter().fold(self, |stdlib, module| {
            stdlib
                .with_module(module)
                .expect("a preset's modules have distinct names")
        })
    }
}
