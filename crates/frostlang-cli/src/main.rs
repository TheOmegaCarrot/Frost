//! `frost`: the reference Frost command-line interface.

use std::env;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use frostlang_driver::{Driver, Exit};
use frostlang_fs_resolver::{DiagnosticStyle, FsResolver};
use frostlang_runtime::{
    Importer, ImporterBuilder, Stdlib, VmRuntimeConfiguration, stdlib::RandomConfig,
};

/// How deeply imports may nest: far deeper than a sound module tree needs, and
/// shallow enough that runaway nesting fails before the native stack runs out.
const MAX_IMPORT_DEPTH: NonZeroUsize = NonZeroUsize::new(64).unwrap();

fn main() -> Exit {
    Driver::new()
        .with_name("frost")
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_configuration(
            VmRuntimeConfiguration::default().with_max_import_depth(Some(MAX_IMPORT_DEPTH)),
        )
        .with_importer_for(importer)
        .run_from_env()
}

/// Everything a script may import: the complete standard library, and Frost
/// files under the directory of `script`, if there is one, the working
/// directory, and each directory in `FROST_MODULE_PATH`, searched in that order.
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
    let importer = ImporterBuilder::new().with_stdlib(Stdlib::complete(RandomConfig::default()));
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
