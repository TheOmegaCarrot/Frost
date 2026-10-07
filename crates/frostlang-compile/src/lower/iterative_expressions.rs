use frostlang_parse::ast::{Expr, Spanned};

use crate::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, Position, globals::global_slot},
};

/// An iteration form: syntax for a call to its global function.
#[derive(Clone, Copy, Debug)]
pub(super) enum Iteration {
    /// `filter structure with operation`: `select(structure, operation)`
    Filter,
    /// `map structure with operation`: `transform(structure, operation)`
    Map,
    /// `reduce structure [init: init] with operation`:
    /// `fold(structure, operation[, init])`
    Reduce,
    /// `foreach structure with operation`: `each(structure, operation)`
    Foreach,
}

impl Iteration {
    /// The global the form calls.
    fn global(self) -> &'static str {
        match self {
            Iteration::Filter => "select",
            Iteration::Map => "transform",
            Iteration::Reduce => "fold",
            Iteration::Foreach => "each",
        }
    }
}

impl FunctionBuilder<'_> {
    /// An iteration form, lowered to a call to its global. The global is loaded
    /// by slot, not by name, so a local that shadows its name does not change
    /// what the form calls.
    pub(super) fn compile_iteration(
        &mut self,
        iteration: Iteration,
        structure: &Spanned<Expr>,
        operation: &Spanned<Expr>,
        init: Option<&Spanned<Expr>>,
        position: Position,
    ) -> Result<ExprFragment, Diagnostics> {
        let slot = global_slot(iteration.global())
            .unwrap_or_else(|| panic!("`{}` is a global", iteration.global()));
        let callee = self.load_global(slot);

        let args = [structure, operation]
            .into_iter()
            .chain(init)
            .map(|expr| self.compile_expression(expr, Position::Inner))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(self.call(callee, args, position))
    }
}
