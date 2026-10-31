use crate::bytecode::Span;
use frostlang_parse::ast::{Expr, Spanned, Statement};

use crate::compile::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, Position, clear_locals},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_do_expression(
        &mut self,
        body: &[Spanned<Statement>],
        value: &Spanned<Expr>,
        position: Position,
        span: Span,
    ) -> Result<ExprFragment, Diagnostics> {
        self.in_scope(|this| {
            let start = this.locals.allocation_point();
            let statements = body
                .iter()
                .map(|stmt| this.compile_statement(stmt, Position::Inner))
                .collect::<Result<Vec<_>, _>>()?;
            let tail = this.compile_expression(value, position)?;

            let foldable = tail.foldable && statements.iter().all(|stmt| stmt.foldable);
            // A body that cannot fold blocks the tail folding any higher, so the
            // tail folds here (see the folding rule in the `fold` module doc).
            let tail = if foldable {
                tail
            } else {
                this.fold_if_eligible(tail)
            };

            // The block's locals are done with once its value is made.
            let clear = clear_locals(this.locals.allocated_since(start), position, span);
            Ok(ExprFragment {
                code: statements
                    .into_iter()
                    .flat_map(|stmt| stmt.code)
                    .chain(tail.code)
                    .chain(clear)
                    .collect(),
                foldable,
            })
        })
    }
}
