use frost_parse::ast::{Expr, Spanned, Statement};

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_do_expression(
        &mut self,
        body: &[Spanned<Statement>],
        value: &Spanned<Expr>,
    ) -> Result<ExprFragment, CompilerErrors> {
        self.in_scope(|this| {
            let statements = body
                .iter()
                .map(|stmt| this.compile_statement(stmt, false))
                .collect::<Result<Vec<_>, _>>()?;
            let tail = this.compile_expression(value)?;

            let foldable = tail.foldable && statements.iter().all(|stmt| stmt.foldable);
            // A body that cannot fold blocks the tail folding any higher, so the
            // tail folds here (see the folding rule in the `fold` module doc).
            let tail = if foldable {
                tail
            } else {
                this.fold_if_eligible(tail)
            };

            Ok(ExprFragment {
                code: statements
                    .into_iter()
                    .flat_map(|stmt| stmt.code)
                    .chain(tail.code)
                    .collect(),
                foldable,
            })
        })
    }
}
