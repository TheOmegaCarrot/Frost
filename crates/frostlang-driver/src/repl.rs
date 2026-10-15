//! [`ReplSettings`]: how a driver starts its interactive sessions.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use frostlang::Value;
use frostlang_repl::{Frontend, InvalidName, Repl};

/// How a [`Driver`](crate::Driver)'s interactive sessions start: their
/// [`Frontend`], their [config file](Repl#config-file), and what is bound
/// before the first input.
///
/// Every session starts afresh from these settings.
#[derive(Debug, Clone)]
pub struct ReplSettings {
    // `None` for the default frontend, which takes the command line's color.
    frontend: Option<FrontendFactory>,
    config_file: ConfigFile,
    bindings: BTreeMap<String, Value>,
    // `None` for the `Repl`'s own default.
    results_kept: Option<usize>,
}

/// Which config file sessions read.
#[derive(Debug, Clone)]
enum ConfigFile {
    Default,
    Path(PathBuf),
    None,
}

/// Makes the frontend for each session.
#[derive(Clone)]
struct FrontendFactory(Arc<dyn Fn() -> Box<dyn Frontend> + Send + Sync>);

impl fmt::Debug for FrontendFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrontendFactory").finish_non_exhaustive()
    }
}

impl Default for ReplSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplSettings {
    /// Sessions on [`frostlang_repl::default_frontend`], colored as the command
    /// line's `--color` says, reading the default config file (see
    /// [`Repl::with_default_config_file`]), with nothing bound, and keeping as
    /// many recent results as [`Repl::new`] does. With `repl --basic`,
    /// sessions are on [`LineFrontend::stdin`](frostlang_repl::LineFrontend::stdin)
    /// instead. With `repl --no-config`, they read no config file.
    pub fn new() -> Self {
        Self {
            frontend: None,
            config_file: ConfigFile::Default,
            bindings: BTreeMap::new(),
            results_kept: None,
        }
    }

    /// Have sessions read the config file at `path` instead, as
    /// [`Repl::with_config_file`] does.
    pub fn with_config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_file = ConfigFile::Path(path.into());
        self
    }

    /// Have sessions read no config file.
    pub fn without_config(mut self) -> Self {
        self.config_file = ConfigFile::None;
        self
    }

    /// Set how many recent results each session keeps, as
    /// [`Repl::with_results_kept`] does.
    pub fn with_results_kept(mut self, count: usize) -> Self {
        self.results_kept = Some(count);
        self
    }

    /// Set the frontend sessions run on: `make` is called for a new frontend
    /// at the start of each session. The command line's `--color` and
    /// `repl --basic` do not apply to it.
    pub fn with_frontend(
        mut self,
        make: impl Fn() -> Box<dyn Frontend> + Send + Sync + 'static,
    ) -> Self {
        self.frontend = Some(FrontendFactory(Arc::new(make)));
        self
    }

    /// Bind `name` to `value` at the start of every session, as
    /// [`Repl::with_binding`] does. Fails if `name` is not one Frost source can
    /// refer to (see [`frostlang_repl::check_name`]).
    pub fn with_binding(
        mut self,
        name: impl Into<String>,
        value: Value,
    ) -> Result<Self, InvalidName> {
        let name = name.into();
        frostlang_repl::check_name(&name)?;
        self.bindings.insert(name, value);
        Ok(self)
    }

    /// [`with_binding`](Self::with_binding) for each of `bindings` in turn: a
    /// name given twice takes its last value.
    pub fn with_bindings<N: Into<String>>(
        self,
        bindings: impl IntoIterator<Item = (N, Value)>,
    ) -> Result<Self, InvalidName> {
        bindings
            .into_iter()
            .try_fold(self, |settings, (name, value)| {
                settings.with_binding(name, value)
            })
    }

    /// A new session's frontend, made by `default_frontend` if these settings
    /// give none, and `repl` with these settings' bindings, recent results,
    /// and, if `config` is true, config file.
    pub(crate) fn start(
        &self,
        repl: Repl,
        default_frontend: impl FnOnce() -> Box<dyn Frontend>,
        config: bool,
    ) -> (Box<dyn Frontend>, Repl) {
        let mut repl = repl
            .with_bindings(self.bindings.clone())
            .expect("each name was checked when it was bound");
        if let Some(count) = self.results_kept {
            repl = repl.with_results_kept(count);
        }
        match &self.config_file {
            _ if !config => {}
            ConfigFile::Default => repl = repl.with_default_config_file(),
            ConfigFile::Path(path) => repl = repl.with_config_file(path.clone()),
            ConfigFile::None => {}
        }
        let frontend = match &self.frontend {
            Some(make) => (make.0)(),
            None => default_frontend(),
        };
        (frontend, repl)
    }
}
