//! [`ReplSettings`]: how a driver starts its interactive sessions.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use frostlang::Value;
use frostlang_repl::{Frontend, InvalidName, Repl};

/// How a [`Driver`](crate::Driver)'s interactive sessions start: their
/// [`Frontend`], and what is bound before the first input.
///
/// Every session starts afresh from these settings.
#[derive(Debug, Clone)]
pub struct ReplSettings {
    // `None` for the default frontend, which takes the command line's color.
    frontend: Option<FrontendFactory>,
    bindings: BTreeMap<String, Value>,
    // `None` for the `Repl`'s own default.
    results_kept: Option<usize>,
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
    /// line's `--color` says, with nothing bound, and keeping as many recent
    /// results as [`Repl::new`] does.
    pub fn new() -> Self {
        Self {
            frontend: None,
            bindings: BTreeMap::new(),
            results_kept: None,
        }
    }

    /// Set how many recent results each session keeps, as
    /// [`Repl::with_results_kept`] does.
    pub fn with_results_kept(mut self, count: usize) -> Self {
        self.results_kept = Some(count);
        self
    }

    /// Set the frontend sessions run on: `make` is called for a new frontend
    /// at the start of each session. The command line's `--color` does not
    /// apply to it.
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

    /// A new session's frontend, colored if `color` is true and it is the
    /// default, and `repl` with these settings' bindings and recent results.
    pub(crate) fn start(&self, repl: Repl, color: bool) -> (Box<dyn Frontend>, Repl) {
        let mut repl = repl
            .with_bindings(self.bindings.clone())
            .expect("each name was checked when it was bound");
        if let Some(count) = self.results_kept {
            repl = repl.with_results_kept(count);
        }
        let frontend = match &self.frontend {
            Some(make) => (make.0)(),
            None => frostlang_repl::default_frontend(color),
        };
        (frontend, repl)
    }
}
