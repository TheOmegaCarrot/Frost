//! `std.fs`, from Frost source.
//!
//! Each case runs with only `std.fs` installed, bound as `fs`, and its own
//! scratch directory, bound as `dir`. The harness runs every case once per
//! optimization permutation, so a script that changes the filesystem undoes the
//! change before it ends, leaving each run the state the first one found.
//!
//! Unix only: the cases use POSIX paths, permission bits, `/dev/null`, FIFOs,
//! and sockets.

#![cfg(unix)]

mod source;

use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs, process};

use frost_runtime::stdlib::RandomConfig;
use frost_runtime::{Importer, ImporterBuilder, Stdlib, Value, stdlib};
use source::Script;

/// A scratch directory of one test's own, removed when it is dropped.
struct Scratch(PathBuf);

impl Scratch {
    /// A fresh, empty directory named for `test`.
    fn new(test: &str) -> Self {
        let path = env::temp_dir().join(format!("frost-std-fs-{}-{test}", process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the scratch directory is created");
        Self(path)
    }

    /// The path of `relative` within the directory.
    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    /// The path of `relative` within the directory, as text.
    fn text(&self, relative: &str) -> String {
        self.path(relative)
            .to_str()
            .expect("a UTF-8 path")
            .to_string()
    }

    /// Write a file holding `content` at `relative`, creating directories on the way.
    fn file(&self, relative: &str, content: &str) -> &Self {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("a file has a parent")).expect("parents created");
        fs::write(path, content).expect("the file is written");
        self
    }

    /// Make a directory at `relative`.
    fn dir(&self, relative: &str) -> &Self {
        fs::create_dir_all(self.path(relative)).expect("the directory is created");
        self
    }

    /// Make a symlink at `relative`, pointing to `target`.
    fn link(&self, relative: &str, target: impl AsRef<Path>) -> &Self {
        std::os::unix::fs::symlink(target, self.path(relative)).expect("the link is made");
        self
    }

    /// `expression`, run with `std.fs` bound as `fs` and this directory as `dir`.
    fn script(&self, expression: &str) -> Script {
        let source = format!(
            r"
            def fs = import('std.fs')
            {expression}
            "
        );
        Script::new(&source)
            .importer(importer())
            .capture("dir", Value::from(self.text("")))
    }

    fn run(&self, expression: &str) -> Value {
        self.script(expression).run()
    }

    fn raises(&self, expression: &str) -> String {
        self.script(expression).raises()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// An importer providing only `std.fs`.
fn importer() -> Arc<Importer> {
    let stdlib = Stdlib::new()
        .with_module(stdlib::fs())
        .expect("a lone module is accepted");
    ImporterBuilder::new().with_stdlib(stdlib).build()
}

/// The value of the Frost expression `source`, which needs no module.
fn frost(source: &str) -> Value {
    Script::new(source).run()
}

/// Assert `raised` is `function`'s I/O failure on `path`; the OS's own words
/// after it vary by platform.
fn assert_io_error(raised: &str, function: &str, path: &str) {
    let prefix = format!("Function {function} failed on `{path}`: ");
    assert!(raised.starts_with(&prefix), "{raised:?} starts {prefix:?}");
}

// --- The module ---

#[test]
fn the_module_holds_its_functions() {
    let scratch = Scratch::new("module");
    assert_eq!(
        scratch.run("sorted(keys(fs))"),
        frost(
            "['absolute', 'canonical', 'concat', 'copy', 'cwd', 'exists', 'extension', \
             'filename', 'is_block', 'is_character', 'is_directory', 'is_fifo', 'is_file', \
             'is_socket', 'is_symlink', 'list', 'list_recursively', 'mkdir', 'move', \
             'parent', 'read_link', 'remove', 'remove_recursively', 'size', 'stat', 'stem', \
             'symlink']"
        )
    );
}

#[test]
fn the_module_is_not_contained() {
    let stdlib = Stdlib::contained(RandomConfig::default());
    let contained = ImporterBuilder::new().with_stdlib(stdlib).build();
    let raised = Script::new("import('std.fs')").importer(contained).raises();
    assert_eq!(raised, "Could not resolve import 'std.fs'");
}

// --- Paths ---

#[test]
fn path_parts() {
    let scratch = Scratch::new("parts");
    for (expression, expected) in [
        ("fs.stem('/a/b/file.txt')", "'file'"),
        ("fs.stem('archive.tar.gz')", "'archive.tar'"),
        ("fs.stem('/a/b/file')", "'file'"),
        ("fs.stem('.bashrc')", "'.bashrc'"),
        ("fs.extension('/a/b/file.txt')", "'.txt'"),
        ("fs.extension('archive.tar.gz')", "'.gz'"),
        ("fs.extension('/a/b/file')", "''"),
        ("fs.extension('.bashrc')", "''"),
        ("fs.filename('/a/b/file.txt')", "'file.txt'"),
        ("fs.filename('/a/b/')", "'b'"),
        ("fs.filename('/')", "''"),
        ("fs.parent('/a/b/file.txt')", "'/a/b'"),
        ("fs.parent('/a/b')", "'/a'"),
        ("fs.parent('file.txt')", "''"),
        ("fs.parent('/')", "''"),
        ("fs.concat('/a', 'b')", "'/a/b'"),
        ("fs.concat('/a/', 'b/c')", "'/a/b/c'"),
        ("fs.concat('a', '')", "'a/'"),
        // An absolute second path replaces the first.
        ("fs.concat('/a', '/b')", "'/b'"),
    ] {
        assert_eq!(scratch.run(expression), frost(expected), "{expression}");
    }
}

#[test]
fn cwd_absolute_and_canonical_resolve_paths() {
    let scratch = Scratch::new("resolve");
    scratch.dir("sub");
    let cwd = env::current_dir().expect("a working directory");
    let cwd = cwd.to_str().expect("a UTF-8 working directory");
    assert_eq!(scratch.run("fs.cwd()"), Value::from(cwd));
    assert_eq!(
        scratch.run("[fs.absolute('x') == fs.concat(fs.cwd(), 'x'), fs.absolute('/abs')]"),
        frost("[true, '/abs']")
    );
    let canonical = fs::canonicalize(scratch.path("sub")).expect("canonical");
    assert_eq!(
        scratch.run("fs.canonical(fs.concat(dir, 'sub/../sub/.'))"),
        Value::from(canonical.to_str().expect("UTF-8"))
    );
    let missing = scratch.text("missing");
    assert_io_error(
        &scratch.raises("fs.canonical(fs.concat(dir, 'missing'))"),
        "fs.canonical",
        &missing,
    );
}

// --- Types and metadata ---

#[test]
fn type_tests_describe_what_is_at_a_path() {
    let scratch = Scratch::new("types");
    scratch
        .file("file", "x")
        .dir("dir")
        .link("to_file", scratch.path("file"))
        .link("dangling", scratch.path("missing"));
    let table = r"
        defn kinds(name) -> do {
            def path = fs.concat(dir, name)
            [fs.exists(path), fs.is_file(path), fs.is_directory(path), fs.is_symlink(path)]
        }
        map ['file', 'dir', 'to_file', 'dangling', 'missing'] with kinds
    ";
    assert_eq!(
        scratch.run(table),
        frost(
            "[
                [true, true, false, false],
                [true, false, true, false],
                [true, true, false, true],
                [false, false, false, true],
                [false, false, false, false],
            ]"
        ),
        "a symlink is followed except by is_symlink"
    );
}

#[test]
fn special_file_tests() {
    let scratch = Scratch::new("special");
    let fifo = scratch.path("fifo");
    let made = process::Command::new("mkfifo").arg(&fifo).status();
    assert!(
        made.is_ok_and(|status| status.success()),
        "mkfifo makes a FIFO"
    );
    let socket = scratch.path("socket");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).expect("a socket is bound");
    scratch.file("file", "x");

    let tests = r"
        defn special(path) -> [fs.is_block(path), fs.is_character(path), fs.is_fifo(path), fs.is_socket(path)]
        map ['/dev/null', fs.concat(dir, 'fifo'), fs.concat(dir, 'socket'), fs.concat(dir, 'file'), fs.concat(dir, 'missing')] with special
    ";
    assert_eq!(
        scratch.run(tests),
        frost(
            "[
                [false, true, false, false],
                [false, false, true, false],
                [false, false, false, true],
                [false, false, false, false],
                [false, false, false, false],
            ]"
        )
    );
    // A block device, where the machine has a familiar one.
    let block = ["/dev/loop0", "/dev/sda", "/dev/vda", "/dev/nvme0n1"]
        .into_iter()
        .find(|path| {
            use std::os::unix::fs::FileTypeExt;
            fs::metadata(path).is_ok_and(|metadata| metadata.file_type().is_block_device())
        });
    if let Some(block) = block {
        assert_eq!(
            scratch.run(&format!("fs.is_block('{block}')")),
            Value::Bool(true)
        );
    }
}

#[test]
fn stat_describes_a_file_without_following_a_link() {
    let scratch = Scratch::new("stat");
    scratch
        .file("file", "x")
        .dir("dir")
        .link("link", scratch.path("file"));
    fs::set_permissions(scratch.path("file"), fs::Permissions::from_mode(0o751))
        .expect("permissions set");
    assert_eq!(
        scratch.run("fs.stat(fs.concat(dir, 'file'))"),
        frost(
            "{
                type: 'regular',
                perms: {
                    owner: {read: true, write: true, exec: true},
                    group: {read: true, write: false, exec: true},
                    others: {read: false, write: false, exec: true},
                },
            }"
        )
    );
    let types = r"
        map [fs.concat(dir, 'dir'), fs.concat(dir, 'link'), '/dev/null'] with fn path -> fs.stat(path).type
    ";
    assert_eq!(
        scratch.run(types),
        frost("['directory', 'symlink', 'character']")
    );
    assert_io_error(
        &scratch.raises("fs.stat(fs.concat(dir, 'missing'))"),
        "fs.stat",
        &scratch.text("missing"),
    );
}

#[test]
fn size_measures_a_regular_file() {
    let scratch = Scratch::new("size");
    scratch.file("five", "hello").file("empty", "").dir("dir");
    assert_eq!(
        scratch.run("[fs.size(fs.concat(dir, 'five')), fs.size(fs.concat(dir, 'empty'))]"),
        frost("[5, 0]")
    );
    assert_eq!(
        scratch.raises("fs.size(fs.concat(dir, 'dir'))"),
        format!(
            "Function fs.size requires a regular file, but `{}` is a directory",
            scratch.text("dir")
        )
    );
    assert_io_error(
        &scratch.raises("fs.size(fs.concat(dir, 'missing'))"),
        "fs.size",
        &scratch.text("missing"),
    );
}

// --- Listing ---

#[test]
fn list_lists_a_directory_in_order() {
    let scratch = Scratch::new("list");
    scratch
        .file("b.txt", "")
        .file("a.txt", "")
        .file("sub/inner", "")
        .dir("empty");
    assert_eq!(
        scratch.run("fs.list(dir)"),
        Value::array([
            scratch.text("a.txt"),
            scratch.text("b.txt"),
            scratch.text("empty"),
            scratch.text("sub"),
        ])
    );
    assert_eq!(scratch.run("fs.list(fs.concat(dir, 'empty'))"), frost("[]"));
    assert_io_error(
        &scratch.raises("fs.list(fs.concat(dir, 'missing'))"),
        "fs.list",
        &scratch.text("missing"),
    );
    assert_io_error(
        &scratch.raises("fs.list(fs.concat(dir, 'a.txt'))"),
        "fs.list",
        &scratch.text("a.txt"),
    );
}

#[test]
fn list_recursively_lists_a_tree_without_entering_links() {
    let scratch = Scratch::new("tree");
    scratch
        .file("tree/a", "")
        .file("tree/sub/b", "")
        .file("tree/sub/deeper/c", "")
        .link("tree/link", scratch.path("tree/sub"));
    let expected: Vec<String> = ["a", "link", "sub", "sub/b", "sub/deeper", "sub/deeper/c"]
        .into_iter()
        .map(|entry| scratch.text(&format!("tree/{entry}")))
        .collect();
    assert_eq!(
        scratch.run("fs.list_recursively(fs.concat(dir, 'tree'))"),
        Value::from_iter(expected.into_iter().map(Value::from))
    );
    assert_io_error(
        &scratch.raises("fs.list_recursively(fs.concat(dir, 'missing'))"),
        "fs.list_recursively",
        &scratch.text("missing"),
    );
}

#[test]
fn a_path_that_is_not_utf8_is_an_error() {
    let scratch = Scratch::new("utf8");
    let name = std::ffi::OsStr::from_bytes(b"bad\xff");
    fs::write(scratch.path("").join(name), "").expect("a non-UTF-8 name is written");
    let raised = scratch.raises("fs.list(dir)");
    assert!(
        raised.starts_with("Function fs.list found a path that is not UTF-8: "),
        "{raised}"
    );
}

// --- Changing the filesystem ---

#[test]
fn mkdir_makes_a_directory_and_its_parents() {
    let scratch = Scratch::new("mkdir");
    scratch.file("file", "");
    let source = r"
        def path = fs.concat(dir, 'new/deeper')
        [fs.mkdir(path), fs.mkdir(path), fs.is_directory(path), fs.remove_recursively(fs.concat(dir, 'new'))]
    ";
    assert_eq!(scratch.run(source), frost("[true, false, true, 2]"));
    assert_io_error(
        &scratch.raises("fs.mkdir(fs.concat(dir, 'file'))"),
        "fs.mkdir",
        &scratch.text("file"),
    );
}

#[test]
fn move_renames() {
    let scratch = Scratch::new("move");
    scratch.file("from", "content");
    let source = r"
        def from = fs.concat(dir, 'from')
        def to = fs.concat(dir, 'to')
        [fs.move(from, to), fs.exists(from), fs.size(to), fs.move(to, from)]
    ";
    assert_eq!(scratch.run(source), frost("[null, false, 7, null]"));
    assert_io_error(
        &scratch.raises("fs.move(fs.concat(dir, 'missing'), fs.concat(dir, 'x'))"),
        "fs.move",
        &scratch.text("missing"),
    );
}

#[test]
fn copy_copies_a_file_without_overwriting() {
    let scratch = Scratch::new("copy_file");
    scratch.file("from", "hello").file("taken", "");
    let source = r"
        def to = fs.concat(dir, 'to')
        [fs.copy(fs.concat(dir, 'from'), to), fs.size(to), fs.remove(to)]
    ";
    assert_eq!(scratch.run(source), frost("[null, 5, true]"));
    assert_io_error(
        &scratch.raises("fs.copy(fs.concat(dir, 'from'), fs.concat(dir, 'taken'))"),
        "fs.copy",
        &scratch.text("from"),
    );
    assert_eq!(
        fs::read_to_string(scratch.path("taken")).unwrap(),
        "",
        "untouched"
    );
}

#[test]
fn copy_copies_a_tree_keeping_links_as_links() {
    let scratch = Scratch::new("copy_tree");
    scratch
        .file("tree/a", "a")
        .file("tree/sub/b", "bb")
        // A link to its own directory: followed, the copy would never end.
        .link("tree/sub/loop", ".");
    let source = r"
        def from = fs.concat(dir, 'tree')
        def to = fs.concat(dir, 'copy')
        defn names(root) -> map fs.list_recursively(root) with fn path -> slice(path, len(root))
        fs.copy(from, to)
        def result = [
            names(to) == names(from),
            fs.size(fs.concat(to, 'sub/b')),
            fs.is_symlink(fs.concat(to, 'sub/loop')),
        ]
        fs.remove_recursively(to)
        result
    ";
    assert_eq!(scratch.run(source), frost("[true, 2, true]"));
}

#[test]
fn symlink_makes_a_link() {
    let scratch = Scratch::new("symlink");
    scratch.file("target", "x").file("taken", "");
    let source = r"
        def link = fs.concat(dir, 'link')
        [fs.symlink(fs.concat(dir, 'target'), link), fs.is_symlink(link), fs.is_file(link), fs.remove(link)]
    ";
    assert_eq!(scratch.run(source), frost("[null, true, true, true]"));
    assert_io_error(
        &scratch.raises("fs.symlink(fs.concat(dir, 'target'), fs.concat(dir, 'taken'))"),
        "fs.symlink",
        &scratch.text("taken"),
    );
}

#[test]
fn read_link_follows_one_link() {
    let scratch = Scratch::new("read_link");
    scratch
        .file("file", "x")
        .link("inner", scratch.path("file"))
        .link("outer", scratch.path("inner"));
    // Each step reveals the next link in the chain, not where the chain ends.
    let source = r"
        def outer = fs.concat(dir, 'outer')
        [fs.read_link(outer), fs.read_link(fs.read_link(outer))]
    ";
    assert_eq!(
        scratch.run(source),
        Value::array([scratch.text("inner"), scratch.text("file")])
    );
}

#[test]
fn read_link_gives_a_target_as_the_link_stores_it() {
    let scratch = Scratch::new("read_link_stored");
    scratch
        .file("file", "x")
        .link("relative", "file")
        .link("dangling", scratch.path("gone"));
    assert_eq!(
        scratch.run("fs.read_link(fs.concat(dir, 'relative'))"),
        frost("'file'"),
        "a relative target stays relative to the link"
    );
    assert_eq!(
        scratch.run("fs.read_link(fs.concat(dir, 'dangling'))"),
        Value::from(scratch.text("gone")),
        "a broken link still names its target"
    );
}

#[test]
fn read_link_gives_any_other_path_itself() {
    let scratch = Scratch::new("read_link_other");
    scratch.file("file", "x").dir("dir");
    for name in ["file", "dir"] {
        assert_eq!(
            scratch.run(&format!("fs.read_link(fs.concat(dir, '{name}'))")),
            Value::from(scratch.text(name)),
            "{name}"
        );
    }
    assert_io_error(
        &scratch.raises("fs.read_link(fs.concat(dir, 'missing'))"),
        "fs.read_link",
        &scratch.text("missing"),
    );
}

#[test]
fn remove_removes_a_file_link_or_empty_directory() {
    let scratch = Scratch::new("remove");
    scratch.file("full/x", "").file("target", "x");
    let source = r"
        def empty = fs.concat(dir, 'empty')
        def link = fs.concat(dir, 'link')
        def target = fs.concat(dir, 'target')
        fs.mkdir(empty)
        fs.symlink(target, link)
        [
            fs.remove(empty),
            fs.remove(link),
            fs.exists(target),
            fs.remove(fs.concat(dir, 'missing')),
        ]
    ";
    assert_eq!(scratch.run(source), frost("[true, true, true, false]"));
    assert_io_error(
        &scratch.raises("fs.remove(fs.concat(dir, 'full'))"),
        "fs.remove",
        &scratch.text("full"),
    );
}

#[test]
fn remove_recursively_counts_what_it_removes() {
    let scratch = Scratch::new("remove_tree");
    scratch
        .file("tree/a", "")
        .file("tree/sub/b", "")
        .file("kept/c", "")
        .link("tree/to_kept", scratch.path("kept"));
    let source = r"
        def from = fs.concat(dir, 'tree')
        def to = fs.concat(dir, 'copy')
        fs.copy(from, to)
        def entries = len(fs.list_recursively(to))
        [
            fs.remove_recursively(to) == entries + 1,
            fs.exists(to),
            fs.exists(fs.concat(dir, 'kept/c')),
            fs.remove_recursively(fs.concat(dir, 'missing')),
        ]
    ";
    assert_eq!(scratch.run(source), frost("[true, false, true, 0]"));
}

// --- Arguments ---

#[test]
fn every_function_checks_its_arguments() {
    let scratch = Scratch::new("arguments");
    let one_path = [
        "absolute",
        "canonical",
        "exists",
        "is_file",
        "is_directory",
        "is_symlink",
        "is_block",
        "is_character",
        "is_fifo",
        "is_socket",
        "stat",
        "size",
        "list",
        "list_recursively",
        "mkdir",
        "remove",
        "remove_recursively",
        "stem",
        "extension",
        "filename",
        "parent",
        "read_link",
    ];
    for function in one_path {
        assert_eq!(
            scratch.raises(&format!("fs.{function}(1)")),
            format!("Function fs.{function} requires String as argument 1 (path), got Int")
        );
        for argc in [0, 2] {
            let args = vec!["'a'"; argc].join(", ");
            assert_eq!(
                scratch.raises(&format!("fs.{function}({args})")),
                format!("Function fs.{function} expects 1 arguments, but was called with {argc}")
            );
        }
    }
    for function in ["move", "copy", "symlink"] {
        assert_eq!(
            scratch.raises(&format!("fs.{function}('a', x'62')")),
            format!("Function fs.{function} requires String as argument 2, got Bytes")
        );
        assert_eq!(
            scratch.raises(&format!("fs.{function}('a')")),
            format!("Function fs.{function} expects 2 arguments, but was called with 1")
        );
    }
    assert_eq!(
        scratch.raises("fs.concat('a', 1)"),
        "Function fs.concat requires String as argument 2 (path), got Int"
    );
    assert_eq!(
        scratch.raises("fs.cwd(1)"),
        "Function fs.cwd expects 0 arguments, but was called with 1"
    );
}
