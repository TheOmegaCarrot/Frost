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
use std::{env, fs, process};

use frost_runtime::stdlib::RandomConfig;
use frost_runtime::{ImporterBuilder, Stdlib, Value, stdlib};
use source::Script;
use source::assertions::Library;

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
        FS.script(expression)
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

/// `std.fs`, bound as `fs`.
const FS: Library = Library::module(stdlib::fs, "fs");

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
            "['absolute', 'append', 'canonical', 'concat', 'copy', 'cwd', 'exists', \
             'extension', 'filename', 'glob', 'is_block', 'is_character', 'is_directory', \
             'is_fifo', \
             'is_file', 'is_socket', 'is_symlink', 'list', 'list_recursively', 'mkdir', 'move', \
             'open_append', 'open_read', 'open_write', 'parent', 'read', 'read_bytes', \
             'read_link', 'remove', 'remove_recursively', 'size', 'stat', 'stem', 'symlink', \
             'write']"
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
    assert_io_error(&scratch.raises("fs.absolute('')"), "fs.absolute", "");
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
    assert_eq!(
        scratch.run(
            r"
            map [fs.concat(dir, 'fifo'), fs.concat(dir, 'socket')] with fn path -> fs.stat(path).type
            "
        ),
        frost("['fifo', 'socket']")
    );
    // A block device, where the machine has a familiar one.
    let block = ["/dev/loop0", "/dev/sda", "/dev/vda", "/dev/nvme0n1"]
        .into_iter()
        .find(|path| {
            use std::os::unix::fs::FileTypeExt;
            fs::metadata(path).is_ok_and(|metadata| metadata.file_type().is_block_device())
        });
    match block {
        Some(block) => {
            assert_eq!(
                scratch.run(&format!("fs.is_block('{block}')")),
                Value::Bool(true)
            );
            assert_eq!(
                scratch.run(&format!("fs.stat('{block}').type")),
                frost("'block'")
            );
        }
        None => eprintln!("skipped: no familiar block device here, so is_block is not tested true"),
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
    scratch
        .file("five", "hello")
        .file("empty", "")
        .dir("dir")
        .link("to_five", scratch.path("five"));
    assert_eq!(
        scratch.run("[fs.size(fs.concat(dir, 'five')), fs.size(fs.concat(dir, 'empty'))]"),
        frost("[5, 0]")
    );
    assert_eq!(
        scratch.run("fs.size(fs.concat(dir, 'to_five'))"),
        Value::Int(5),
        "a link is followed to the file's size"
    );
    assert_eq!(
        scratch.raises("fs.size('/dev/null')"),
        "Function fs.size requires a regular file, but `/dev/null` is a character device"
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
fn listings_sort_by_path_component() {
    let scratch = Scratch::new("component_order");
    scratch.file("sub/b", "").file("sub-x", "");
    // Sorted as text, `sub-x` would come before `sub/b`.
    let expected: Vec<String> = ["sub", "sub/b", "sub-x"]
        .into_iter()
        .map(|entry| scratch.text(entry))
        .collect();
    assert_eq!(
        scratch.run("fs.list_recursively(dir)"),
        Value::from_iter(expected.into_iter().map(Value::from))
    );
}

/// Restores a directory's permissions when dropped, so a failed assertion
/// cannot leave a directory the scratch cleanup cannot remove.
struct Restore {
    path: PathBuf,
    mode: u32,
}

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(self.mode));
    }
}

#[test]
fn list_recursively_lists_but_skips_a_directory_it_may_not_read() {
    let scratch = Scratch::new("denied");
    scratch
        .file("tree/open/x", "")
        .file("tree/locked/hidden", "");
    let locked = scratch.path("tree/locked");
    let mode = fs::metadata(&locked)
        .expect("metadata")
        .permissions()
        .mode();
    let _restore = Restore {
        path: locked.clone(),
        mode,
    };
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("permissions set");
    // Root reads any directory, so the denial cannot be arranged: skip.
    if fs::read_dir(&locked).is_ok() {
        eprintln!("skipped: this user can read a directory with no permissions (root?)");
        return;
    }
    let expected: Vec<String> = ["locked", "open", "open/x"]
        .into_iter()
        .map(|entry| scratch.text(&format!("tree/{entry}")))
        .collect();
    assert_eq!(
        scratch.run("fs.list_recursively(fs.concat(dir, 'tree'))"),
        Value::from_iter(expected.into_iter().map(Value::from)),
        "the unreadable directory is listed, not entered"
    );
    assert_io_error(
        &scratch.raises("fs.list(fs.concat(dir, 'tree/locked'))"),
        "fs.list",
        &scratch.text("tree/locked"),
    );
}

#[test]
fn a_path_that_is_not_utf8_is_an_error() {
    let scratch = Scratch::new("utf8");
    let name = std::ffi::OsStr::from_bytes(b"bad\xff");
    fs::write(scratch.path("").join(name), "").expect("a non-UTF-8 name is written");
    for (call, function) in [
        ("fs.list(dir)", "fs.list"),
        ("fs.glob(dir + '*')", "fs.glob"),
    ] {
        let raised = scratch.raises(call);
        assert!(
            raised.starts_with(&format!(
                "Function {function} found a path that is not UTF-8: "
            )),
            "{raised}"
        );
    }
}

// --- glob ---

/// A tree for glob cases: files at several depths, hidden files and a hidden
/// directory, and symlinks to a directory and to the tree itself.
fn glob_tree(test: &str) -> Scratch {
    let scratch = Scratch::new(test);
    scratch
        .file("a.rs", "")
        .file("b.txt", "")
        .file(".hidden.rs", "")
        .file("src/c.rs", "")
        .file("src/d.rs", "")
        .file("src/.f.rs", "")
        .file("src/sub/e.rs", "")
        .file(".git/g.rs", "")
        .link("linked", scratch.path("src"))
        .link("src/loop", "..");
    scratch
}

/// Assert `fs.glob` of each pattern, written below the scratch directory and
/// given `options` (Frost source for the second argument, or empty), finds
/// exactly the paths named, below the scratch directory, in order.
fn assert_globs(scratch: &Scratch, options: &str, cases: &[(&str, &[&str])]) {
    let options = if options.is_empty() {
        String::new()
    } else {
        format!(", {options}")
    };
    for (pattern, expected) in cases {
        let call = format!("fs.glob(dir + '{pattern}'{options})");
        let expected =
            Value::from_iter(expected.iter().map(|name| Value::from(scratch.text(name))));
        assert_eq!(scratch.run(&call), expected, "{call}");
    }
}

#[test]
fn glob_matches_wildcards_within_a_name() {
    let scratch = glob_tree("glob_wildcards");
    assert_globs(
        &scratch,
        "",
        &[
            ("*.rs", &["a.rs"]),
            ("src/*.rs", &["src/c.rs", "src/d.rs"]),
            ("?.*", &["a.rs", "b.txt"]),
            ("[ab].*", &["a.rs", "b.txt"]),
            ("[!a]*", &["b.txt", "linked", "src"]),
            (
                "*/*.rs",
                &["linked/c.rs", "linked/d.rs", "src/c.rs", "src/d.rs"],
            ),
            ("*.none", &[]),
        ],
    );
}

#[test]
fn glob_matches_any_depth_with_a_double_star() {
    let scratch = glob_tree("glob_double_star");
    assert_globs(
        &scratch,
        "",
        &[
            // The symlinks match, but are not entered: no loop, no duplicates.
            ("**/*.rs", &["a.rs", "src/c.rs", "src/d.rs", "src/sub/e.rs"]),
            ("src/**/*.rs", &["src/c.rs", "src/d.rs", "src/sub/e.rs"]),
            ("**/sub", &["src/sub"]),
            ("**/l*", &["linked", "src/loop"]),
            // `*` follows `linked` into `src`; the `**` within it does not
            // follow `loop`.
            (
                "*/**/*.rs",
                &[
                    "linked/c.rs",
                    "linked/d.rs",
                    "linked/sub/e.rs",
                    "src/c.rs",
                    "src/d.rs",
                    "src/sub/e.rs",
                ],
            ),
        ],
    );
}

#[test]
fn glob_ending_in_a_double_star_matches_everything_below() {
    let scratch = glob_tree("glob_trailing_double_star");
    assert_globs(
        &scratch,
        "",
        &[(
            "src/**",
            &[
                "src/c.rs",
                "src/d.rs",
                "src/loop",
                "src/sub",
                "src/sub/e.rs",
            ],
        )],
    );
}

#[test]
fn glob_finds_each_path_once() {
    let scratch = glob_tree("glob_once");
    assert_globs(
        &scratch,
        "",
        &[
            // `src/sub/e.rs` is reached with either `**` taking `sub`. The `*`
            // follows `src/loop` back to the top, once.
            (
                "**/*/**/e.rs",
                &["linked/sub/e.rs", "src/loop/src/sub/e.rs", "src/sub/e.rs"],
            ),
            (
                "**/**/*.rs",
                &["a.rs", "src/c.rs", "src/d.rs", "src/sub/e.rs"],
            ),
        ],
    );
}

#[test]
fn glob_follows_symlinked_directories_a_single_component_matches() {
    let scratch = glob_tree("glob_symlinks");
    // Without `**` the depth is bounded, so even `loop` cannot loop.
    assert_globs(
        &scratch,
        "",
        &[("*/*/a.rs", &["linked/loop/a.rs", "src/loop/a.rs"])],
    );
}

#[test]
fn glob_wildcards_skip_hidden_names_unless_asked() {
    let scratch = glob_tree("glob_hidden");
    assert_globs(
        &scratch,
        "",
        &[
            // A `.` written in the pattern matches one.
            (".*", &[".git", ".hidden.rs"]),
            (".git/*", &[".git/g.rs"]),
            ("**/.f.rs", &["src/.f.rs"]),
        ],
    );
    assert_globs(
        &scratch,
        "{hidden: true}",
        &[
            ("*.rs", &[".hidden.rs", "a.rs"]),
            (
                "**/*.rs",
                &[
                    ".git/g.rs",
                    ".hidden.rs",
                    "a.rs",
                    "src/.f.rs",
                    "src/c.rs",
                    "src/d.rs",
                    "src/sub/e.rs",
                ],
            ),
        ],
    );
    assert_globs(&scratch, "{hidden: false}", &[("*.rs", &["a.rs"])]);
}

#[test]
fn glob_matches_case_sensitively_unless_asked() {
    let scratch = glob_tree("glob_case");
    assert_globs(&scratch, "", &[("*.RS", &[]), ("[A-B].*", &[])]);
    assert_globs(
        &scratch,
        "{case_sensitive: false}",
        &[("*.RS", &["a.rs"]), ("[A-B].*", &["a.rs", "b.txt"])],
    );
    assert_globs(&scratch, "{case_sensitive: true}", &[("*.RS", &[])]);
}

#[test]
fn glob_with_a_trailing_separator_matches_directories() {
    let scratch = glob_tree("glob_directories");
    // A symlink to a directory counts; the path drops the separator.
    assert_globs(&scratch, "", &[("*/", &["linked", "src"]), ("a.rs/", &[])]);
    assert_globs(&scratch, "", &[("src/", &["src"])]);
}

#[test]
fn glob_follows_the_directories_a_pattern_writes_out() {
    let scratch = glob_tree("glob_literal_prefix");
    // Written out, a symlink to a directory is searched like a directory.
    assert_globs(
        &scratch,
        "",
        &[("linked/*.rs", &["linked/c.rs", "linked/d.rs"])],
    );
}

#[test]
fn glob_without_wildcards_names_one_path() {
    let scratch = glob_tree("glob_literal");
    assert_globs(
        &scratch,
        "",
        &[
            ("a.rs", &["a.rs"]),
            ("src/sub", &["src/sub"]),
            ("missing", &[]),
            ("src/loop", &["src/loop"]),
        ],
    );
    assert_eq!(scratch.run("fs.glob('')"), frost("[]"), "an empty pattern");
}

#[test]
fn glob_finds_nothing_below_what_is_not_a_directory() {
    let scratch = glob_tree("glob_missing");
    assert_globs(&scratch, "", &[("missing/*", &[]), ("a.rs/*", &[])]);
}

#[test]
fn glob_keeps_a_relative_pattern_relative() {
    // Relative to the working directory, which tests run in: this crate's root.
    let scratch = Scratch::new("glob_relative");
    assert_eq!(
        scratch.run("fs.glob('Cargo.tom?')"),
        frost("['Cargo.toml']")
    );
    assert_eq!(
        scratch.run("fs.glob('./src/stdlib/f?.rs')"),
        frost("['./src/stdlib/fs.rs']")
    );
}

#[test]
fn glob_checks_its_pattern_and_options() {
    let scratch = Scratch::new("glob_arguments");
    let raised = scratch.raises("fs.glob('[a')");
    assert!(
        raised.starts_with(r#"Function fs.glob got an invalid pattern "[a": "#),
        "{raised}"
    );
    assert_eq!(
        scratch.raises("fs.glob('*', {hiden: true})"),
        "Function fs.glob requires valid options: unknown field `hiden`, expected \
         `hidden` or `case_sensitive` (at a key)"
    );
    assert_eq!(
        scratch.raises("fs.glob('*', {hidden: 1})"),
        "Function fs.glob requires valid options: expected Bool, got Int (at `hidden`)"
    );
    assert_eq!(
        scratch.raises("fs.glob(1)"),
        "Function fs.glob requires String as argument 1 (pattern), got Int"
    );
    assert_eq!(
        scratch.raises("fs.glob('*', true)"),
        "Function fs.glob requires Map as argument 2 (options), got Bool"
    );
    for argc in [0, 3] {
        let args = vec!["'*'"; argc].join(", ");
        assert_eq!(
            scratch.raises(&format!("fs.glob({args})")),
            format!(
                "Function fs.glob expects between 1 and 2 arguments, but was called with {argc}"
            )
        );
    }
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
fn move_replaces_a_file() {
    let scratch = Scratch::new("move_replace");
    scratch.file("old", "old");
    // An update made whole: write a new file, then move it over the old one.
    let source = r"
        def old = fs.concat(dir, 'old')
        def new = fs.concat(dir, 'new')
        fs.write(new, 'replaced')
        fs.move(new, old)
        def result = [fs.read(old), fs.exists(new)]
        fs.write(old, 'old')
        result
    ";
    assert_eq!(scratch.run(source), frost("['replaced', false]"));
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
fn copy_follows_a_link_at_the_top() {
    let scratch = Scratch::new("copy_top_link");
    scratch
        .file("file", "hi")
        .link("link", scratch.path("file"));
    let source = r"
        def to = fs.concat(dir, 'to')
        fs.copy(fs.concat(dir, 'link'), to)
        def result = [fs.is_symlink(to), fs.is_file(to), fs.read(to)]
        fs.remove(to)
        result
    ";
    assert_eq!(scratch.run(source), frost("[false, true, 'hi']"));
}

#[test]
fn copy_merges_a_tree_into_an_existing_directory() {
    let scratch = Scratch::new("copy_merge");
    scratch
        .file("tree/a", "a")
        .file("tree/sub/b", "b")
        .file("copy/keep", "k");
    let source = r"
        def to = fs.concat(dir, 'copy')
        fs.copy(fs.concat(dir, 'tree'), to)
        def result = [
            map fs.list(to) with fs.filename,
            fs.read(fs.concat(to, 'sub/b')),
            fs.read(fs.concat(to, 'keep')),
        ]
        fs.remove(fs.concat(to, 'a'))
        fs.remove_recursively(fs.concat(to, 'sub'))
        result
    ";
    assert_eq!(
        scratch.run(source),
        frost("[['a', 'keep', 'sub'], 'b', 'k']")
    );
}

#[test]
fn copy_refuses_to_overwrite_a_file_inside_a_tree() {
    let scratch = Scratch::new("copy_nested_collision");
    // Entries copied before and after the collision, were nothing checked first.
    scratch
        .file("tree/a", "a")
        .file("tree/sub/b", "new")
        .file("tree/z", "z")
        .file("copy/sub/b", "old");
    assert_eq!(
        scratch.raises("fs.copy(fs.concat(dir, 'tree'), fs.concat(dir, 'copy'))"),
        format!(
            "Function fs.copy failed on `{}`: `{}` already exists",
            scratch.text("tree"),
            scratch.text("copy/sub/b")
        )
    );
    assert_eq!(
        fs::read_to_string(scratch.path("copy/sub/b")).unwrap(),
        "old",
        "untouched"
    );
    assert_eq!(
        scratch.run("fs.list_recursively(fs.concat(dir, 'copy'))"),
        Value::array([scratch.text("copy/sub"), scratch.text("copy/sub/b")]),
        "a copy that collides copies nothing"
    );
}

#[test]
fn copy_refuses_anything_in_the_way() {
    let scratch = Scratch::new("copy_in_the_way");
    scratch
        .file("tree/file", "x")
        .file("tree/dir/inner", "x")
        .dir("copy_dir_at_file/file")
        .file("copy_file_at_dir/dir", "")
        .file("copy_link_at_file/keep", "");
    scratch.link("copy_link_at_file/file", scratch.path("gone"));
    for (destination, in_the_way) in [
        // A directory where a file would go.
        ("copy_dir_at_file", "copy_dir_at_file/file"),
        // A file where a directory would go.
        ("copy_file_at_dir", "copy_file_at_dir/dir"),
        // A broken link where a file would go.
        ("copy_link_at_file", "copy_link_at_file/file"),
    ] {
        assert_eq!(
            scratch.raises(&format!(
                "fs.copy(fs.concat(dir, 'tree'), fs.concat(dir, '{destination}'))"
            )),
            format!(
                "Function fs.copy failed on `{}`: `{}` already exists",
                scratch.text("tree"),
                scratch.text(in_the_way)
            ),
            "{destination}"
        );
    }
    assert!(
        !scratch.path("copy_link_at_file/dir").exists(),
        "a copy that collides copies nothing"
    );
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
        def file = fs.concat(dir, 'file')
        def target = fs.concat(dir, 'target')
        fs.mkdir(empty)
        fs.symlink(target, link)
        fs.write(file, 'x')
        [
            fs.remove(empty),
            fs.remove(link),
            fs.exists(target),
            fs.remove(file),
            fs.exists(file),
            fs.remove(fs.concat(dir, 'missing')),
        ]
    ";
    assert_eq!(
        scratch.run(source),
        frost("[true, true, true, true, false, false]")
    );
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
        def lone = fs.concat(dir, 'lone')
        def link = fs.concat(dir, 'link')
        fs.write(lone, 'x')
        fs.symlink(fs.concat(dir, 'kept'), link)
        [
            fs.remove_recursively(to),
            fs.exists(to),
            fs.exists(fs.concat(dir, 'kept/c')),
            fs.remove_recursively(fs.concat(dir, 'missing')),
            fs.remove_recursively(lone),
            fs.exists(lone),
            fs.remove_recursively(link),
            fs.exists(link),
            fs.exists(fs.concat(dir, 'kept/c')),
        ]
    ";
    // The copy holds itself, `a`, `sub`, `sub/b`, and the link `to_kept`.
    assert_eq!(
        scratch.run(source),
        frost("[5, false, true, 0, 1, false, 1, false, true]")
    );
}

// --- Reading and writing ---
//
// What a stream does is tested through `std.string`'s buffers, which share the
// implementation; these cases cover what is particular to files.

#[test]
fn read_reads_a_whole_file() {
    let scratch = Scratch::new("read");
    scratch.file("text", "h\u{e9}llo\n");
    fs::write(scratch.path("binary"), [0xff]).expect("the file is written");
    let source = r"
        def text = fs.concat(dir, 'text')
        [fs.read(text), fs.read_bytes(text), fs.read_bytes(fs.concat(dir, 'binary'))]
    ";
    assert_eq!(
        scratch.run(source),
        frost(r"['h\u{e9}llo\n', x'68c3a96c6c6f0a', x'ff']")
    );
    assert_eq!(
        scratch.raises("fs.read(fs.concat(dir, 'binary'))"),
        format!(
            "Function fs.read read text that is not UTF-8 from `{}`",
            scratch.text("binary")
        )
    );
    scratch.dir("directory");
    for function in ["read", "read_bytes"] {
        assert_io_error(
            &scratch.raises(&format!("fs.{function}(fs.concat(dir, 'missing'))")),
            &format!("fs.{function}"),
            &scratch.text("missing"),
        );
        assert_io_error(
            &scratch.raises(&format!("fs.{function}(fs.concat(dir, 'directory'))")),
            &format!("fs.{function}"),
            &scratch.text("directory"),
        );
    }
}

#[test]
fn write_replaces_and_append_extends() {
    let scratch = Scratch::new("write");
    let source = r"
        def path = fs.concat(dir, 'new')
        def written = [fs.write(path, 'one\n'), fs.read(path)]
        def appended = [fs.append(path, x'74776f'), fs.read(path)]
        def replaced = [fs.write(path, 'three'), fs.read(path)]
        fs.remove(path)
        def created = [fs.append(path, 'four'), fs.read(path)]
        fs.remove(path)
        [written, appended, replaced, created]
    ";
    assert_eq!(
        scratch.run(source),
        frost(r"[[null, 'one\n'], [null, 'one\ntwo'], [null, 'three'], [null, 'four']]")
    );
    for function in ["write", "append"] {
        assert_io_error(
            &scratch.raises(&format!(
                "fs.{function}(fs.concat(dir, 'missing/file'), '')"
            )),
            &format!("fs.{function}"),
            &scratch.text("missing/file"),
        );
    }
}

#[test]
fn file_streams_close_and_append_streams_have_no_position() {
    let scratch = Scratch::new("stream_keys");
    scratch.file("file", "");
    let source = r"
        def path = fs.concat(dir, 'new')
        def r = fs.open_read(fs.concat(dir, 'file'))
        def w = fs.open_write(path)
        def a = fs.open_append(path)
        def shapes = [sorted(keys(r)), sorted(keys(w)), sorted(keys(a))]
        r.close()
        w.close()
        a.close()
        fs.remove(path)
        shapes
    ";
    assert_eq!(
        scratch.run(source),
        frost(
            "[
                ['close', 'eof', 'is_open', 'read_bytes', 'read_line', 'read_one', 'read_rest', \
                 'read_rest_bytes', 'seek', 'tell'],
                ['close', 'flush', 'is_open', 'seek', 'tell', 'write', 'writeln'],
                ['close', 'flush', 'is_open', 'write', 'writeln'],
            ]"
        )
    );
}

#[test]
fn open_read_reads_a_file_until_closed() {
    let scratch = Scratch::new("open_read");
    scratch.file("file", "one\ntwo\nthree");
    let source = r"
        def r = fs.open_read(fs.concat(dir, 'file'))
        def first = r.read_line()
        def position = r.tell()
        r.seek(0)
        def again = r.read_line()
        def rest = r.read_rest()
        def open = r.is_open()
        r.close()
        r.close()
        [first, position, again, rest, open, r.is_open()]
    ";
    assert_eq!(
        scratch.run(source),
        frost(r"['one', 4, 'one', 'two\nthree', true, false]")
    );
    for call in ["read_line()", "eof()", "tell()", "seek(0)"] {
        let function = call.split('(').next().expect("a call has a name");
        let source = format!(
            r"
            def r = fs.open_read(fs.concat(dir, 'file'))
            r.close()
            r.{call}
            "
        );
        assert_eq!(
            scratch.raises(&source),
            format!("Function reader.{function} requires an open stream, but it is closed")
        );
    }
    assert_io_error(
        &scratch.raises("fs.open_read(fs.concat(dir, 'missing'))"),
        "fs.open_read",
        &scratch.text("missing"),
    );
}

#[test]
fn open_write_empties_a_file_and_writes_it() {
    let scratch = Scratch::new("open_write");
    scratch.file("file", "old content");
    let source = r"
        def path = fs.concat(dir, 'file')
        def w = fs.open_write(path)
        def emptied = fs.read(path)
        w.write('hello')
        w.writeln(x'21')
        def position = w.tell()
        w.seek(0)
        w.write('J')
        w.close()
        def written = fs.read(path)
        fs.write(path, 'old content')
        [emptied, position, written, w.is_open()]
    ";
    assert_eq!(scratch.run(source), frost(r"['', 7, 'Jello!\n', false]"));
    let source = r"
        def w = fs.open_write(fs.concat(dir, 'file'))
        w.close()
        fs.write(fs.concat(dir, 'file'), 'old content')
        w.write('late')
    ";
    assert_eq!(
        scratch.raises(source),
        "Function writer.write requires an open stream, but it is closed"
    );
    assert_io_error(
        &scratch.raises("fs.open_write(fs.concat(dir, 'missing/file'))"),
        "fs.open_write",
        &scratch.text("missing/file"),
    );
}

#[test]
fn open_append_writes_at_the_end_of_a_file() {
    let scratch = Scratch::new("open_append");
    scratch.file("file", "start");
    let source = r"
        def path = fs.concat(dir, 'file')
        def a = fs.open_append(path)
        a.write(' more')
        a.flush()
        def flushed = fs.read(path)
        a.writeln(' end')
        a.close()
        def closed = fs.read(path)
        fs.write(path, 'start')
        def new = fs.concat(dir, 'new')
        def b = fs.open_append(new)
        b.write('fresh')
        b.close()
        def created = fs.read(new)
        fs.remove(new)
        [flushed, closed, created]
    ";
    assert_eq!(
        scratch.run(source),
        frost(r"['start more', 'start more end\n', 'fresh']")
    );
    assert_io_error(
        &scratch.raises("fs.open_append(fs.concat(dir, 'missing/file'))"),
        "fs.open_append",
        &scratch.text("missing/file"),
    );
}

#[test]
fn a_closed_file_stream_refuses_every_operation_but_close_and_is_open() {
    let scratch = Scratch::new("closed");
    scratch.file("file", "content");
    for call in [
        "read_one()",
        "read_rest()",
        "read_bytes(1)",
        "read_rest_bytes()",
    ] {
        let member = call.split('(').next().expect("a call has a name");
        let source = format!(
            r"
            def r = fs.open_read(fs.concat(dir, 'file'))
            r.close()
            r.{call}
            "
        );
        assert_eq!(
            scratch.raises(&source),
            format!("Function reader.{member} requires an open stream, but it is closed")
        );
    }
    for call in ["writeln('x')", "tell()", "seek(0)", "flush()"] {
        let member = call.split('(').next().expect("a call has a name");
        let source = format!(
            r"
            def w = fs.open_write(fs.concat(dir, 'out'))
            w.close()
            w.{call}
            "
        );
        assert_eq!(
            scratch.raises(&source),
            format!("Function writer.{member} requires an open stream, but it is closed")
        );
    }
    let source = r"
        def w = fs.open_write(fs.concat(dir, 'out'))
        w.close()
        w.close()
        w.is_open()
    ";
    assert_eq!(scratch.run(source), Value::Bool(false));
}

#[test]
fn file_stream_members_check_their_arguments() {
    let scratch = Scratch::new("stream_arguments");
    scratch.file("file", "content");
    for member in [
        "close",
        "is_open",
        "read_line",
        "read_one",
        "read_rest",
        "read_rest_bytes",
        "eof",
        "tell",
    ] {
        let source = format!(
            r"
            def r = fs.open_read(fs.concat(dir, 'file'))
            r.{member}(1)
            "
        );
        assert_eq!(
            scratch.raises(&source),
            format!("Function reader.{member} expects 0 arguments, but was called with 1")
        );
    }
    for member in ["close", "is_open", "flush", "tell"] {
        let source = format!(
            r"
            def w = fs.open_write(fs.concat(dir, 'out'))
            w.{member}(1)
            "
        );
        assert_eq!(
            scratch.raises(&source),
            format!("Function writer.{member} expects 0 arguments, but was called with 1")
        );
    }
    for (stream, open, kind) in [
        ("r", "fs.open_read(fs.concat(dir, 'file'))", "reader"),
        ("w", "fs.open_write(fs.concat(dir, 'out'))", "writer"),
    ] {
        for (argument, problem) in [
            ("'0'", "requires Int as argument 1, got String"),
            ("-1", "requires argument 1 to be at least 0, got -1"),
        ] {
            let source = format!(
                r"
                def {stream} = {open}
                {stream}.seek({argument})
                "
            );
            assert_eq!(
                scratch.raises(&source),
                format!("Function {kind}.seek {problem}")
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn closing_a_writer_reports_a_failure_to_write_out_and_still_closes_it() {
    let scratch = Scratch::new("full");
    // `/dev/full` accepts a buffered write, then fails the flush as out of space.
    let raised = scratch.raises(
        r"
        def w = fs.open_write('/dev/full')
        w.write('x')
        w.close()
        ",
    );
    assert!(
        raised.starts_with("Function writer.close failed: "),
        "{raised}"
    );
    let source = r"
        def w = fs.open_write('/dev/full')
        w.write('x')
        def closed = try_call(w.close)
        [closed.ok, w.is_open()]
    ";
    assert_eq!(scratch.run(source), frost("[false, false]"));
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
        "read",
        "read_bytes",
        "open_read",
        "open_write",
        "open_append",
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
            scratch.raises(&format!("fs.{function}(x'62', 'a')")),
            format!("Function fs.{function} requires String as argument 1, got Bytes")
        );
        assert_eq!(
            scratch.raises(&format!("fs.{function}('a', x'62')")),
            format!("Function fs.{function} requires String as argument 2, got Bytes")
        );
        for argc in [0, 1, 3] {
            let args = vec!["'a'"; argc].join(", ");
            assert_eq!(
                scratch.raises(&format!("fs.{function}({args})")),
                format!("Function fs.{function} expects 2 arguments, but was called with {argc}")
            );
        }
    }
    for function in ["write", "append"] {
        assert_eq!(
            scratch.raises(&format!("fs.{function}(1, 'a')")),
            format!("Function fs.{function} requires String as argument 1 (path), got Int")
        );
        assert_eq!(
            scratch.raises(&format!("fs.{function}('a', 1)")),
            format!(
                "Function fs.{function} requires String or Bytes as argument 2 (content), got Int"
            )
        );
        for argc in [1, 3] {
            let args = vec!["'a'"; argc].join(", ");
            assert_eq!(
                scratch.raises(&format!("fs.{function}({args})")),
                format!("Function fs.{function} expects 2 arguments, but was called with {argc}")
            );
        }
    }
    assert_eq!(
        scratch.raises("fs.concat('a', 1)"),
        "Function fs.concat requires String as argument 2 (path), got Int"
    );
    assert_eq!(
        scratch.raises("fs.concat(1, 'a')"),
        "Function fs.concat requires String as argument 1 (base), got Int"
    );
    for argc in [0, 1, 3] {
        let args = vec!["'a'"; argc].join(", ");
        assert_eq!(
            scratch.raises(&format!("fs.concat({args})")),
            format!("Function fs.concat expects 2 arguments, but was called with {argc}")
        );
    }
    assert_eq!(
        scratch.raises("fs.cwd(1)"),
        "Function fs.cwd expects 0 arguments, but was called with 1"
    );
}
