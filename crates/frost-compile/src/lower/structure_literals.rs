use frost_parse::ast::{Expr, MapEntry, Spanned};
use frost_runtime::Bytecode;

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder, Ir, Position},
};

impl FunctionBuilder<'_> {
    /// `[a, b, c]`
    pub(super) fn compile_array_literal(
        &mut self,
        elements: &[Spanned<Expr>],
    ) -> Result<ExprFragment, CompilerErrors> {
        let count = elements.len();

        let elements = elements
            .iter()
            .map(|expr| self.compile_expression(expr, Position::Inner))
            .collect::<Result<Vec<_>, _>>()?;
        let (elements, foldable) = self.fold_sibling_list(elements);

        Ok(ExprFragment {
            foldable,
            code: elements
                .into_iter()
                .flat_map(|fragment| fragment.code.into_iter())
                .chain([Ir::Ready(Bytecode::MakeArray(count))])
                .collect(),
        })
    }

    /// `{ [k1]: v1, [k2]: v2 }`
    pub(super) fn compile_map_literal(
        &mut self,
        entries: &[Spanned<MapEntry>],
    ) -> Result<ExprFragment, CompilerErrors> {
        let pair_count = entries.len();

        let exprs = entries
            .iter()
            .flat_map(|entry| [&entry.node.key, &entry.node.value])
            .map(|expr| self.compile_expression(expr, Position::Inner))
            .collect::<Result<Vec<_>, _>>()?;

        let (exprs, foldable) = self.fold_sibling_list(exprs);

        Ok(ExprFragment {
            foldable,
            code: exprs
                .into_iter()
                .flat_map(|expr| expr.code)
                .chain([Ir::Ready(Bytecode::MakeMap(pair_count))])
                .collect(),
        })
    }
}
