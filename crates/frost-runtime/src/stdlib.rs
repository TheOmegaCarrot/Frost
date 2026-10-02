//! The modules of Frost's standard library, one constructor each, and the
//! [`Stdlib`] presets that gather them.
//!
//! A host chooses which modules its scripts may import. A preset is the usual
//! start, and modules may still be added to it:
//!
//! ```
//! use frost_runtime::{ImporterBuilder, Stdlib};
//!
//! let importer = ImporterBuilder::new().with_stdlib(Stdlib::pure()).build();
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
mod os;

pub use encoding::encoding;
pub use os::os;

use crate::{Stdlib, StdlibModule};

impl Stdlib {
    /// Every module that only computes: none reads or changes anything outside
    /// the script, such as files, the environment, or the clock.
    ///
    /// Includes [`encoding`].
    pub fn pure() -> Self {
        Self::of([encoding()])
    }

    /// Every module, including those with effects outside the script.
    ///
    /// Includes everything in [`pure`](Self::pure), and [`os`].
    pub fn complete() -> Self {
        Self::pure()
            .with_module(os())
            .expect("the pure preset holds no effectful module")
    }

    /// A library of `modules`, whose names must be distinct.
    fn of(modules: impl IntoIterator<Item = StdlibModule>) -> Self {
        modules.into_iter().fold(Self::new(), |stdlib, module| {
            stdlib
                .with_module(module)
                .expect("a preset's modules have distinct names")
        })
    }
}
