//! Lexical scope resolution over a function's flat slot table.
//!
//! `table` is the authoritative, monotonic list of local slots (a slot is an
//! index into it) and becomes the function's `name_table`. `live` holds the
//! bindings currently in scope, and `marks` records where each nested scope
//! began, so a `do` block or `match` arm can be entered and unwound without
//! disturbing the slots it allocated.
//!
//! This does slot bookkeeping only: no IR, no diagnostics. It returns slots and
//! a bookkeeping-level error (the original binding's span); the lowering code
//! turns those into opcodes and `CompilerError`s.

#[cfg(test)]
mod tests;

use frost_parse::ast::SourceSpan;
use frost_runtime::NameEntry;

/// One in-scope binding: the slot it occupies and the span that introduced it,
/// kept so a later redefinition can point back at the original.
#[derive(Debug)]
struct Local {
    slot: usize,
    span: SourceSpan,
}

/// A function's locals, resolved across nested lexical scopes onto one flat
/// slot table.
#[derive(Debug, Default)]
pub(super) struct Locals {
    table: Vec<NameEntry>,
    live: Vec<Local>,
    marks: Vec<usize>,
}

impl Locals {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Introduce `name` in the current scope, returning its slot.
    ///
    /// Errors with the original binding's span when `name` is already bound in
    /// this same scope; shadowing a binding from an outer scope is allowed and
    /// allocates a fresh slot.
    pub(super) fn define(
        &mut self,
        name: String,
        span: SourceSpan,
        exported: bool,
    ) -> Result<usize, SourceSpan> {
        let floor = self.marks.last().copied().unwrap_or(0);
        if let Some(original) = self.live[floor..]
            .iter()
            .find(|local| self.table[local.slot].name == name)
        {
            return Err(original.span);
        }
        let slot = self.table.len();
        self.table.push(NameEntry { name, exported });
        self.live.push(Local { slot, span });
        Ok(slot)
    }

    /// Resolve `name` to its slot, innermost binding first (shadowing).
    pub(super) fn resolve(&self, name: &str) -> Option<usize> {
        self.live
            .iter()
            .rev()
            .find(|local| self.table[local.slot].name == name)
            .map(|local| local.slot)
    }

    /// Open a nested scope (a `do` block or a `match` arm).
    pub(super) fn enter(&mut self) {
        self.marks.push(self.live.len());
    }

    /// Close the innermost scope, dropping its bindings from view. The slots
    /// they allocated stay in `table` (never reclaimed at -O0).
    pub(super) fn exit(&mut self) {
        let mark = self.marks.pop().expect("exit without a matching enter");
        self.live.truncate(mark);
    }

    /// Consume the resolver, yielding the function's `name_table`.
    pub(super) fn into_name_table(self) -> Vec<NameEntry> {
        self.table
    }
}
