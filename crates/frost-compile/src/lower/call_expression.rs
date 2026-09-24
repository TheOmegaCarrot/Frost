use std::iter;

use frost_parse::ast::{Expr, Spanned};
use frost_runtime::Bytecode;

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder, Ir},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_call_expression(
        &mut self,
        callee: &Spanned<Expr>,
        args: &[Spanned<Expr>],
    ) -> Result<ExprFragment, CompilerErrors> {
        let arity = args.len();

        let callee = self.compile_expression(callee)?;
        let args = args
            .iter()
            .map(|expr| self.compile_expression(expr))
            .collect::<Result<Vec<_>, _>>()?;

        let (exprs, foldable) = self.fold_sibling_list(iter::once(callee).chain(args).collect());

        Ok(ExprFragment {
            foldable,
            code: exprs
                .into_iter()
                .flat_map(|expr| expr.code)
                .chain([Ir::Ready(Bytecode::Call(arity))])
                .collect(),
        })
    }
}
