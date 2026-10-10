//! Harness for the resolver's tests: a scratch directory tree of module files,
//! and a script run with an importer that serves them.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{
    FrostError, Importer, ImporterBuilder, RunError, Value, Vm, VmRuntimeConfiguration,
};
use frostlang_fs_resolver::FsResolver;

/// A fresh, empty directory for one test's files, named for the test.
pub(crate) struct Tree {
    root: PathBuf,
}

impl Tree {
    /// An empty tree named `name`, unique to the calling test.
    pub(crate) fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("frostlang-fs-resolver-tests")
            .join(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the scratch directory is creatable");
        Self { root }
    }

    /// `relative` within the tree.
    pub(crate) fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Write `contents` to the file at `relative`, creating its directories.
    pub(crate) fn file(&self, relative: &str, contents: impl AsRef<[u8]>) -> &Self {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("a file has a parent"))
            .expect("the file's directory is creatable");
        fs::write(&path, contents).expect("the file is writable");
        self
    }

    /// Create the directory at `relative`.
    pub(crate) fn dir(&self, relative: &str) -> &Self {
        fs::create_dir_all(self.path(relative)).expect("the directory is creatable");
        self
    }

    /// Create a symbolic link at `link` to `target`, both within the tree.
    #[cfg(unix)]
    pub(crate) fn symlink(&self, link: &str, target: &str) -> &Self {
        let link = self.path(link);
        fs::create_dir_all(link.parent().expect("a link has a parent"))
            .expect("the link's directory is creatable");
        std::os::unix::fs::symlink(self.path(target), link).expect("the link is creatable");
        self
    }

    /// A resolver searching `roots`, each relative to the tree.
    pub(crate) fn resolver(&self, roots: &[&str]) -> FsResolver {
        FsResolver::new(roots.iter().map(|root| self.path(root)))
            .expect("every root is a directory")
    }
}

/// An importer with `resolver` as its only resolver.
pub(crate) fn importer(resolver: FsResolver) -> Arc<Importer> {
    ImporterBuilder::new()
        .append_resolver(Arc::new(resolver))
        .build()
}

/// What running a script observed: its tail value or error, and what it printed.
#[derive(Debug)]
pub(crate) struct Run {
    pub(crate) outcome: Result<Value, FrostError>,
    pub(crate) printed: Vec<String>,
}

impl Run {
    /// The tail value of a run that must succeed.
    pub(crate) fn value(self) -> Value {
        self.outcome
            .unwrap_or_else(|error| panic!("the script should run, but raised: {error}"))
    }

    /// The error of a run that must raise.
    pub(crate) fn error(self) -> FrostError {
        match self.outcome {
            Ok(value) => panic!("the script should raise, but produced {value:?}"),
            Err(error) => error,
        }
    }

    /// The error message of a run that must raise.
    pub(crate) fn message(self) -> String {
        self.error().message().into_owned()
    }
}

/// Run `source` as the top-level script `main.frst` with `importer`.
pub(crate) fn run(source: &str, importer: &Arc<Importer>) -> Run {
    let closure = compile_program("main.frst", source, CompilerOptions::new())
        .unwrap_or_else(|errors| panic!("the script compiles:\n{}", errors.render_plain()))
        .code
        .into_closure()
        .expect("the script captures nothing");
    let printed = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let printed = Arc::clone(&printed);
        move |text: &str| printed.lock().unwrap().push(text.to_string())
    };
    let configuration = VmRuntimeConfiguration::default().with_print_sink(Arc::new(sink));
    let outcome = Vm::factory()
        .configuration(configuration)
        .with_importer(Arc::clone(importer))
        .build(closure)
        .run()
        .map_err(RunError::into_error)
        .map(|result| result.tail().clone());
    let printed = printed.lock().unwrap().clone();
    Run { outcome, printed }
}

/// How many of `printed` are exactly `line`.
pub(crate) fn count(printed: &[String], line: &str) -> usize {
    printed.iter().filter(|printed| *printed == line).count()
}
