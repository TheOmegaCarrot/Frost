use crate::bytecode::{Bytecode, Span};
use crate::vm::to_operand;
use frostlang_parse::ast::{Expr, MapEntry, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, Op, Position},
};

impl FunctionBuilder<'_> {
    /// `[a, b, c]`
    pub(super) fn compile_array_literal(
        &mut self,
        elements: &[Spanned<Expr>],
        span: Span,
    ) -> Result<ExprFragment, Diagnostics> {
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
                .chain([Op::Ready(Bytecode::MakeArray(to_operand(count))).at(span)])
                .collect(),
        })
    }

    /// `{ [k1]: v1, [k2]: v2 }`
    pub(super) fn compile_map_literal(
        &mut self,
        entries: &[Spanned<MapEntry>],
        span: Span,
    ) -> Result<ExprFragment, Diagnostics> {
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
                .chain([Op::Ready(Bytecode::MakeMap(to_operand(pair_count))).at(span)])
                .collect(),
        })
    }
}
