//! Finding the file a module specification names.

use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};

use frostlang_runtime::{FrostError, is_identifier_like};

use crate::{FsResolver, cannot_import};

/// Why a probe of one root failed after finding something there.
enum ProbeError {
    /// A symbolic link where symbolic links are not followed.
    Symlink(PathBuf),
    /// The filesystem refused an operation on the path.
    Io(PathBuf, io::Error),
}

impl FsResolver {
    /// The canonical path of the file `spec` names, from the first root that has
    /// it, or `None` when no root has it or `spec` names no file.
    pub(crate) fn find(&self, spec: &str) -> Result<Option<PathBuf>, FrostError> {
        let Some(relative) = relative_path(spec) else {
            return Ok(None);
        };
        let probe = |root: &PathBuf| {
            let found = if self.follow_symlinks {
                probe_following(root, &relative)
            } else {
                probe_without_symlinks(root, &relative)
            };
            found
                .and_then(|path| path.map(canonical).transpose())
                .transpose()
        };
        self.roots
            .iter()
            .find_map(probe)
            .transpose()
            .map_err(|error| match error {
                ProbeError::Symlink(path) => cannot_import(
                    spec,
                    format_args!(
                        "'{}' is a symbolic link, which this importer does not follow",
                        path.display()
                    ),
                ),
                ProbeError::Io(path, error) => cannot_import(
                    spec,
                    format_args!("cannot access '{}': {error}", path.display()),
                ),
            })
    }
}

/// The file `spec` names, relative to a root: `a.b.c` names `a/b/c.frst`.
/// `None` unless `spec` is a `.`-separated path of identifier-like names.
fn relative_path(spec: &str) -> Option<PathBuf> {
    if !spec.split('.').all(is_identifier_like) {
        return None;
    }
    let mut path: PathBuf = spec.split('.').collect();
    path.set_extension("frst");
    Some(path)
}

/// `relative` under `root`, if it is a file there, following symbolic links.
fn probe_following(root: &Path, relative: &Path) -> Result<Option<PathBuf>, ProbeError> {
    let path = root.join(relative);
    match fs::metadata(&path) {
        Ok(metadata) => Ok(metadata.is_file().then_some(path)),
        Err(error) if is_absent(&error) => Ok(None),
        Err(error) => Err(ProbeError::Io(path, error)),
    }
}

/// `relative` under `root`, if it is a file there, refusing a symbolic link at
/// any step below `root`.
fn probe_without_symlinks(root: &Path, relative: &Path) -> Result<Option<PathBuf>, ProbeError> {
    let mut path = root.to_path_buf();
    let mut metadata: Option<Metadata> = None;
    for component in relative.components() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(found) if found.is_symlink() => return Err(ProbeError::Symlink(path)),
            Ok(found) => metadata = Some(found),
            Err(error) if is_absent(&error) => return Ok(None),
            Err(error) => return Err(ProbeError::Io(path, error)),
        }
    }
    Ok(metadata
        .is_some_and(|metadata| metadata.is_file())
        .then_some(path))
}

/// `path` with every symbolic link and relative step resolved.
fn canonical(path: PathBuf) -> Result<PathBuf, ProbeError> {
    fs::canonicalize(&path).map_err(|error| ProbeError::Io(path, error))
}

/// Whether `error` means there is no such file, as opposed to a file the
/// filesystem refused to show.
fn is_absent(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}
