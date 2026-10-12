//! The outcomes of a run: [`ProgramResult`] (success) and [`RunError`] (failure),
//! plus [`IdleVm`], the warm VM either can be recycled into.

use std::sync::Arc;

use crate::{FrostError, Value};

use super::Vm;
use super::function::Closure;

/// A successful run.
/// Provides the script's tail value and exports, and holds the warm [`Vm`] for reuse.
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
            .zip(self.0.frame_slots(base))
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
            .zip(self.0.frame_slots(base))
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

impl ProgramResult {
    /// The number of function calls the program made: the fuel it consumed.
    /// Reported whether or not a [`fuel`](super::VmRuntimeConfiguration::fuel) limit was set.
    pub fn fuel_consumed(&self) -> usize {
        self.0.fuel_used
    }
}

/// A failed run. Holds the raised [`FrostError`] and the warm [`Vm`], which (unlike a
/// [`ProgramResult`]) exposes no program state (`tail`/`exports`), since a failed run
/// leaves the Vm indeterminate. Recover the error with [`into_error`](Self::into_error),
/// or recycle the Vm via [`into_idle_vm`](Self::into_idle_vm).
pub struct RunError {
    pub(super) vm: Vm,
    pub(super) error: FrostError,
}

impl RunError {
    /// The error that ended the run.
    pub fn error(&self) -> &FrostError {
        &self.error
    }

    /// Take the error, discarding the Vm.
    pub fn into_error(self) -> FrostError {
        self.error
    }

    /// Whether the program raised the error or the run was aborted.
    pub fn kind(&self) -> RunErrorKind {
        match self.vm.abort_reason() {
            Some(reason) => RunErrorKind::Aborted(reason),
            None => RunErrorKind::Raised,
        }
    }

    /// The number of function calls the program made before it failed.
    pub fn fuel_consumed(&self) -> usize {
        self.vm.fuel_used
    }
}

/// How a run failed; see [`RunError::kind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunErrorKind {
    /// The program raised an error that nothing caught. Frost code could have
    /// caught it with `try_call`.
    Raised,
    /// The run was stopped, for the given reason. No Frost code can catch an
    /// abort.
    Aborted(AbortReason),
}

/// Why a run was aborted; see [`RunErrorKind::Aborted`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AbortReason {
    /// The run made more function calls than its
    /// [`fuel`](super::VmRuntimeConfiguration::fuel) allows.
    FuelExhausted,
    /// The run's [`CancelToken`](super::CancelToken) was cancelled.
    Cancelled,
}

/// A warm [`Vm`] with no program loaded: its allocations are kept for reuse,
/// ready to be rebound to a closure. Recovered from a finished run via
/// [`ProgramResult::into_idle_vm`] / [`RunError::into_idle_vm`].
///
/// Recycling a warm Vm is cheaper than [building](super::VmFactory::build) a fresh one.
/// The recycled Vm keeps its configuration and importer.
pub struct IdleVm(Vm);

impl IdleVm {
    /// Bind this warm VM to `closure`, clearing the previous run's stack and
    /// frames and resetting fuel, yielding a runnable [`Vm`].
    ///
    /// Detaches the previous run's [`CancelToken`](super::CancelToken) and
    /// [`ModuleId`](super::ModuleId); attach them again with
    /// [`Vm::with_cancel_token`] and [`Vm::with_module_id`] as needed.
    pub fn build(self, closure: Arc<Closure>) -> Vm {
        self.0.rearm(closure)
    }
}

impl ProgramResult {
    /// Recover the warm VM as an [`IdleVm`] for reuse, discarding this result.
    pub fn into_idle_vm(self) -> IdleVm {
        IdleVm(self.0)
    }

    /// Take the result apart: its tail value, its exports, and the warm VM.
    ///
    /// Unlike cloning out through [`tail`](Self::tail) and [`exports`](Self::exports),
    /// this leaves the [`IdleVm`] no copy of any value, so none becomes shared.
    pub fn into_parts(self) -> ProgramParts {
        let mut vm = self.0;
        let tail = vm.stack.pop().unwrap_or(Value::Null);
        let base = vm.base_frame();
        let (function, slot_base) = (Arc::clone(&base.this_fn), base.slot_base);
        // Draining every slot also drops the non-exported bindings,
        // which may share a value with the tail or an export.
        let exports = function
            .name_table
            .iter()
            .zip(vm.slots.drain(slot_base..))
            .filter(|(entry, _)| entry.exported)
            .map(|(entry, slot)| {
                let value = slot.expect("IMPOSSIBLE: exported slot unfilled after execution");
                (entry.name.clone(), value)
            })
            .collect();
        ProgramParts {
            tail,
            exports,
            idle_vm: IdleVm(vm),
        }
    }
}

/// A successful run taken apart by value; see [`ProgramResult::into_parts`].
pub struct ProgramParts {
    /// The value of the script's tail expression, as [`ProgramResult::tail`] reads it.
    pub tail: Value,
    /// The exported bindings and their values, as [`ProgramResult::exports`] lists them.
    pub exports: Vec<(String, Value)>,
    /// The warm VM, for reuse.
    pub idle_vm: IdleVm,
}

impl std::fmt::Debug for ProgramParts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Elide the (large, indeterminate) Vm from `{:?}` output.
        f.debug_struct("ProgramParts")
            .field("tail", &self.tail)
            .field("exports", &self.exports)
            .finish_non_exhaustive()
    }
}

impl RunError {
    /// Recover the warm VM as an [`IdleVm`] for reuse, discarding the error.
    pub fn into_idle_vm(self) -> IdleVm {
        IdleVm(self.vm)
    }

    /// Take the failure apart: the error that ended the run, and the warm VM.
    pub fn into_parts(self) -> (FrostError, IdleVm) {
        (self.error, IdleVm(self.vm))
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
