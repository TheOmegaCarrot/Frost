//! [`ReplSettings`]: how a driver starts its interactive sessions.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use frost_repl::{Frontend, InvalidName, Repl};
use frost_runtime::Value;

/// How a [`Driver`](crate::Driver)'s interactive sessions start: their
/// [`Frontend`], and what is bound before the first input.
///
/// Every session starts afresh from these settings.
#[derive(Debug, Clone)]
pub struct ReplSettings {
    // `None` for the default frontend, which takes the command line's color.
    frontend: Option<FrontendFactory>,
    bindings: BTreeMap<String, Value>,
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
    /// Sessions on [`frost_repl::default_frontend`], colored as the command
    /// line's `--color` says, with nothing bound.
    pub fn new() -> Self {
        Self {
            frontend: None,
            bindings: BTreeMap::new(),
        }
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
    /// refer to (see [`frost_repl::check_name`]).
    pub fn with_binding(
        mut self,
        name: impl Into<String>,
        value: Value,
    ) -> Result<Self, InvalidName> {
        let name = name.into();
        frost_repl::check_name(&name)?;
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
    /// default, and `repl` with these settings' bindings.
    pub(crate) fn start(&self, repl: Repl, color: bool) -> (Box<dyn Frontend>, Repl) {
        let repl = repl
            .with_bindings(self.bindings.clone())
            .expect("each name was checked when it was bound");
        let frontend = match &self.frontend {
            Some(make) => (make.0)(),
            None => frost_repl::default_frontend(color),
        };
        (frontend, repl)
    }
}
