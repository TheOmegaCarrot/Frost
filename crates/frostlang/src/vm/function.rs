//! Compiled functions and the assertion and closing that make them runnable:
//! [`CompiledFunction`] (or [`CompiledProgram`]) -> [`WellFormedProgram`] -> [`Closure`].

use std::sync::Arc;

use crate::core::free;
use crate::{MapKey, Value};

use super::bytecode::Bytecode;
use super::serialize;
use crate::vm::debug_info::DebugInfo;

/// Compiled representation of a single function.
/// A script's top-level is also a function.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompiledFunction {
    /// Stamps the runtime version into a serialized image; see [`FormatVersion`](crate::bytecode::FormatVersion).
    // First, so a mismatched image fails before the rest is decoded.
    pub version: serialize::FormatVersion,
    /// The function's name, as reported in errors and backtraces.
    pub name: String,
    /// The base name of the file the function was compiled from,
    /// as [`BacktraceFrame::origin`](crate::BacktraceFrame::origin) reports it.
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
    /// Debug info for the function's code.
    /// `None` when it was compiled without debug info or has been [stripped](Self::strip).
    pub debug_info: Option<DebugInfo>,
}

impl CompiledFunction {
    /// This function without [`debug_info`](Self::debug_info), nor any in the functions it defines.
    ///
    /// A nested function shared beyond this one, such as by a [`Closure`] made from it,
    /// is copied rather than changed in place.
    #[must_use]
    pub fn strip(self) -> Self {
        let child_fns = self.child_fns.into_iter().map(strip_shared).collect();
        Self {
            child_fns,
            debug_info: None,
            ..self
        }
    }

    /// The capture names this function expects, in slot order.
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.name_table[..self.num_captures]
            .iter()
            .map(|entry| entry.name.as_str())
    }

    /// Asserts that this compiled-function tree is well-formed, yielding a runnable
    /// [`WellFormedProgram`], under the obligation of
    /// [`CompiledProgram::assert_well_formed`].
    pub fn assert_well_formed(self: Arc<Self>) -> WellFormedProgram {
        WellFormedProgram(self)
    }
}

/// A compiled program not yet asserted well-formed, such as one loaded from a program image.
/// [`assert_well_formed`](Self::assert_well_formed) makes it runnable.
#[derive(Debug, Clone)]
pub struct CompiledProgram(Arc<CompiledFunction>);

impl CompiledProgram {
    /// Asserts that this program is well-formed, yielding a runnable [`WellFormedProgram`].
    ///
    /// Every [`Closure`], and so every [`Vm`](super::Vm), is built from a
    /// [`WellFormedProgram`], so this is the gate to running bytecode.
    /// The Frost compiler's output is always well-formed;
    /// call this on deserialized or cached bytecode only when you control its source.
    ///
    /// This is an assertion, not a check: running malformed bytecode *panics*.
    /// It is never memory-unsafe, so this is a safe function, but the obligation is real.
    /// A future verifier will offer a *checked* path; prefer it for bytecode you did not author.
    pub fn assert_well_formed(self) -> WellFormedProgram {
        self.0.assert_well_formed()
    }

    /// This program without debug info.
    ///
    /// Debug info records where in source each instruction came from,
    /// for runtime error backtraces and disassembly.
    /// The program runs the same without it, and its `image` is smaller.
    ///
    /// Copies any part of the program shared beyond this one, such as by a clone of it.
    #[must_use]
    pub fn strip(self) -> Self {
        Self(strip_shared(self.0))
    }
}

/// `function` [stripped](CompiledFunction::strip), copied first if it is shared.
fn strip_shared(function: Arc<CompiledFunction>) -> Arc<CompiledFunction> {
    Arc::new(Arc::unwrap_or_clone(function).strip())
}

/// The [`CompiledProgram`] whose root is `function`.
pub fn program_from_function(function: Arc<CompiledFunction>) -> CompiledProgram {
    CompiledProgram(function)
}

/// The [`CompiledFunction`] at the root of `program`.
pub fn program_function(program: &CompiledProgram) -> &Arc<CompiledFunction> {
    &program.0
}

/// The [`CompiledFunction`] that `closure` runs.
pub fn closure_function(closure: &Closure) -> &Arc<CompiledFunction> {
    &closure.function
}

/// A compiled program asserted (or, in future, verified) well-formed, and therefore
/// runnable. Mint one via [`CompiledProgram::assert_well_formed`]; bind its captures with
/// [`close`](Self::close) to obtain a [`Closure`].
///
/// Well-formed means only that the bytecode upholds the Vm's internal invariants
/// (never popping an empty stack, never jumping out of bounds),
/// so running it reaches none of the Vm's panics.
/// It says nothing of what the program does: every script that compiles is well-formed,
/// whatever its intent, including one that is malicious, never terminates, or exhausts memory.
/// What a script may reach is governed by its [`Importer`](super::Importer),
/// and how much it may run by [`VmRuntimeConfiguration`](super::VmRuntimeConfiguration).
#[derive(Debug, Clone)]
pub struct WellFormedProgram(pub(crate) Arc<CompiledFunction>);

impl WellFormedProgram {
    /// The names of the captures this program expects, in capture order: the names
    /// [`close`](Self::close) or [`close_with`](Self::close_with) must be given values for.
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.0.capture_names()
    }

    /// This program [stripped](CompiledProgram::strip) of debug info. It stays well-formed.
    /// A [`Closure`] made from it shares it, so stripping copies the shared part.
    #[must_use]
    pub fn strip(self) -> Self {
        Self(strip_shared(self.0))
    }

    /// Binds this program's captures from `captures`, yielding a runnable [`Closure`].
    ///
    /// Each pair names a capture and supplies its value.
    /// A pair naming no capture of this program is ignored,
    /// and a name given more than once takes its last value.
    /// Every capture that no pair names is reported, together, as [`MissingCaptures`].
    ///
    /// This visits every pair. To close against a large environment that the program
    /// mostly does not use, prefer [`close_with`](Self::close_with).
    pub fn close<K: AsRef<str>>(
        self,
        captures: impl IntoIterator<Item = (K, Value)>,
    ) -> Result<Arc<Closure>, MissingCaptures> {
        let names = &self.0.name_table[..self.0.num_captures];
        let mut slots: Vec<Option<Value>> = vec![None; names.len()];
        for (name, value) in captures {
            let name = name.as_ref();
            if let Some(slot) = names.iter().position(|entry| entry.name == name) {
                slots[slot] = Some(value);
            }
        }
        self.seat(slots)
    }

    /// Binds this program's captures by asking `lookup` for each, yielding a runnable
    /// [`Closure`].
    ///
    /// `lookup` is called once per capture, with its name, in the order of
    /// [`capture_names`](Self::capture_names).
    /// If it returns `None` for any capture, `close_with` fails: every capture it returned
    /// `None` for is reported, together, as [`MissingCaptures`], and the values it did
    /// return are dropped.
    pub fn close_with(
        self,
        lookup: impl FnMut(&str) -> Option<Value>,
    ) -> Result<Arc<Closure>, MissingCaptures> {
        let slots = self.capture_names().map(lookup).collect();
        self.seat(slots)
    }

    /// Convenience for [`close`](Self::close) with no host-supplied captures.
    /// Succeeds when the function needs no host captures;
    /// otherwise returns the [`MissingCaptures`] it still requires.
    pub fn into_closure(self) -> Result<Arc<Closure>, MissingCaptures> {
        self.close_with(|_| None)
    }

    /// The closure seating `slots`, one per capture in capture order,
    /// or every capture whose slot is empty.
    fn seat(self, slots: Vec<Option<Value>>) -> Result<Arc<Closure>, MissingCaptures> {
        let names: Vec<String> = self
            .capture_names()
            .zip(&slots)
            .filter(|(_, slot)| slot.is_none())
            .map(|(name, _)| name.to_owned())
            .collect();
        if !names.is_empty() {
            return Err(MissingCaptures { names });
        }
        Ok(Arc::new(Closure {
            function: self.0,
            // Every slot is filled, so this keeps them all.
            captures: slots.into_iter().flatten().collect(),
        }))
    }
}

/// One or more of a program's captures were given no value by [`WellFormedProgram::close`]
/// or [`WellFormedProgram::close_with`]; reports every missing name, not just the first.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("missing capture(s): {}", .names.join(", "))]
pub struct MissingCaptures {
    /// The missing capture names, in capture order.
    pub names: Vec<String>,
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Arity {
    /// Exactly this many arguments.
    Exact(usize),
    /// From the first bound to the second, inclusive.
    Between(usize, usize),
    /// At least this many arguments.
    AtLeast(usize),
}

/// A runnable function: compiled code with its captures bound.
/// Obtain one from [`WellFormedProgram::close`] and run it on a Vm built by [`VmFactory::build`](crate::VmFactory::build).
#[derive(Debug)]
pub struct Closure {
    pub(super) function: Arc<CompiledFunction>,

    // Captured values, seated into slots 0..num_captures. Empty when nothing is captured.
    pub(crate) captures: Vec<Value>,
}

impl Drop for Closure {
    fn drop(&mut self) {
        free::free(std::mem::take(&mut self.captures), Vec::into_iter);
    }
}
