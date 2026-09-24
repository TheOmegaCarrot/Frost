//! Lexical scope resolution over a function's locals.
//!
//! `infos` is the authoritative, monotonic list of locals keyed by [`LocalId`].
//! `live` holds the ids currently in scope, and `marks` records where each
//! nested scope began, so a `do` block or `match` arm can be entered and unwound
//! without disturbing the locals it allocated.
//!
//! This does bookkeeping only: no IR, no diagnostics. It returns ids and a
//! bookkeeping-level error (the original binding's span); the lowering code turns
//! those into opcodes and `CompilerError`s. Concrete frame slots are assigned
//! only at assembly, by [`Locals::plan_slots`].

#[cfg(test)]
mod tests;

use frost_parse::ast::SourceSpan;
use frost_runtime::{NameEntry, Value};

use crate::lower::{Ir, LocalId};

/// What role a local plays in its frame. Captures are seated by the VM into the
/// leading slots; params and bindings are defined by the code itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LocalKind {
    Capture,
    Param,
    Binding,
}

/// A local's metadata: its runtime name-table fields, its role, the span that
/// introduced it (kept so a redefinition can point back at the original), and
/// its value if the binding is compile-time known.
#[derive(Debug)]
pub(super) struct LocalInfo {
    pub(super) name: String,
    pub(super) span: SourceSpan,
    pub(super) exported: bool,
    pub(super) constant: Option<Value>,
    pub(super) kind: LocalKind,
}

/// A function's locals, resolved across nested lexical scopes.
#[derive(Debug, Default)]
pub(super) struct Locals {
    infos: Vec<LocalInfo>,
    live: Vec<LocalId>,
    marks: Vec<usize>,
}

impl Locals {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Introduce a local in the current scope, returning its [`LocalId`].
    ///
    /// Errors with the original binding's span when the name is already bound in
    /// this same scope; shadowing a binding from an outer scope is allowed and
    /// allocates a fresh id.
    pub(super) fn define(&mut self, info: LocalInfo) -> Result<LocalId, SourceSpan> {
        let floor = self.marks.last().copied().unwrap_or(0);
        // A binding may shadow a same-named capture: the capture is the enclosing
        // scope's value (used by an rhs before the binding exists), the binding a
        // fresh local. Only another binding in this scope is a real duplicate.
        if let Some(id) = self.live[floor..].iter().find(|id| {
            let existing = self.info(**id);
            existing.name == info.name && existing.kind != LocalKind::Capture
        }) {
            return Err(self.info(*id).span);
        }
        let id = LocalId(self.infos.len());
        self.infos.push(info);
        self.live.push(id);
        Ok(id)
    }

    /// Resolve `name` to its id, innermost binding first (shadowing).
    pub(super) fn resolve(&self, name: &str) -> Option<LocalId> {
        self.live
            .iter()
            .rev()
            .copied()
            .find(|id| self.info(*id).name == name)
    }

    /// The compile-time value of a local, if the binding is a known constant.
    pub(super) fn constant(&self, id: LocalId) -> Option<&Value> {
        self.info(id).constant.as_ref()
    }

    /// Whether the current scope is the function's outermost (no nested scope
    /// open), i.e. a binding defined now is a top-level one.
    pub(super) fn at_top_scope(&self) -> bool {
        self.marks.is_empty()
    }

    /// Open a nested scope (a `do` block or a `match` arm).
    pub(super) fn enter(&mut self) {
        self.marks.push(self.live.len());
    }

    /// Close the innermost scope, dropping its bindings from view. The ids they
    /// allocated stay in `infos`.
    pub(super) fn exit(&mut self) {
        let mark = self.marks.pop().expect("exit without a matching enter");
        self.live.truncate(mark);
    }

    /// Assign each local a concrete frame slot and build the parallel name table,
    /// both from one traversal of `code` so they cannot disagree.
    ///
    /// Captures lead the slots (`0..num_captures`), in definition order, since
    /// the VM seats the closure's captured values there. Every other local the
    /// code defines follows, in first-definition order; a local the optimized
    /// code no longer defines is simply left out, so removing one leaves no hole.
    /// A `LoadLocal` of a local that is neither a capture nor defined here has no
    /// slot, and [`SlotPlan::slot_of`] will catch it.
    pub(super) fn plan_slots(&self, code: &[Ir]) -> SlotPlan {
        let mut plan = SlotPlan::sized(self.infos.len());
        for (index, info) in self.infos.iter().enumerate() {
            if info.kind == LocalKind::Capture {
                plan.assign(LocalId(index), info);
            }
        }
        plan.num_captures = plan.name_table.len();
        self.assign_defined(&mut plan, code);
        plan
    }

    /// [`plan_slots`](Self::plan_slots) for a fragment evaluated on its own, as
    /// in a constant fold: no captures, since there is no closure to seat them,
    /// so only the locals the fragment itself defines get a slot. A fragment that
    /// reads a local it does not define has no business being evaluated alone,
    /// and [`SlotPlan::slot_of`] will catch it.
    pub(super) fn plan_fragment_slots(&self, code: &[Ir]) -> SlotPlan {
        let mut plan = SlotPlan::sized(self.infos.len());
        self.assign_defined(&mut plan, code);
        plan
    }

    /// Give each local `code` defines, and that has no slot yet, the next slot,
    /// in first-definition order.
    fn assign_defined(&self, plan: &mut SlotPlan, code: &[Ir]) {
        for ir in code {
            if let Ir::DefLocal(id) = ir
                && plan.slots[id.0].is_none()
            {
                plan.assign(*id, self.info(*id));
            }
        }
    }

    fn info(&self, id: LocalId) -> &LocalInfo {
        &self.infos[id.0]
    }
}

/// A function's frame layout: the slot each local occupies and the parallel name
/// table, derived together by [`Locals::plan_slots`].
pub(super) struct SlotPlan {
    // Slot per local, indexed by `LocalId`; `None` for a local with no slot (a
    // capture is always present, a body local only if the code defines it).
    slots: Vec<Option<usize>>,
    name_table: Vec<NameEntry>,
    num_captures: usize,
}

impl SlotPlan {
    /// The layout for code with no locals.
    pub(super) fn empty() -> Self {
        Self::sized(0)
    }

    /// A plan with no slots assigned yet, for `num_locals` locals.
    fn sized(num_locals: usize) -> Self {
        Self {
            slots: vec![None; num_locals],
            name_table: Vec::new(),
            num_captures: 0,
        }
    }

    /// The frame slot of a local the code references.
    pub(super) fn slot_of(&self, id: LocalId) -> usize {
        self.slots[id.0].expect("a referenced local was assigned no slot")
    }

    pub(super) fn num_captures(&self) -> usize {
        self.num_captures
    }

    pub(super) fn into_name_table(self) -> Vec<NameEntry> {
        self.name_table
    }

    /// Give `id` the next slot and append its name-table entry.
    fn assign(&mut self, id: LocalId, info: &LocalInfo) {
        self.slots[id.0] = Some(self.name_table.len());
        self.name_table.push(NameEntry {
            name: info.name.clone(),
            exported: info.exported,
        });
    }
}
