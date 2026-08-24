//! The outcomes of a run: [`ProgramResult`] (success), [`RunError`] (failure),
//! and the [`RunOutcome`] surface they share.

use std::sync::Arc;

use crate::{FrostError, Value};

use super::Vm;
use super::function::Closure;

/// The result of executing a CompiledFunction.
/// Provides access to the top-level defined values and exports of a script.
pub struct ProgramResult(pub(super) Vm);

impl std::fmt::Debug for ProgramResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Elide the (large, indeterminate) Vm from `{:?}`/`unwrap` output.
        f.debug_struct("ProgramResult")
            .field("tail", self.tail())
            .finish_non_exhaustive()
    }
}

impl ProgramResult {
    /// Get the value of the tail expression of a script.
    /// Often `null`.
    pub fn tail(&self) -> &Value {
        self.0.stack.last().unwrap_or(&Value::Null)
    }

    /// Look up the value of an exported binding.
    /// None indicates the name was not exported.
    pub fn get_export<'a>(&'a self, name: &str) -> Option<&'a Value> {
        let base = self.0.base_frame();
        // There cannot be duplicate names that are both exported.
        // Exports can only be defined at the top-level in an `export def`,
        // and the compiler must reject any duplicate bindings.
        base.this_fn
            .name_table
            .iter()
            .zip(&base.local_slots)
            .find(|(entry, _)| entry.exported && entry.name == name)
            .map(|(_, slot)| {
                slot.as_ref()
                    .expect("IMPOSSIBLE: exported slot unfilled after execution")
            })
    }

    /// Get all values exported by the script.
    pub fn exports(&self) -> impl Iterator<Item = (&str, &Value)> {
        let base = self.0.base_frame();
        base.this_fn
            .name_table
            .iter()
            .zip(&base.local_slots)
            .filter(|(entry, _)| entry.exported)
            .map(|(entry, slot)| {
                (
                    entry.name.as_str(),
                    slot.as_ref()
                        .expect("IMPOSSIBLE: exported slot unfilled after execution"),
                )
            })
    }
}

/// The surface common to both outcomes of a run (success or failure): each still owns the
/// warm [`Vm`], so either can be metered or recycled.
pub trait RunOutcome {
    /// The number of function calls the program made: the fuel it consumed.
    /// Reported whether or not a [`fuel`](super::VmRuntimeConfiguration::fuel) limit was set.
    fn fuel_consumed(&self) -> usize;

    /// Recycle the warm [`Vm`] to run another [`Closure`], reusing its internal allocations
    /// rather than building a fresh one.
    fn reset(self, closure: Arc<Closure>) -> Vm;
}

impl RunOutcome for ProgramResult {
    fn fuel_consumed(&self) -> usize {
        self.0.fuel_used
    }

    fn reset(self, closure: Arc<Closure>) -> Vm {
        self.0.rearm(closure)
    }
}

/// A failed run. Holds the raised [`FrostError`] and the warm [`Vm`], which (unlike a
/// [`ProgramResult`]) exposes no program state (`tail`/`exports`), since a failed run
/// leaves the Vm indeterminate. Recover the error with [`into_error`](Self::into_error),
/// or recycle the Vm via [`RunOutcome::reset`].
pub struct RunError {
    pub(super) vm: Vm,
    pub(super) error: FrostError,
}

impl RunError {
    /// The error that ended the run.
    pub fn error(&self) -> &FrostError {
        &self.error
    }

    /// Take the error, discarding the Vm. The cheap exit for a host that will not reuse it.
    pub fn into_error(self) -> FrostError {
        self.error
    }
}

impl RunOutcome for RunError {
    fn fuel_consumed(&self) -> usize {
        self.vm.fuel_used
    }

    fn reset(self, closure: Arc<Closure>) -> Vm {
        self.vm.rearm(closure)
    }
}

impl std::fmt::Debug for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Elide the (large, indeterminate) Vm from `{:?}`/`unwrap` output.
        f.debug_struct("RunError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
