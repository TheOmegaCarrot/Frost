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
    /// Stamps the runtime version into a serialized image; see [`FormatVersion`](crate::FormatVersion).
    // First, so a mismatched image fails before the rest is decoded.
    pub version: serialize::FormatVersion,
    /// The function's name, as reported in errors and backtraces.
    pub name: String,
    /// The base name of the file the function was compiled from, such as `util.frst`,
    /// as reported in backtraces. `None` when there is no such file.
    // An `Arc` because every function from one compilation shares it.
    pub origin: Option<Arc<str>>,
    /// The function body.
    pub code: Vec<Bytecode>,
    /// Functions defined in this function's body, indexed by [`Bytecode::CreateClosure`].
    pub child_fns: Vec<Arc<CompiledFunction>>,
    /// Constant values that cannot be inlined in an instruction, indexed by [`Bytecode::LoadConst`].
    /// Serialization fails if one is a function or an Opaque value, or contains one.
    // Mostly Strings, but can include any structured value the compiler can constant-fold.
    // Serialized through `ConstValue` (see `serialize`).
    #[serde(with = "serialize::const_pool")]
    pub constants: Vec<Value>,
    /// Constant Map keys, indexed by [`Bytecode::HardIndexMap`].
    // In their own pool so a keyed instruction can borrow one
    // rather than build a `MapKey` per execution. `MapKey` cannot hold a function,
    // so unlike `constants` this needs no serialization guard.
    pub key_constants: Vec<MapKey>,
    /// The name of each slot, in slot order,
    /// so a binding can be looked up by name or a slot named in an error.
    pub name_table: Vec<NameEntry>,
    /// How many leading slots (`0..num_captures`) are captures;
    /// the remaining slots are locals, including parameters.
    pub num_captures: usize,
    /// The number of arguments the function accepts.
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
#[derive(Debug)]
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
            match captures.get(entry.name.as_str()) {
                Some(value) => seated.push(value.clone()),
                // Keep scanning so every missing name is reported at once.
                None => missing.push(entry.name.clone()),
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
    /// The missing capture names, in capture order.
    pub names: Vec<String>,
}

impl std::fmt::Display for MissingCaptures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing capture(s): {}", self.names.join(", "))
    }
}

impl std::error::Error for MissingCaptures {}

/// One slot's entry in a [`CompiledFunction`]'s [`name_table`](CompiledFunction::name_table).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NameEntry {
    /// The binding's name.
    pub name: String,
    /// Whether the binding is exported (see [`ProgramResult::exports`](crate::ProgramResult::exports)).
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
    /// Exactly this many arguments.
    Exact(usize),
    /// From the first bound to the second, inclusive.
    Between(usize, usize),
    /// At least this many arguments.
    AtLeast(usize),
}

/// A runnable function: a [`CompiledFunction`] with its captures bound.
/// Obtain one from [`TrustedProgram::close`] and run it on a Vm built by [`VmFactory::build`](crate::VmFactory::build).
#[derive(Debug)]
pub struct Closure {
    pub(super) function: Arc<CompiledFunction>,

    // Captured values, seated into slots 0..num_captures. Empty when nothing is captured.
    pub(super) captures: Vec<Value>,
}

impl Closure {
    /// The [`CompiledFunction`] this closure runs.
    pub fn inner_fn(&self) -> &CompiledFunction {
        &self.function
    }

    /// Like [`inner_fn`](Self::inner_fn), but returns a shared handle.
    pub fn inner_fn_arc(&self) -> Arc<CompiledFunction> {
        self.function.clone()
    }
}
