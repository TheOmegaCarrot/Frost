//! [`EditDir`]: where the external editor edits a segment.

use std::env;
use std::fs::{self, DirBuilder};
use std::io;
use std::iter;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// How many names to try before giving up: a collision with a random name is
/// rare, so repeated ones mean something is wrong.
const ATTEMPTS: usize = 8;

/// A directory only this user can enter, removed when dropped. The external
/// editor's file lives in it, so no other user can plant a file or symlink at
/// the file's path.
pub(crate) struct EditDir(PathBuf);

impl EditDir {
    /// A new private directory in the system's temporary directory, under an
    /// unpredictable name.
    pub(crate) fn new() -> io::Result<Self> {
        let names = iter::repeat_with(|| format!("frost-edit-{:016x}", fastrand::u64(..)));
        Self::create_in(&env::temp_dir(), names)
    }

    /// A new private directory in `parent`, named by the first of `names` that
    /// no entry has taken, trying at most [`ATTEMPTS`] of them.
    fn create_in(parent: &Path, names: impl IntoIterator<Item = String>) -> io::Result<Self> {
        let mut builder = DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        let mut taken = None;
        for name in names.into_iter().take(ATTEMPTS) {
            let path = parent.join(name);
            // Creating a directory fails, rather than follows, if anything is
            // at the path already, a symlink included.
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => taken = Some(error),
                Err(error) => return Err(error),
            }
        }
        Err(taken.unwrap_or_else(|| io::Error::other("no name to create a directory under")))
    }

    /// The file the editor edits. Its extension lets the editor recognize Frost
    /// source.
    pub(crate) fn file(&self) -> PathBuf {
        self.0.join("segment.frst")
    }
}

impl Drop for EditDir {
    fn drop(&mut self) {
        // Nothing can be done about a failure here; the directory is only
        // left behind.
        let _ = fs::remove_dir_all(&self.0);
    }
}
