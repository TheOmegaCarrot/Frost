use std::sync::{Arc, LazyLock};

use crate::Value;

use super::GlobalSet;

// Sync-only macro: names and slot initializers expand from the same `name => init`
// list in the same order, so `names[i]` and `slots[i]` cannot drift apart.
macro_rules! define_globals {
    ($($name:literal => $init:expr),* $(,)?) => {
        impl GlobalSet {
            /// Names of all predefined globals.
            /// Every GlobalSet shares the same names, so it is not a
            /// field. The single source of truth for global ordering.
            const NAMES: &'static [&'static str] = &[ $($name),* ];

            fn build_defaults() -> Self {
                GlobalSet ( vec![ $($init),* ] )
            }
        }
    };
}

define_globals! {
     // TODO: actually implement these
    "print"     => Value::Null,
    "transform" => Value::Null,
}

static DEFAULT_GLOBALS: LazyLock<Arc<GlobalSet>> =
    LazyLock::new(|| Arc::new(GlobalSet::build_defaults()));

impl GlobalSet {
    pub fn defaults() -> Arc<Self> {
        DEFAULT_GLOBALS.clone()
    }

    /// Override an existing global by name. Returns the set on
    /// success, or `None` if `name` is not a known global, the set is closed,
    /// so new globals cannot be added.
    pub fn with_override(mut self: Arc<Self>, name: &str, value: Value) -> Option<Arc<GlobalSet>> {
        let idx = self.resolve(name)?;
        Arc::make_mut(&mut self).0[idx] = value;
        Some(self)
    }

    /// Look up a global's slot index by name.
    fn resolve(&self, name: &str) -> Option<usize> {
        // Yes, this is a linear scan, but this should be a pretty cold path.
        Self::NAMES.iter().position(|&n| n == name)
    }

    /// Get the value at a global slot index. Used by the VM during execution.
    pub fn get(&self, idx: usize) -> &Value {
        &self.0[idx]
    }
}
