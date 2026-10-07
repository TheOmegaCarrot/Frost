//! Lexical scope resolution over a function's locals.
//!
//! `infos` is the authoritative, monotonic list of locals keyed by [`LocalId`].
//! `live` holds the ids currently in scope, and `marks` records where each
//! nested scope began, so a `do` block or `match` arm can be entered and unwound
//! without disturbing the locals it allocated.
//!
//! This does bookkeeping only: no IR, no diagnostics. It returns ids and a
//! bookkeeping-level error (the original binding's span); the lowering code turns
//! those into opcodes and `Diagnostic`s. Concrete frame slots are assigned
//! only at assembly, by [`Locals::plan_slots`].

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};

use frostlang_parse::ast::SourceSpan;
use frostlang_runtime::{NameEntry, Value};

use crate::lower::{Ir, LocalId};

/// What role a local plays in its frame. Captures are seated by the VM into the
/// leading slots; params and bindings are defined by the code itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LocalKind {
    Capture,
    /// A capture whose value is compile-time known, built into the function
    /// instead of seated: every lookup of it loads the constant, so it never
    /// occupies a slot.
    Hoisted,
    Param,
    Binding,
}

impl LocalKind {
    /// Whether the local comes from the enclosing scope, which a binding in this
    /// function may shadow.
    fn is_inherited(self) -> bool {
        matches!(self, LocalKind::Capture | LocalKind::Hoisted)
    }
}

/// A local's metadata: its runtime name-table fields, its role, the span that
/// introduced it (kept so a redefinition can point back at the original), and
/// its value if the binding is compile-time known.
#[derive(Debug)]
pub(super) struct LocalInfo {
    pub(super) name: String,
    pub(super) span: SourceSpan,
    pub(super) exported: bool,
    // Any known value, including one the constant pool cannot hold, such as a
    // function: `compile_name_lookup` decides how a lookup uses it.
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

    /// The locals of a function whose `captures` are live from the start and
    /// seated in this order (see [`plan_slots`](Self::plan_slots)), each with its
    /// value if compile-time known, and whose `hoisted` captures are built in as
    /// constants.
    pub(super) fn with_captures(
        captures: impl IntoIterator<Item = (String, Option<Value>)>,
        hoisted: impl IntoIterator<Item = (String, Value)>,
    ) -> Self {
        let captured = captures
            .into_iter()
            .map(|(name, constant)| (name, constant, LocalKind::Capture));
        let hoisted = hoisted
            .into_iter()
            .map(|(name, value)| (name, Some(value), LocalKind::Hoisted));
        let mut locals = Self::new();
        for (name, constant, kind) in captured.chain(hoisted) {
            locals
                .define(LocalInfo {
                    name,
                    span: SourceSpan::default(),
                    exported: false,
                    constant,
                    kind,
                })
                .expect("an inherited local never collides: only a binding can");
        }
        locals
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
            existing.name == info.name && !existing.kind.is_inherited()
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

    /// The compile-time value of a local, if it is known.
    pub(super) fn constant(&self, id: LocalId) -> Option<&Value> {
        self.info(id).constant.as_ref()
    }

    /// Whether a local is exported: its slot is read once the function finishes.
    pub(super) fn is_exported(&self, id: LocalId) -> bool {
        self.info(id).exported
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

    /// The bindings in view now, to return to with [`rewind`](Self::rewind).
    pub(super) fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.live.len())
    }

    /// Drop from view every binding made since `checkpoint`, which must be in the
    /// current scope. The ids they allocated stay in `infos`.
    pub(super) fn rewind(&mut self, checkpoint: Checkpoint) {
        let floor = self.marks.last().copied().unwrap_or(0);
        assert!(
            (floor..=self.live.len()).contains(&checkpoint.0),
            "a checkpoint is rewound to within its own scope"
        );
        self.live.truncate(checkpoint.0);
    }

    /// Bring an existing local back into view in the current scope, as if it were
    /// defined again.
    pub(super) fn revive(&mut self, id: LocalId) {
        self.live.push(id);
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
    /// in a constant fold, along with the value to seat in each of its captures,
    /// by name.
    ///
    /// The fragment's captures are the locals it reads but does not define, in
    /// first-read order. A fragment is evaluated alone only if each such local's
    /// value is compile-time known, and that value is what is seated. The locals
    /// the fragment defines follow.
    pub(super) fn plan_fragment_slots(&self, code: &[Ir]) -> (SlotPlan, BTreeMap<String, Value>) {
        let defined: HashSet<LocalId> = code
            .iter()
            .filter_map(|ir| match ir {
                Ir::DefLocal(id) => Some(*id),
                _ => None,
            })
            .collect();
        let mut plan = SlotPlan::sized(self.infos.len());
        let mut captures = BTreeMap::new();
        for ir in code {
            if let Ir::LoadLocal(id) = ir
                && !defined.contains(id)
                && plan.slots[id.0].is_none()
            {
                let info = self.info(*id);
                let value = info
                    .constant
                    .clone()
                    .expect("a fragment evaluated alone reads only known outer locals");
                // The outer locals a fragment reads are among those in view where
                // it starts, which have distinct names.
                let clash = captures.insert(info.name.clone(), value);
                assert!(
                    clash.is_none(),
                    "two outer locals a fragment reads share a name"
                );
                plan.assign(*id, info);
            }
        }
        plan.num_captures = plan.name_table.len();
        self.assign_defined(&mut plan, code);
        (plan, captures)
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

/// The bindings in view at some point, from [`Locals::checkpoint`].
#[derive(Clone, Copy, Debug)]
pub(super) struct Checkpoint(usize);

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
