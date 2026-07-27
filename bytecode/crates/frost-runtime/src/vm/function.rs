//! Compiled functions and the trust/closing path that makes them runnable:
//! [`CompiledFunction`] -> [`TrustedProgram`] -> [`Closure`].

use std::{collections::BTreeMap, sync::Arc};

use crate::{MapKey, Value};

use super::bytecode::Bytecode;
use super::serialize;

/// Compiled representation of a single function.
/// A script's top-level is also a function.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompiledFunction {
    // Version marker: stamps the runtime version on serialize, rejects a mismatch on
    // deserialize. First, so a mismatched image fails before the rest is decoded.
    pub version: serialize::FormatVersion,
    // Functions always have a name
    pub name: String,
    pub code: Vec<Bytecode>,
    // Functions which are defined in this function's body
    pub child_fns: Vec<Arc<CompiledFunction>>,
    // Constant values that can't be inlined in an opcode.
    // Mostly strings, but can include any structured value the compiler can constant-fold.
    // Serialized through `ConstValue` (see `serialize`); a function-valued constant is rejected.
    #[serde(with = "serialize::const_pool")]
    pub constants: Vec<Value>,
    // Constant map keys, in their own pool so a keyed instruction can borrow one
    // rather than build a `MapKey` per execution. `MapKey` cannot hold a function,
    // so unlike `constants` this needs no serialization guard.
    pub key_constants: Vec<MapKey>,
    // Table so that locals can be looked up by name at runtime,
    // or their slot given a name by an error.
    pub name_table: Vec<NameEntry>,
    // Number of leading `name_table`/slot entries that are captures (slots `0..num_captures`);
    // the remainder are locals, including params. May be 0.
    pub num_captures: usize,
    pub arity: Arity,
}

impl CompiledFunction {
    /// The capture names this function expects.
    /// Tells a host which names to include in the map passed to [`close`](TrustedProgram::close).
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.name_table[..self.num_captures]
            .iter()
            .map(|entry| entry.name.as_str())
    }

    /// Vouch that this compiled-function tree is well-formed, yielding a runnable [`TrustedProgram`].
    ///
    /// The VM only runs trusted bytecode: [`Closure`] (hence [`Vm`](super::Vm)) construction goes
    /// through [`TrustedProgram`], so this is the gate.
    /// The Frost compiler's output is always well-formed;
    /// call this on deserialized or cached bytecode only when you control its source.
    ///
    /// This is an assertion, not a check: running malformed bytecode *panics*.
    /// It is never memory-unsafe, so this is a safe function, but the obligation is real.
    /// A future verifier will offer a *checked* path; prefer it for bytecode you did not author.
    pub fn assert_trusted(self: Arc<Self>) -> TrustedProgram {
        TrustedProgram(self)
    }
}

/// A [`CompiledFunction`] tree asserted (or, in future, verified) well-formed, and therefore
/// runnable. Mint one via [`CompiledFunction::assert_trusted`]; bind its captures with
/// [`close`](Self::close) to obtain a [`Closure`].
///
/// "Trusted" means the bytecode upholds the Vm's internal invariants
/// (never popping an empty stack, never jumping out-of-bounds),
/// *not* that the program is benign, correct, or even terminates.
/// A malicious or runaway script is still "trusted" in this sense.
/// What such a script may reach is governed by its [`Importer`](super::Importer),
/// and how much it may run by [`VmRuntimeConfiguration`](super::VmRuntimeConfiguration).
pub struct TrustedProgram(Arc<CompiledFunction>);

impl TrustedProgram {
    /// Bind this function's captures into a runnable [`Closure`].
    ///
    /// Required captures are looked up by name in `captures`.
    /// Extra entries in the map are ignored.
    /// Any required capture name absent from the map is reported, together, as [`MissingCaptures`].
    pub fn close(self, captures: BTreeMap<String, Value>) -> Result<Arc<Closure>, MissingCaptures> {
        let function = self.0;
        let mut seated = Vec::with_capacity(function.num_captures);
        let mut missing = Vec::new();
        for entry in &function.name_table[..function.num_captures] {
            match entry.name.as_str() {
                // Frost-internal capture: runtime-supplied, not overridable.
                // (Always false for now: direct execution; the future `import`
                // path will need to supply `true`.)
                "imported" => seated.push(Value::Bool(false)),
                name => match captures.get(name) {
                    Some(value) => seated.push(value.clone()),
                    // Keep scanning so every missing name is reported at once.
                    None => missing.push(name.to_owned()),
                },
            }
        }
        if !missing.is_empty() {
            return Err(MissingCaptures { names: missing });
        }
        Ok(Arc::new(Closure {
            function,
            captures: seated,
        }))
    }

    /// Convenience for [`close`](Self::close) with no host-supplied captures.
    /// Succeeds when the function needs no host captures;
    /// otherwise returns the [`MissingCaptures`] it still requires.
    pub fn into_closure(self) -> Result<Arc<Closure>, MissingCaptures> {
        self.close(BTreeMap::new())
    }
}

/// One or more required captures were absent from the map passed to [`TrustedProgram::close`];
/// reports every missing name, not just the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingCaptures {
    pub names: Vec<String>,
}

impl std::fmt::Display for MissingCaptures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing capture(s): {}", self.names.join(", "))
    }
}

impl std::error::Error for MissingCaptures {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NameEntry {
    pub name: String,
    pub exported: bool,
}

/// Representation of the arity of a Frost function.
/// Every function has a certain number of fixed args, and may or may not be variadic.
/// A native function may have a specific arity range, without being variadic.
/// ```frost
/// fn a, b, c, ...more -> ...
/// # at least 3
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Arity {
    Exact(usize),
    Between(usize, usize),
    AtLeast(usize),
}

#[derive(Debug)]
pub struct Closure {
    pub(super) function: Arc<CompiledFunction>,

    // Captured values, seated into slots 0..num_captures. Empty when nothing is captured.
    pub(super) captures: Vec<Value>,
}

impl Closure {
    pub fn inner_fn(&self) -> &CompiledFunction {
        &self.function
    }

    pub fn inner_fn_arc(&self) -> Arc<CompiledFunction> {
        self.function.clone()
    }
}
