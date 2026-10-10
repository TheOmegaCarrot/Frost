//! `frost`: the reference Frost command-line interface.

use std::env;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use frostlang::{Importer, ImporterBuilder, Stdlib, VmRuntimeConfiguration, stdlib::StdlibConfig};
use frostlang_driver::{Driver, Exit};
use frostlang_fs_resolver::{DiagnosticStyle, FsResolver};

/// How deeply imports may nest: far deeper than a sound module tree needs, and
/// shallow enough that runaway nesting fails before the native stack runs out.
const MAX_IMPORT_DEPTH: NonZeroUsize = NonZeroUsize::new(64).unwrap();

/// How deeply calls may nest: far deeper than any sound recursion needs, and
/// shallow enough that runaway recursion fails before it exhausts memory.
const MAX_CALL_DEPTH: NonZeroUsize = NonZeroUsize::new(1_000_000).unwrap();

fn main() -> Exit {
    Driver::new()
        .with_name("frost")
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_configuration(
            VmRuntimeConfiguration::default()
                .with_max_import_depth(Some(MAX_IMPORT_DEPTH))
                .with_max_call_depth(Some(MAX_CALL_DEPTH)),
        )
        .with_importer_for(importer)
        .run_from_env()
}

/// Everything a script may import: the complete standard library, every
/// first-party extension, and Frost files under the directory of `script`, if
/// there is one, the working directory, and each directory in
/// `FROST_MODULE_PATH`, searched in that order.
fn importer(script: Option<&Path>) -> Arc<Importer> {
    let script_dir = script.map(|script| match script.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
        _ => PathBuf::from("."),
    });
    let module_path: Vec<PathBuf> = env::var_os("FROST_MODULE_PATH")
        .map(|paths| env::split_paths(&paths).collect())
        .unwrap_or_default();
    // As with `PATH`, a directory that does not exist is skipped.
    let roots: Vec<PathBuf> = script_dir
        .into_iter()
        .chain([PathBuf::from(".")])
        .chain(module_path)
        .filter(|root| root.is_dir())
        .collect();
    let importer = ImporterBuilder::new()
        .with_stdlib(Stdlib::complete(StdlibConfig::default()))
        .with_extension(frostlang_uuid::extension())
        .expect("each extension has its own name");
    match FsResolver::new(roots) {
        Ok(resolver) => importer.append_resolver(Arc::new(
            resolver
                .with_symlinks_followed(true)
                .with_diagnostics(DiagnosticStyle::Unicode),
        )),
        // Only a root removed since it was checked fails here; the standard
        // library is still importable.
        Err(_) => importer,
    }
    .build()
}
