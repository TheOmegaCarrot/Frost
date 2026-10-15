use std::path::{Path, PathBuf};
use std::{fs, io};

/// A script in the corpus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The script's path under the corpus root, `/`-separated and without its
    /// extension: its test name, such as `match/good/alternatives`.
    pub name: String,
    pub path: PathBuf,
}

/// Every script under `root`, sorted by name.
///
/// A script is a `.frst` file, and a `.md` file is documentation. Any other
/// file is an error, so that a misnamed script cannot silently go unrun.
pub fn discover(root: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    walk(root, root, &mut entries)?;
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn walk(root: &Path, dir: &Path, entries: &mut Vec<Entry>) -> io::Result<()> {
    for dir_entry in fs::read_dir(dir)? {
        let path = dir_entry?.path();
        if path.is_dir() {
            walk(root, &path, entries)?;
            continue;
        }
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("frst") => entries.push(Entry {
                name: name_of(root, &path),
                path,
            }),
            Some("md") => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{} is neither a script (.frst) nor documentation (.md)",
                        path.display()
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn name_of(root: &Path, path: &Path) -> String {
    let relative = path
        .strip_prefix(root)
        .expect("a walked path is under its root")
        .with_extension("");
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
