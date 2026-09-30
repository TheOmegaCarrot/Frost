//! [`ReplSettings`]: how a driver starts its interactive sessions.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use frost_repl::{InvalidName, Repl, ReplInput};
use frost_runtime::Value;

/// How a [`Driver`](crate::Driver)'s interactive sessions start: where they read
/// their input, and what is bound before the first input.
///
/// Every session starts afresh from these settings.
#[derive(Debug, Clone)]
pub struct ReplSettings {
    input: InputFactory,
    bindings: BTreeMap<String, Value>,
}

/// Makes the input for each session.
#[derive(Clone)]
struct InputFactory(Arc<dyn Fn() -> Box<dyn ReplInput> + Send + Sync>);

impl fmt::Debug for InputFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputFactory").finish_non_exhaustive()
    }
}

impl Default for ReplSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplSettings {
    /// Sessions reading [`frost_repl::default_input`], with nothing bound.
    pub fn new() -> Self {
        Self {
            input: InputFactory(Arc::new(frost_repl::default_input)),
            bindings: BTreeMap::new(),
        }
    }

    /// Set where sessions read their input: `make` is called for a new input
    /// at the start of each session.
    pub fn with_input(
        mut self,
        make: impl Fn() -> Box<dyn ReplInput> + Send + Sync + 'static,
    ) -> Self {
        self.input = InputFactory(Arc::new(make));
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

    /// A new session's input, and `repl` with these settings' bindings.
    pub(crate) fn start(&self, repl: Repl) -> (Box<dyn ReplInput>, Repl) {
        let repl = repl
            .with_bindings(self.bindings.clone())
            .expect("each name was checked when it was bound");
        ((self.input.0)(), repl)
    }
}
