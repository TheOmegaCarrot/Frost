//! `std.fs`: paths, file metadata, directory listings, reading and writing
//! files, and creating, copying, moving, and removing files.
//!
//! Paths are Strings. A function whose result would hold a path that is not
//! UTF-8 raises. A relative path is resolved against the host process's working
//! directory, which scripts can read but not change.

use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::native::{Flat, Optional};
use crate::stdlib::stream::{self, Kind};
use crate::{
    FrostError, FrostMap, FrostResult, FrostString, NativeCtx, StdlibModule, Value, from_value,
    frost_closure, frost_fn,
};

/// The `std.fs` module: path manipulation, file metadata and type tests,
/// directory listing and glob matching, reading and writing files whole or as
/// streams, and
/// creating, copying, moving, linking, and removing files and directories.
///
/// It reaches outside the script: a script with it can read and change any
/// file the host process can.
pub fn fs() -> StdlibModule {
    StdlibModule::builder("fs")
        .value("read", path_function("std.fs.read", read))
        .value("read_bytes", path_function("std.fs.read_bytes", read_bytes))
        .value("write", content_function("std.fs.write", write))
        .value("append", content_function("std.fs.append", append))
        .value("open_read", path_function("std.fs.open_read", open_read))
        .value("open_write", path_function("std.fs.open_write", open_write))
        .value(
            "open_append",
            path_function("std.fs.open_append", open_append),
        )
        .value("absolute", path_function("std.fs.absolute", absolute))
        .value("canonical", path_function("std.fs.canonical", canonical))
        .value("exists", path_test("std.fs.exists", Path::exists))
        .value("is_file", path_test("std.fs.is_file", Path::is_file))
        .value(
            "is_directory",
            path_test("std.fs.is_directory", Path::is_dir),
        )
        .value(
            "is_symlink",
            path_test("std.fs.is_symlink", Path::is_symlink),
        )
        .value(
            "is_block",
            path_test("std.fs.is_block", |path| special_type(path, Special::Block)),
        )
        .value(
            "is_character",
            path_test("std.fs.is_character", |path| {
                special_type(path, Special::Character)
            }),
        )
        .value(
            "is_fifo",
            path_test("std.fs.is_fifo", |path| special_type(path, Special::Fifo)),
        )
        .value(
            "is_socket",
            path_test("std.fs.is_socket", |path| {
                special_type(path, Special::Socket)
            }),
        )
        .value("stat", path_function("std.fs.stat", stat))
        .value("size", path_function("std.fs.size", size))
        .function(cwd_frost())
        .value("list", path_function("std.fs.list", list))
        .function(glob_frost())
        .value(
            "list_recursively",
            path_function("std.fs.list_recursively", list_recursively),
        )
        .value("mkdir", path_function("std.fs.mkdir", mkdir))
        .value("move", two_path_function("std.fs.move", move_path))
        .value("copy", two_path_function("std.fs.copy", copy))
        .function(symlink_frost())
        .value("read_link", path_function("std.fs.read_link", read_link))
        .value("remove", path_function("std.fs.remove", remove))
        .value(
            "remove_recursively",
            path_function("std.fs.remove_recursively", remove_recursively),
        )
        .function(concat_frost())
        .value("stem", path_part("std.fs.stem", Path::file_stem))
        .function(extension_frost())
        .value("filename", path_part("std.fs.filename", Path::file_name))
        .value(
            "parent",
            path_part("std.fs.parent", |path| path.parent().map(Path::as_os_str)),
        )
        .build()
}

/// The path in a String argument.
fn path_arg(arg: &FrostString) -> &Path {
    Path::new(arg.as_str())
}

/// `path` as a String, if it is UTF-8; `function` names it in the error.
fn path_value(function: &str, path: &Path) -> FrostResult {
    path.to_str()
        .map(Value::from)
        .ok_or_else(|| not_utf8(function, path))
}

/// The error for `function` finding `path`, which is not UTF-8.
fn not_utf8(function: &str, path: &Path) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} found a path that is not UTF-8: {}",
        path.display()
    ))
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
    frost_closure!(name, move |path: FrostString| body(name, path_arg(&path)))
}

/// A function of two paths, a source and a destination, computing `body`.
fn two_path_function(name: &'static str, body: fn(&str, &Path, &Path) -> FrostResult) -> Value {
    frost_closure!(
        name,
        move |source: FrostString, destination: FrostString| {
            body(name, path_arg(&source), path_arg(&destination))
        }
    )
}

/// A test of one path, which is false of a path that does not exist.
fn path_test(name: &'static str, test: fn(&Path) -> bool) -> Value {
    frost_closure!(name, move |path: FrostString| test(path_arg(&path)))
}

/// A function of a path and content to put there, a String or Bytes,
/// computing `body`.
fn content_function(name: &'static str, body: fn(&str, &Path, &[u8]) -> FrostResult) -> Value {
    frost_closure!(name, move |path: FrostString, content: Flat| {
        body(name, path_arg(&path), content.as_bytes())
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

#[frost_fn]
fn cwd(#[frost(ctx)] ctx: &mut NativeCtx<'_>) -> FrostResult {
    let name = ctx.name();
    let cwd = std::env::current_dir()
        .map_err(|err| FrostError::from_string(format!("Function {name} failed: {err}")))?;
    path_value(name, &cwd)
}

#[frost_fn]
fn concat(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    base: FrostString,
    path: FrostString,
) -> FrostResult {
    let joined = path_arg(&base).join(path_arg(&path));
    path_value(ctx.name(), &joined)
}

/// A function returning the part of a path `part` picks, or the empty String
/// if there is none.
fn path_part(name: &'static str, part: fn(&Path) -> Option<&std::ffi::OsStr>) -> Value {
    frost_closure!(name, move |path: FrostString| {
        let part = part(path_arg(&path)).unwrap_or_default();
        path_value(name, Path::new(part))
    })
}

#[frost_fn]
fn extension(path: FrostString) -> Value {
    match path_arg(&path).extension() {
        // The extension is UTF-8: it is part of a String.
        Some(extension) => format!(".{}", extension.to_string_lossy()).into(),
        None => Value::from(""),
    }
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

/// A file's type.
#[derive(Clone, Copy)]
enum FileType {
    Symlink,
    Regular,
    Directory,
    Block,
    Character,
    Fifo,
    Socket,
    Unknown,
}

impl FileType {
    /// The type of the file `metadata` describes.
    fn of(metadata: &Metadata) -> Self {
        let kind = metadata.file_type();
        if kind.is_symlink() {
            return Self::Symlink;
        }
        if kind.is_file() {
            return Self::Regular;
        }
        if kind.is_dir() {
            return Self::Directory;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            if kind.is_block_device() {
                return Self::Block;
            }
            if kind.is_char_device() {
                return Self::Character;
            }
            if kind.is_fifo() {
                return Self::Fifo;
            }
            if kind.is_socket() {
                return Self::Socket;
            }
        }
        Self::Unknown
    }

    /// The type's name, as `stat` reports it.
    fn name(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Regular => "regular",
            Self::Directory => "directory",
            Self::Block => "block",
            Self::Character => "character",
            Self::Fifo => "fifo",
            Self::Socket => "socket",
            Self::Unknown => "unknown",
        }
    }

    /// What a file of the type is, for error messages: "`path` is {phrase}".
    fn phrase(self) -> &'static str {
        match self {
            Self::Symlink => "a symlink",
            Self::Regular => "a regular file",
            Self::Directory => "a directory",
            Self::Block => "a block device",
            Self::Character => "a character device",
            Self::Fifo => "a FIFO",
            Self::Socket => "a socket",
            Self::Unknown => "of an unknown type",
        }
    }
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
        ("type", Value::from(FileType::of(&metadata).name())),
        ("perms", permissions(&metadata)),
    ]))
}

fn size(name: &str, path: &Path) -> FrostResult {
    let metadata = fs::metadata(path).map_err(|err| io_error(name, path, &err))?;
    if !metadata.is_file() {
        return Err(FrostError::from_string(format!(
            "Function {name} requires a regular file, but `{}` is {}",
            path.display(),
            FileType::of(&metadata).phrase()
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

/// The options `fs.glob` takes in its optional second argument.
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct GlobOptions {
    /// Whether a wildcard may match a name's leading `.`.
    hidden: bool,
    /// Off, an ASCII letter matches either case; any other letter still
    /// matches only as written.
    case_sensitive: bool,
}

impl Default for GlobOptions {
    fn default() -> Self {
        Self {
            hidden: false,
            case_sensitive: true,
        }
    }
}

/// `glob(pattern, options?)`: the paths that `pattern` matches, sorted.
#[frost_fn]
fn glob(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    pattern: FrostString,
    options: Optional<FrostMap>,
) -> FrostResult {
    let name = ctx.name();
    let options = match options {
        Optional(Some(options)) => from_value(Value::from(options)).map_err(|err| {
            FrostError::from_string(format!(
                "Function {name} requires valid options: {}",
                err.message()
            ))
        })?,
        Optional(None) => GlobOptions::default(),
    };
    glob_paths(name, &pattern, &options)
}

/// One component of a glob pattern.
enum GlobStep<'a> {
    /// A component without wildcards, such as `src`, `..`, or the root,
    /// followed as written.
    Exact(&'a OsStr),
    /// A name with wildcards, matched against a directory's entries.
    Wildcard(glob::Pattern),
    /// `**`: any number of directories, the current one included.
    AnyDirectories,
}

/// The paths `pattern` matches, for the function named `function`.
fn glob_paths(function: &str, pattern: &str, options: &GlobOptions) -> FrostResult {
    let invalid = |err: glob::PatternError| {
        FrostError::from_string(format!(
            "Function {function} got an invalid pattern {}: {err}",
            Value::from(pattern).to_debug_string()
        ))
    };
    // Checked whole first, so an error's position is within the whole pattern.
    glob::Pattern::new(pattern).map_err(invalid)?;
    let mut steps = Path::new(pattern)
        .components()
        .map(|component| {
            let name = component.as_os_str();
            Ok(match name.to_str().expect("a component of a String") {
                "**" => GlobStep::AnyDirectories,
                text if text.contains(['*', '?', '[']) => {
                    GlobStep::Wildcard(glob::Pattern::new(text).map_err(invalid)?)
                }
                _ => GlobStep::Exact(name),
            })
        })
        .collect::<Result<Vec<_>, FrostError>>()?;
    // Repeated, `**` would find the same paths again; last, it means `**/*`.
    steps.dedup_by(|a, b| {
        matches!(a, GlobStep::AnyDirectories) && matches!(b, GlobStep::AnyDirectories)
    });
    if matches!(steps.last(), Some(GlobStep::AnyDirectories)) {
        let any = glob::Pattern::new("*").expect("`*` is a valid pattern");
        steps.push(GlobStep::Wildcard(any));
    }

    let mut glob = Glob {
        function,
        hidden: options.hidden,
        match_options: glob::MatchOptions {
            case_sensitive: options.case_sensitive,
            require_literal_leading_dot: !options.hidden,
            ..glob::MatchOptions::new()
        },
        found: Vec::new(),
    };
    // An empty pattern names nothing.
    if !steps.is_empty() {
        glob.expand(PathBuf::new(), &steps)?;
    }
    let mut found = glob.found;
    // A trailing separator, which `components` drops, matches only directories.
    if pattern.ends_with(std::path::is_separator) {
        found.retain(|path| path.is_dir());
    }
    // Different ways through two `**`s can reach the same path.
    found.sort();
    found.dedup();
    sorted_paths(function, found)
}

/// A glob search in progress.
struct Glob<'a> {
    /// The name of the function searching, for errors.
    function: &'a str,
    hidden: bool,
    match_options: glob::MatchOptions,
    found: Vec<PathBuf>,
}

impl Glob<'_> {
    /// Finds what `steps` match from `path`.
    fn expand(&mut self, path: PathBuf, steps: &[GlobStep]) -> Result<(), FrostError> {
        let Some((step, rest)) = steps.split_first() else {
            self.found.push(path);
            return Ok(());
        };
        match step {
            GlobStep::Exact(name) => {
                let next = path.join(name);
                if fs::symlink_metadata(&next).is_ok() {
                    self.expand(next, rest)?;
                }
            }
            GlobStep::Wildcard(pattern) => {
                for entry in glob_entries(self.function, &path)? {
                    if pattern.matches_with(&entry.name, self.match_options) {
                        self.expand(entry.path, rest)?;
                    }
                }
            }
            GlobStep::AnyDirectories => {
                self.expand(path.clone(), rest)?;
                for entry in glob_entries(self.function, &path)? {
                    // Never through a symlink, which could loop.
                    let hidden = entry.name.starts_with('.');
                    if entry.is_real_directory && (self.hidden || !hidden) {
                        self.expand(entry.path, steps)?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// An entry of a directory a glob searches.
struct GlobEntry {
    name: String,
    path: PathBuf,
    /// A directory, not a symlink to one.
    is_real_directory: bool,
}

/// The entries of `dir`, the current directory if empty. A path that is not a
/// directory, or one the process cannot read, has none. `function` names the
/// function searching, for errors.
fn glob_entries(function: &str, dir: &Path) -> Result<Vec<GlobEntry>, FrostError> {
    let on_disk = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let read = match fs::read_dir(on_disk) {
        Ok(read) => read,
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::NotFound
                    | io::ErrorKind::NotADirectory
                    | io::ErrorKind::PermissionDenied
            ) =>
        {
            return Ok(Vec::new());
        }
        Err(err) => return Err(io_error(function, on_disk, &err)),
    };
    read.map(|entry| {
        let entry = entry.map_err(|err| io_error(function, on_disk, &err))?;
        let path = dir.join(entry.file_name());
        let Ok(name) = entry.file_name().into_string() else {
            return Err(not_utf8(function, &path));
        };
        Ok(GlobEntry {
            name,
            is_real_directory: entry.file_type().is_ok_and(|kind| kind.is_dir()),
            path,
        })
    })
    .collect()
}

// --- Changing the filesystem ---

fn mkdir(name: &str, path: &Path) -> FrostResult {
    if path.is_dir() {
        return Ok(Value::Bool(false));
    }
    fs::create_dir_all(path).map_err(|err| io_error(name, path, &err))?;
    Ok(Value::Bool(true))
}

/// Moves `from` to `to`, replacing a file already at `to`. Within a filesystem,
/// the replacement is atomic, so writing a new file and moving it over the old
/// one updates the old one atomically.
fn move_path(name: &str, from: &Path, to: &Path) -> FrostResult {
    fs::rename(from, to).map_err(|err| io_error(name, from, &err))?;
    Ok(Value::Null)
}

fn copy(name: &str, from: &Path, to: &Path) -> FrostResult {
    check_copy(from, to, true)
        .and_then(|()| copy_tree(from, to, true))
        .map_err(|err| io_error(name, from, &err))?;
    Ok(Value::Null)
}

/// The metadata of `from`, a path to copy: of what a symlink points to if
/// `follow`, else of the link itself.
fn copied_metadata(from: &Path, follow: bool) -> io::Result<Metadata> {
    if follow {
        fs::metadata(from)
    } else {
        fs::symlink_metadata(from)
    }
}

/// Whether anything, even a broken symlink, is at `path`.
fn occupied(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn already_exists(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("`{}` already exists", path.display()),
    )
}

/// Checks that [`copy_tree`] would find nothing in its way, so that a copy
/// that would collide fails before copying anything. Something made in the
/// way during the copy still fails it, partway.
fn check_copy(from: &Path, to: &Path, follow: bool) -> io::Result<()> {
    if !copied_metadata(from, follow)?.is_dir() {
        return if occupied(to) {
            Err(already_exists(to))
        } else {
            Ok(())
        };
    }
    // A directory is copied into one already there, or a link to one.
    match fs::metadata(to) {
        Ok(existing) if existing.is_dir() => {
            for entry in entries(from)? {
                let name = entry.file_name().expect("a directory entry has a name");
                check_copy(&entry, &to.join(name), false)?;
            }
            Ok(())
        }
        Ok(_) => Err(already_exists(to)),
        Err(_) if occupied(to) => Err(already_exists(to)),
        // Nothing there, so nothing within it can be in the way.
        Err(_) => Ok(()),
    }
}

/// Copy `from` to `to`: a file's content, or a directory and all it holds.
/// A symlink is followed at the top (`follow`), and recreated as a link within
/// a tree, so a link loop cannot make the copy endless. An existing file is
/// never overwritten.
fn copy_tree(from: &Path, to: &Path, follow: bool) -> io::Result<()> {
    let metadata = copied_metadata(from, follow)?;
    if metadata.is_dir() {
        fs::create_dir_all(to)?;
        for entry in entries(from)? {
            let name = entry.file_name().expect("a directory entry has a name");
            copy_tree(&entry, &to.join(name), false)?;
        }
        return Ok(());
    }
    if metadata.file_type().is_symlink() {
        return make_symlink(&fs::read_link(from)?, to);
    }
    if occupied(to) {
        return Err(already_exists(to));
    }
    fs::copy(from, to).map(drop)
}

/// Make a symlink at `link_path` pointing to `target`.
fn make_symlink(target: &Path, link_path: &Path) -> io::Result<()> {
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

#[frost_fn]
fn symlink(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    target: FrostString,
    link: FrostString,
) -> FrostResult {
    let link_path = path_arg(&link);
    make_symlink(path_arg(&target), link_path)
        .map_err(|err| io_error(ctx.name(), link_path, &err))?;
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
