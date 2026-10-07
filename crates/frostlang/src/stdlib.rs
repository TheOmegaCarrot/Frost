//! The modules of Frost's standard library, one constructor each, and the
//! [`Stdlib`] presets that gather them.
//!
//! A host chooses which modules its scripts may import. A preset is the usual
//! start, and modules may still be added to it:
//!
//! ```
//! use frostlang::stdlib::StdlibConfig;
//! use frostlang::{ImporterBuilder, Stdlib};
//!
//! let stdlib = Stdlib::contained(StdlibConfig::default());
//! let importer = ImporterBuilder::new().with_stdlib(stdlib).build();
//! ```
//!
//! Or the modules can be picked one by one:
//!
//! ```
//! use frostlang::{ImporterBuilder, Stdlib, stdlib};
//!
//! let stdlib = Stdlib::new().with_module(stdlib::encoding())?;
//! let importer = ImporterBuilder::new().with_stdlib(stdlib).build();
//! # Ok::<(), frostlang::StdlibError>(())
//! ```

mod encoding;
mod fs;
mod math;
mod os;
mod random;
mod regex;
mod stream;
mod string;

pub use encoding::encoding;
pub use fs::fs;
pub use math::math;
pub use os::os;
pub use random::{RandomConfig, random};
pub use regex::regex;
pub use string::string;

use crate::{Stdlib, StdlibModule};

/// The configuration of every configurable module, for the [`Stdlib`] presets.
///
/// The default suits most hosts.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct StdlibConfig {
    /// The configuration of [`random`].
    pub random: RandomConfig,
}

impl StdlibConfig {
    /// This configuration with [`random`](Self::random) set to `config`.
    #[must_use]
    pub fn with_random(mut self, config: RandomConfig) -> Self {
        self.random = config;
        self
    }
}

impl Stdlib {
    /// Every module contained within the script: nothing a script does with
    /// them reads or changes anything outside it, such as files, the
    /// environment, other processes, or the clock. Each configurable module is
    /// configured as `config` says.
    ///
    /// Containment is not a security boundary: a contained script can still
    /// exhaust memory or run forever.
    ///
    /// Includes [`encoding`], [`math`], [`random`], [`regex`], and [`string`].
    pub fn contained(config: StdlibConfig) -> Self {
        Self::new().with_modules([encoding(), math(), random(config.random), regex(), string()])
    }

    /// Every module, including those that reach outside the script. Each
    /// configurable module is configured as `config` says.
    ///
    /// Includes everything in [`contained`](Self::contained), and [`fs`] and [`os`].
    pub fn complete(config: StdlibConfig) -> Self {
        Self::contained(config).with_modules([fs(), os()])
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
