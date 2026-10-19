//! SQLite databases for Frost scripts.
//!
//! A host installs the extension with [`extension`], and scripts import it as
//! `ext.sqlite`:
//!
//! ```
//! use frostlang::ImporterBuilder;
//!
//! let importer = ImporterBuilder::new()
//!     .with_extension(frostlang_sqlite::extension())
//!     .expect("no other extension is named `sqlite`")
//!     .build();
//! ```
//!
//! The extension is built on [`rusqlite`], re-exported here so a host names the
//! same version.
//!
//! # Linking SQLite
//!
//! The host chooses which SQLite library the extension uses, and so its version
//! and compile-time options.
//!
//! - **Bundled:** the `bundled` feature compiles SQLite from source into the
//!   build. It needs only a C compiler. `LIBSQLITE3_FLAGS` adds compile-time
//!   options, such as `LIBSQLITE3_FLAGS="-DSQLITE_MAX_ATTACHED=20"`.
//! - **System:** without `bundled`, the build links the SQLite that `pkg-config`
//!   finds (or `vcpkg`, with MSVC), and failing that, `-lsqlite3` from the
//!   linker's default search path.
//! - **Custom-built:** with `SQLITE3_LIB_DIR` set, the build links the
//!   `libsqlite3` in that directory. It links dynamically, or statically with
//!   `SQLITE3_STATIC=1`. `pkg-config` still takes precedence, and finds any
//!   system SQLite first. Either set `SQLITE3_NO_PKG_CONFIG=1`, or put a
//!   `pkgconfig/sqlite3.pc` in the directory, which can also list what a static
//!   library itself links against.
//!
//! A dynamically linked library must also be found at run time: on Linux, as
//! through `LD_LIBRARY_PATH`; on macOS, `DYLD_LIBRARY_PATH` or an rpath; on
//! Windows, the DLL beside the executable or on `PATH`.
//!
//! The build rechecks the environment variables above, but not the library
//! files. After replacing a static library in place, run
//! `cargo clean -p libsqlite3-sys`.
//!
//! `LIBSQLITE3_SYS_USE_PKG_CONFIG=1` links as without `bundled` even where it
//! is enabled.
//!
//! The `bundled` feature is off by default and should stay off in libraries:
//! Cargo features unify across a build, so a library enabling it would take the
//! choice away from every host depending on that library.
//!
//! Whatever the mode, these hold:
//!
//! - A build links exactly one SQLite through `libsqlite3-sys`. Every crate
//!   using it, such as `rusqlite`, `sqlx`, or `diesel`, must agree on its
//!   version, and any one of them enabling its `bundled` feature bundles SQLite
//!   for all. A host already using `rusqlite` uses the version this crate
//!   re-exports.
//! - Enabling `bundled` also enables API in [`rusqlite`] that needs a recent
//!   SQLite. Host code using it compiles only with `bundled` enabled, and the
//!   library linked must then be recent, even through
//!   `LIBSQLITE3_SYS_USE_PKG_CONFIG`.
//! - SQLite 3.34.1 is the oldest supported version. The build does not reliably
//!   reject an older one.
//! - SQLite must be built thread-safe (`SQLITE_THREADSAFE` of 1 or 2), and not
//!   configured single-threaded at run time. Opening a database with such a
//!   library is an error.

pub use rusqlite;

use frostlang::{Extension, Value};

/// The `ext.sqlite` extension.
///
/// - `version` is the version of the SQLite library in use, such as
///   `'3.53.2'`.
pub fn extension() -> Extension {
    Extension::builder("sqlite")
        .value("version", Value::from(rusqlite::version()))
        .build()
}
