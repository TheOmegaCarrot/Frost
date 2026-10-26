//! The [`Repl`](crate::Repl)'s [config file](crate::Repl#config-file):
//! reading it, and the settings in its `session` section.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use frostlang::compile::{CompilerOptions, Diagnostics, compile_in_scope};
use frostlang::{FrostError, Nullable, Value, from_value};
use serde::Deserialize;

use crate::config_runner::{ConfigRunner, ScriptFailure};
use crate::{Frontend, InvalidName, Repl, ReplError, check_name};

/// The export the REPL takes its own settings from.
const SESSION: &str = "session";

/// Whether the config file is read as a session starts or starts over (at
/// `:reset`), or reloaded (at `:config reload`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reading {
    Start,
    Reload,
}

impl Repl {
    /// Read the config file at `path`, apply what it sets, and show any
    /// problem on `frontend`. If it sets nothing, for an error, nothing
    /// changes.
    pub(crate) fn read_config(
        &mut self,
        path: &Path,
        frontend: &mut dyn Frontend,
        reading: Reading,
    ) -> io::Result<()> {
        let mut exports = match load(path, frontend.ansi_styling()) {
            Ok(exports) => exports,
            Err(error) => return frontend.render(Err(&ReplError::Config(error))),
        };
        let mut take = |key: &str| {
            let at = exports.iter().position(|(name, _)| name == key)?;
            Some(exports.swap_remove(at).1)
        };
        let mut problems = Vec::new();
        let settings = SessionSettings::from_section(take(SESSION)).unwrap_or_else(|problem| {
            problems.push(problem);
            SessionSettings::default()
        });
        let section = frontend.config_key().and_then(take);
        self.config_results_kept = chosen(settings.results_kept);
        self.forget_excess_results();
        if reading == Reading::Start {
            for (Name(name), path) in chosen(settings.preload).unwrap_or_default() {
                if let Err(error) = self.preload(&name, &path) {
                    problems.push(Problem::PreloadFailed { name, error }.into());
                }
            }
        }
        for problem in problems {
            frontend.render(Err(&ReplError::Config(problem)))?;
        }
        frontend.configure(section)
    }

    /// Bind `name` to the module at `path`, imported as an input imports it.
    fn preload(&mut self, name: &str, path: &str) -> Result<(), FrostError> {
        let program = compile_in_scope(
            "<preload>",
            "import(path)",
            CompilerOptions::new(),
            &["path"],
        )
        .expect("the import compiles")
        .code;
        let closure = program
            .close([("path", Value::from(path))])
            .expect("the import captures only `path`");
        let token = self.cancel.begin();
        let outcome = self.run_closure(closure, token);
        self.cancel.end();
        match outcome {
            Ok((module, _)) => {
                self.bindings.insert(name.to_owned(), module);
                Ok(())
            }
            Err(ReplError::Run(error) | ReplError::Cancelled(error)) => Err(error),
            Err(other) => {
                unreachable!("a run fails only by raising or by being cancelled: {other}")
            }
        }
    }
}

/// The configuration file in the platform's configuration directory, if it
/// has one.
pub(crate) fn default_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("frost").join("repl.frst"))
}

/// The exports of the configuration in the file at `path`, run with
/// `repl.colors` and `repl.style` coloring only if `color` is true. With no
/// file there, there are none.
fn load(path: &Path, color: bool) -> Result<Vec<(String, Value)>, ConfigError> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(ConfigError::Unreadable {
                path: path.to_owned(),
                error: Arc::new(error),
            });
        }
    };
    ConfigRunner::new(color)
        .run_script(&path.display().to_string(), &source)
        .map_err(|failure| match failure {
            ScriptFailure::Compile(diagnostics) => ConfigError::Compile(diagnostics),
            ScriptFailure::Run(error) => ConfigError::Run(error),
        })
}

/// What the `session` section sets. Read each setting through [`chosen`].
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionSettings {
    results_kept: Option<Nullable<usize>>,
    /// Each name to bind, and the path to import it from.
    preload: Option<Nullable<BTreeMap<Name, String>>>,
}

/// The value a setting of the config file chooses: none if it is left out or
/// Null, either of which keeps the default. The serde bridge tells the two
/// apart; the config takes them alike, so that a script can leave a setting
/// to its default with a Null.
pub(crate) fn chosen<T>(setting: Option<Nullable<T>>) -> Option<T> {
    setting.and_then(Option::from)
}

impl SessionSettings {
    /// What `section`, the `session` export if there is one, sets.
    fn from_section(section: Option<Value>) -> Result<Self, ConfigError> {
        section.map_or_else(
            || Ok(Self::default()),
            |section| from_value(section).map_err(|error| Problem::Invalid(error).into()),
        )
    }
}

/// A name Frost source can refer to.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
struct Name(String);

impl TryFrom<String> for Name {
    type Error = InvalidName;

    fn try_from(name: String) -> Result<Self, InvalidName> {
        check_name(&name).map(|()| Self(name))
    }
}

/// Something wrong with the [config file](crate::Repl#config-file). The
/// session carries on without the part of it that is wrong.
///
/// [`Display`](fmt::Display) shows it as plain text, saying whether it is an
/// error or a warning.
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// The file is there, but cannot be read, so it sets nothing.
    Unreadable {
        /// The file.
        path: PathBuf,
        /// Why it cannot be read.
        error: Arc<io::Error>,
    },
    /// The configuration did not compile, so it sets nothing.
    Compile(Diagnostics),
    /// The configuration raised an error while running, so it sets nothing.
    Run(FrostError),
    /// The `session` section is wrong, so it sets nothing, or a module it
    /// preloads failed to import.
    Session(SessionProblem),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, error } => write!(
                f,
                "Error: cannot read the REPL configuration {}: {error}",
                path.display()
            ),
            #[cfg(feature = "graphical-diagnostics")]
            Self::Compile(diagnostics) => f.write_str(diagnostics.render_plain().trim_end()),
            #[cfg(not(feature = "graphical-diagnostics"))]
            Self::Compile(diagnostics) => f.write_str(diagnostics.render_narrated().trim_end()),
            Self::Run(error) => write!(
                f,
                "Error: the REPL configuration failed: {}",
                error.with_backtrace()
            ),
            Self::Session(problem) => write!(f, "{problem}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unreadable { error, .. } => Some(&**error),
            Self::Compile(diagnostics) => Some(diagnostics),
            Self::Run(error) => Some(error),
            Self::Session(_) => None,
        }
    }
}

/// What is wrong with the [config file](crate::Repl#config-file)'s `session`
/// section; [`Display`](fmt::Display) says what.
#[derive(Debug, Clone)]
pub struct SessionProblem(Problem);

#[derive(Debug, Clone)]
enum Problem {
    /// The section does not hold the settings, as their types say.
    Invalid(FrostError),
    /// A module to preload failed to import.
    PreloadFailed { name: String, error: FrostError },
}

impl From<Problem> for ConfigError {
    fn from(problem: Problem) -> Self {
        Self::Session(SessionProblem(problem))
    }
}

impl fmt::Display for SessionProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Problem::Invalid(error) => write!(
                f,
                "Warning: the REPL configuration's `{SESSION}` sets nothing: {error}"
            ),
            Problem::PreloadFailed { name, error } => write!(
                f,
                "Error: the REPL configuration's `session.preload.{name}` failed to import: {}",
                error.with_backtrace()
            ),
        }
    }
}
