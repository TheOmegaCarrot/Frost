//! Finding every module file under the root.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use frostlang::is_identifier_like;

/// A module file found by a scan.
pub(super) struct Found {
    pub(super) path: PathBuf,
    /// The file's text, or why it could not be read.
    pub(super) source: Result<Arc<str>, String>,
}

/// Every module file under `root`, by module name: `a/b.frst` is `a.b`.
///
/// Only directories and files whose names `import` could spell are visited,
/// and symbolic links are skipped, so the tree scanned is the tree watched.
/// A directory that cannot be listed is skipped too: its modules are absent
/// until it can be.
pub(super) fn scan(root: &Path) -> BTreeMap<String, Found> {
    let mut found = BTreeMap::new();
    visit(root, &mut Vec::new(), &mut found);
    found
}

fn visit(dir: &Path, prefix: &mut Vec<String>, found: &mut BTreeMap<String, Found>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if let Some(name) = path.file_name().and_then(|name| name.to_str())
                && is_identifier_like(name)
            {
                prefix.push(name.to_owned());
                visit(&path, prefix, found);
                prefix.pop();
            }
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "frst")
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            && is_identifier_like(stem)
        {
            let name = prefix
                .iter()
                .map(String::as_str)
                .chain([stem])
                .collect::<Vec<_>>()
                .join(".");
            let source = fs::read_to_string(&path)
                .map(Arc::from)
                .map_err(|error| error.to_string());
            found.insert(name, Found { path, source });
        }
    }
}

/// Whether `spec` could name a module under the root.
pub(super) fn is_module_name(spec: &str) -> bool {
    spec.split('.').all(is_identifier_like)
}
