use std::iter;

use frostlang_parse::ast::{Expr, Spanned};
use frostlang_runtime::Bytecode;

use crate::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, Ir, Position},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_call_expression(
        &mut self,
        callee: &Spanned<Expr>,
        args: &[Spanned<Expr>],
        position: Position,
    ) -> Result<ExprFragment, Diagnostics> {
        let callee = self.compile_expression(callee, Position::Inner)?;
        let args = args
            .iter()
            .map(|expr| self.compile_expression(expr, Position::Inner))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(self.call(callee, args, position))
    }

    /// Call the already-compiled `callee` with the already-compiled `args`,
    /// which run in that order before the call.
    pub(super) fn call(
        &self,
        callee: ExprFragment,
        args: Vec<ExprFragment>,
        position: Position,
    ) -> ExprFragment {
        let arity = args.len();

        let (exprs, foldable) = self.fold_sibling_list(iter::once(callee).chain(args).collect());

        let call = match position {
            Position::Tail => Bytecode::TailCall(arity),
            Position::Inner => Bytecode::Call(arity),
        };

        ExprFragment {
            foldable,
            code: exprs
                .into_iter()
                .flat_map(|expr| expr.code)
                .chain([Ir::Ready(call)])
                .collect(),
        }
    }
}
