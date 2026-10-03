//! `std.fs`: paths, file metadata, directory listings, reading and writing
//! files, and creating, copying, moving, and removing files.
//!
//! Paths are Strings. A function whose result would hold a path that is not
//! UTF-8 raises. A relative path is resolved against the host process's working
//! directory, which scripts can read but not change.

use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::stdlib::stream::{self, Kind};
use crate::{Arity, FrostError, FrostResult, FrostType, Param, Params, StdlibModule, Value};

/// The `std.fs` module: path manipulation, file metadata and type tests,
/// directory listing, reading and writing files whole or as streams, and
/// creating, copying, moving, linking, and removing files and directories.
///
/// It reaches outside the script: a script with it can read and change any
/// file the host process can.
pub fn fs() -> StdlibModule {
    StdlibModule::new(
        "fs",
        Value::map([
            ("read", path_function("fs.read", read)),
            ("read_bytes", path_function("fs.read_bytes", read_bytes)),
            ("write", content_function("fs.write", write)),
            ("append", content_function("fs.append", append)),
            ("open_read", path_function("fs.open_read", open_read)),
            ("open_write", path_function("fs.open_write", open_write)),
            ("open_append", path_function("fs.open_append", open_append)),
            ("absolute", path_function("fs.absolute", absolute)),
            ("canonical", path_function("fs.canonical", canonical)),
            ("exists", path_test("fs.exists", Path::exists)),
            ("is_file", path_test("fs.is_file", Path::is_file)),
            ("is_directory", path_test("fs.is_directory", Path::is_dir)),
            ("is_symlink", path_test("fs.is_symlink", Path::is_symlink)),
            (
                "is_block",
                path_test("fs.is_block", |path| special_type(path, Special::Block)),
            ),
            (
                "is_character",
                path_test("fs.is_character", |path| {
                    special_type(path, Special::Character)
                }),
            ),
            (
                "is_fifo",
                path_test("fs.is_fifo", |path| special_type(path, Special::Fifo)),
            ),
            (
                "is_socket",
                path_test("fs.is_socket", |path| special_type(path, Special::Socket)),
            ),
            ("stat", path_function("fs.stat", stat)),
            ("size", path_function("fs.size", size)),
            ("cwd", cwd()),
            ("list", path_function("fs.list", list)),
            (
                "list_recursively",
                path_function("fs.list_recursively", list_recursively),
            ),
            ("mkdir", path_function("fs.mkdir", mkdir)),
            ("move", two_path_function("fs.move", move_path)),
            ("copy", two_path_function("fs.copy", copy)),
            ("symlink", two_path_function("fs.symlink", symlink)),
            ("read_link", path_function("fs.read_link", read_link)),
            ("remove", path_function("fs.remove", remove)),
            (
                "remove_recursively",
                path_function("fs.remove_recursively", remove_recursively),
            ),
            ("concat", concat()),
            ("stem", path_part("fs.stem", Path::file_stem)),
            ("extension", extension()),
            ("filename", path_part("fs.filename", Path::file_name)),
            (
                "parent",
                path_part("fs.parent", |path| path.parent().map(Path::as_os_str)),
            ),
        ]),
    )
}

const ONE_PATH: Params = Params::new(&[Param::of(FrostType::STRING).named("path")]);

/// The path in a type-checked String argument.
fn path_arg(arg: &Value) -> &Path {
    Path::new(arg.as_str().expect("type-checked as a String"))
}

/// `path` as a String, if it is UTF-8; `function` names it in the error.
fn path_value(function: &str, path: &Path) -> FrostResult {
    path.to_str().map(Value::from).ok_or_else(|| {
        FrostError::from_string(format!(
            "Function {function} found a path that is not UTF-8: {}",
            path.display()
        ))
    })
}

/// The error for `function` failing with `err` on `path`.
fn io_error(function: &str, path: &Path, err: &io::Error) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} failed on `{}`: {err}",
        path.display()
    ))
}

/// A function of one path, computing `body`, which reports failures as I/O errors.
fn path_function(name: &'static str, body: fn(&str, &Path) -> FrostResult) -> Value {
    Value::checked_native(name, ONE_PATH, move |_, args| {
        body(name, path_arg(&args[0]))
    })
}

/// A function of two paths, a source and a destination, computing `body`.
fn two_path_function(name: &'static str, body: fn(&str, &Path, &Path) -> FrostResult) -> Value {
    const PARAMS: Params =
        Params::new(&[Param::of(FrostType::STRING), Param::of(FrostType::STRING)]);
    Value::checked_native(name, PARAMS, move |_, args| {
        body(name, path_arg(&args[0]), path_arg(&args[1]))
    })
}

/// A test of one path, which is false of a path that does not exist.
fn path_test(name: &'static str, test: fn(&Path) -> bool) -> Value {
    Value::checked_native(name, ONE_PATH, move |_, args| {
        Ok(Value::Bool(test(path_arg(&args[0]))))
    })
}

/// A function of a path and content to put there, a String or Bytes,
/// computing `body`.
fn content_function(name: &'static str, body: fn(&str, &Path, &[u8]) -> FrostResult) -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING).named("path"),
        Param::of(FrostType::FLAT).named("content"),
    ]);
    Value::checked_native(name, PARAMS, move |_, args| {
        let content = args[1].as_byte_slice().expect("type-checked as Flat");
        body(name, path_arg(&args[0]), content)
    })
}

// --- Reading and writing ---

fn read(name: &str, path: &Path) -> FrostResult {
    let content = fs::read(path).map_err(|err| io_error(name, path, &err))?;
    String::from_utf8(content).map(Value::from).map_err(|_| {
        FrostError::from_string(format!(
            "Function {name} read text that is not UTF-8 from `{}`",
            path.display()
        ))
    })
}

fn read_bytes(name: &str, path: &Path) -> FrostResult {
    let content = fs::read(path).map_err(|err| io_error(name, path, &err))?;
    Ok(Value::from(content))
}

fn write(name: &str, path: &Path, content: &[u8]) -> FrostResult {
    fs::write(path, content).map_err(|err| io_error(name, path, &err))?;
    Ok(Value::Null)
}

fn append(name: &str, path: &Path, content: &[u8]) -> FrostResult {
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .and_then(|mut file| file.write_all(content))
        .map_err(|err| io_error(name, path, &err))?;
    Ok(Value::Null)
}

fn open_read(name: &str, path: &Path) -> FrostResult {
    let file = File::open(path).map_err(|err| io_error(name, path, &err))?;
    Ok(stream::reader(BufReader::new(file), Kind::File))
}

/// Opens a file for writing, emptying it first, or creating it.
fn open_write(name: &str, path: &Path) -> FrostResult {
    let file = File::create(path).map_err(|err| io_error(name, path, &err))?;
    Ok(stream::writer(BufWriter::new(file), Kind::File))
}

/// Opens a file for writing at its end, creating it if need be.
fn open_append(name: &str, path: &Path) -> FrostResult {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|err| io_error(name, path, &err))?;
    Ok(stream::writer(BufWriter::new(file), Kind::AppendFile))
}

// --- Paths ---

fn absolute(name: &str, path: &Path) -> FrostResult {
    let absolute = std::path::absolute(path).map_err(|err| io_error(name, path, &err))?;
    path_value(name, &absolute)
}

fn canonical(name: &str, path: &Path) -> FrostResult {
    let canonical = fs::canonicalize(path).map_err(|err| io_error(name, path, &err))?;
    path_value(name, &canonical)
}

fn cwd() -> Value {
    Value::native("fs.cwd", Arity::Exact(0), |_, _| {
        let cwd = std::env::current_dir()
            .map_err(|err| FrostError::from_string(format!("Function fs.cwd failed: {err}")))?;
        path_value("fs.cwd", &cwd)
    })
}

fn concat() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING).named("base"),
        Param::of(FrostType::STRING).named("path"),
    ]);
    Value::checked_native("fs.concat", PARAMS, |_, args| {
        let joined = path_arg(&args[0]).join(path_arg(&args[1]));
        path_value("fs.concat", &joined)
    })
}

/// A function returning the part of a path `part` picks, or the empty String
/// if there is none.
fn path_part(name: &'static str, part: fn(&Path) -> Option<&std::ffi::OsStr>) -> Value {
    Value::checked_native(name, ONE_PATH, move |_, args| {
        let part = part(path_arg(&args[0])).unwrap_or_default();
        path_value(name, Path::new(part))
    })
}

fn extension() -> Value {
    Value::checked_native("fs.extension", ONE_PATH, |_, args| {
        Ok(match path_arg(&args[0]).extension() {
            // The extension is UTF-8: it is part of a String.
            Some(extension) => format!(".{}", extension.to_string_lossy()).into(),
            None => Value::from(""),
        })
    })
}

// --- Metadata ---

/// The kinds of file only Unix has.
#[derive(Clone, Copy)]
enum Special {
    Block,
    Character,
    Fifo,
    Socket,
}

/// Whether `path` is a file of the `special` kind, following a symlink. False
/// on a platform without such files.
fn special_type(path: &Path, special: Special) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        let Ok(metadata) = fs::metadata(path) else {
            return false;
        };
        let kind = metadata.file_type();
        match special {
            Special::Block => kind.is_block_device(),
            Special::Character => kind.is_char_device(),
            Special::Fifo => kind.is_fifo(),
            Special::Socket => kind.is_socket(),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (path, special);
        false
    }
}

/// The name of `metadata`'s file type, as `stat` reports it.
fn type_name(metadata: &Metadata) -> &'static str {
    let kind = metadata.file_type();
    if kind.is_symlink() {
        return "symlink";
    }
    if kind.is_file() {
        return "regular";
    }
    if kind.is_dir() {
        return "directory";
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if kind.is_block_device() {
            return "block";
        }
        if kind.is_char_device() {
            return "character";
        }
        if kind.is_fifo() {
            return "fifo";
        }
        if kind.is_socket() {
            return "socket";
        }
    }
    "unknown"
}

/// `metadata`'s permissions, as `stat` reports them: read, write, and execute
/// for the owner, the group, and others.
fn permissions(metadata: &Metadata) -> Value {
    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    };
    // Elsewhere, only read-only is known: everyone reads, and writes unless it is set.
    #[cfg(not(unix))]
    let mode = if metadata.permissions().readonly() {
        0o444
    } else {
        0o666
    };

    let class = |shift: u32| {
        let bits = mode >> shift;
        Value::map([
            ("read", Value::Bool(bits & 0o4 != 0)),
            ("write", Value::Bool(bits & 0o2 != 0)),
            ("exec", Value::Bool(bits & 0o1 != 0)),
        ])
    };
    Value::map([
        ("owner", class(6)),
        ("group", class(3)),
        ("others", class(0)),
    ])
}

fn stat(name: &str, path: &Path) -> FrostResult {
    // The link itself, not what it points to.
    let metadata = fs::symlink_metadata(path).map_err(|err| io_error(name, path, &err))?;
    Ok(Value::map([
        ("type", Value::from(type_name(&metadata))),
        ("perms", permissions(&metadata)),
    ]))
}

fn size(name: &str, path: &Path) -> FrostResult {
    let metadata = fs::metadata(path).map_err(|err| io_error(name, path, &err))?;
    if !metadata.is_file() {
        return Err(FrostError::from_string(format!(
            "Function {name} requires a regular file, but `{}` is a {}",
            path.display(),
            type_name(&metadata)
        )));
    }
    Ok(Value::Int(
        i64::try_from(metadata.len()).expect("a file size fits in an Int"),
    ))
}

// --- Listing ---

/// The entries of the directory at `dir`, as paths under it.
fn entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect()
}

/// `paths`, sorted, as Strings.
fn sorted_paths(name: &str, mut paths: Vec<PathBuf>) -> FrostResult {
    paths.sort();
    paths
        .iter()
        .map(|path| path_value(name, path))
        .collect::<Result<Vec<Value>, FrostError>>()
        .map(Value::from)
}

fn list(name: &str, path: &Path) -> FrostResult {
    let paths = entries(path).map_err(|err| io_error(name, path, &err))?;
    sorted_paths(name, paths)
}

fn list_recursively(name: &str, path: &Path) -> FrostResult {
    let mut found = Vec::new();
    let mut pending = entries(path).map_err(|err| io_error(name, path, &err))?;
    while let Some(entry) = pending.pop() {
        // A symlink to a directory is listed, not entered.
        let is_dir = fs::symlink_metadata(&entry).is_ok_and(|metadata| metadata.is_dir());
        if is_dir {
            match entries(&entry) {
                Ok(inner) => pending.extend(inner),
                Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {}
                Err(err) => return Err(io_error(name, &entry, &err)),
            }
        }
        found.push(entry);
    }
    sorted_paths(name, found)
}

// --- Changing the filesystem ---

fn mkdir(name: &str, path: &Path) -> FrostResult {
    if path.is_dir() {
        return Ok(Value::Bool(false));
    }
    fs::create_dir_all(path).map_err(|err| io_error(name, path, &err))?;
    Ok(Value::Bool(true))
}

fn move_path(name: &str, from: &Path, to: &Path) -> FrostResult {
    fs::rename(from, to).map_err(|err| io_error(name, from, &err))?;
    Ok(Value::Null)
}

fn copy(name: &str, from: &Path, to: &Path) -> FrostResult {
    copy_tree(from, to, true).map_err(|err| io_error(name, from, &err))?;
    Ok(Value::Null)
}

/// Copy `from` to `to`: a file's content, or a directory and all it holds.
/// A symlink is followed at the top (`follow`), and recreated as a link within
/// a tree, so a link loop cannot make the copy endless. An existing file is
/// never overwritten.
fn copy_tree(from: &Path, to: &Path, follow: bool) -> io::Result<()> {
    let metadata = if follow {
        fs::metadata(from)?
    } else {
        fs::symlink_metadata(from)?
    };
    if metadata.is_dir() {
        fs::create_dir_all(to)?;
        for entry in entries(from)? {
            let name = entry.file_name().expect("a directory entry has a name");
            copy_tree(&entry, &to.join(name), false)?;
        }
        return Ok(());
    }
    if metadata.file_type().is_symlink() {
        return link(&fs::read_link(from)?, to);
    }
    if to.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("`{}` already exists", to.display()),
        ));
    }
    fs::copy(from, to).map(drop)
}

/// Make a symlink at `link_path` pointing to `target`.
fn link(target: &Path, link_path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)
    }
    #[cfg(windows)]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link_path)
        } else {
            std::os::windows::fs::symlink_file(target, link_path)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link_path);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "symbolic links are not supported on this platform",
        ))
    }
}

fn symlink(name: &str, target: &Path, link_path: &Path) -> FrostResult {
    link(target, link_path).map_err(|err| io_error(name, link_path, &err))?;
    Ok(Value::Null)
}

/// One step of resolution: a symlink's target exactly as the link stores it,
/// even if nothing is there; any other path itself.
fn read_link(name: &str, path: &Path) -> FrostResult {
    let metadata = fs::symlink_metadata(path).map_err(|err| io_error(name, path, &err))?;
    if !metadata.file_type().is_symlink() {
        return path_value(name, path);
    }
    let target = fs::read_link(path).map_err(|err| io_error(name, path, &err))?;
    path_value(name, &target)
}

fn remove(name: &str, path: &Path) -> FrostResult {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Value::Bool(false)),
        Err(err) => return Err(io_error(name, path, &err)),
    };
    let removed = if metadata.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    };
    removed.map_err(|err| io_error(name, path, &err))?;
    Ok(Value::Bool(true))
}

fn remove_recursively(name: &str, path: &Path) -> FrostResult {
    let removed = remove_tree(path).map_err(|err| io_error(name, path, &err))?;
    Ok(Value::Int(
        i64::try_from(removed).expect("a count of files fits in an Int"),
    ))
}

/// Remove `path` and all it holds, without following symlinks, counting what
/// is removed; nothing if it does not exist.
fn remove_tree(path: &Path) -> io::Result<u64> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(err) => return Err(err),
    };
    if !metadata.is_dir() {
        fs::remove_file(path)?;
        return Ok(1);
    }
    let mut removed = 0;
    for entry in entries(path)? {
        removed += remove_tree(&entry)?;
    }
    fs::remove_dir(path)?;
    Ok(removed + 1)
}
