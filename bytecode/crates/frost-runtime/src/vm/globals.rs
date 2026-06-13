use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use crate::Value;

use super::GlobalSet;

macro_rules! define_globals {
    ($($variant:ident = $name:literal => $init:expr),* $(,)?) => {
        #[derive(Clone, Copy, Debug)]
        pub enum GlobalName {
            $($variant),*
        }

        impl GlobalName {
            pub fn as_str(self) -> &'static str {
                match self { $( Self::$variant => $name, )* }
            }
        }

        impl GlobalSet {
            const GLOBAL_COUNT: usize = [$(stringify!($variant)),*].len();

            fn build_defaults() -> Self {
                let names: BTreeMap<String, usize> = [
                    $( ($name.to_string(), GlobalName::$variant as usize), )*
                ].into_iter().collect();

                let mut slots = vec![Value::Null; Self::GLOBAL_COUNT];
                $( slots[GlobalName::$variant as usize] = $init; )*

                GlobalSet { names: Arc::new(names), slots }
            }
        }
    }
}

define_globals! {
    Print     = "print"     => Value::Null, // TODO: native print
    Transform = "transform" => Value::Null, // TODO: native transform
}

static DEFAULT_GLOBALS: LazyLock<Arc<GlobalSet>> =
    LazyLock::new(|| Arc::new(GlobalSet::build_defaults()));

impl GlobalSet {
    pub fn defaults() -> Arc<Self> {
        DEFAULT_GLOBALS.clone()
    }

    /// Override an existing global by name. The name must be a known global
    /// (a variant of `GlobalName`); new globals cannot be added.
    pub fn with_override(mut self: Arc<Self>, name: GlobalName, value: Value) -> Arc<GlobalSet> {
        Arc::make_mut(&mut self).slots[name as usize] = value;
        self
    }

    /// Look up a global's slot index by name. Used by the compiler to resolve
    /// global references to LoadGlobal indices.
    pub fn resolve(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    /// Get the value at a global slot index. Used by the VM during execution.
    pub fn get(&self, idx: usize) -> &Value {
        &self.slots[idx]
    }
}
