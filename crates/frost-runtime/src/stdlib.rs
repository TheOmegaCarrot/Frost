//! The modules of Frost's standard library, one constructor each.
//!
//! A host chooses which modules its scripts may import, and adds each to a
//! [`Stdlib`](crate::Stdlib):
//!
//! ```
//! use frost_runtime::{ImporterBuilder, Stdlib, stdlib};
//!
//! let stdlib = Stdlib::new().with_module(stdlib::encoding())?;
//! let importer = ImporterBuilder::new().with_stdlib(stdlib).build();
//! # Ok::<(), frost_runtime::StdlibError>(())
//! ```

mod encoding;

pub use encoding::encoding;
