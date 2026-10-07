use frostlang_parse::ast::{Expr, Spanned};
use frostlang_runtime::{Bytecode, MapKey};

use crate::{
    Diagnostics,
    lower::{ConstKeyOp, ExprFragment, FunctionBuilder, Ir, Position},
};

impl FunctionBuilder<'_> {
    /// `target[key]`
    pub(super) fn compile_soft_index(
        &mut self,
        target: &Spanned<Expr>,
        key: &Spanned<Expr>,
    ) -> Result<ExprFragment, Diagnostics> {
        let target = self.compile_expression(target, Position::Inner)?;
        let key = self.compile_expression(key, Position::Inner)?;

        let ([target, key], foldable) = self.fold_siblings([target, key]);

        Ok(ExprFragment {
            foldable,
            code: target
                .code
                .into_iter()
                .chain(key.code)
                .chain([Ir::Ready(Bytecode::SoftIndexStructure)])
                .collect(),
        })
    }

    /// `target.key`
    pub(super) fn compile_hard_index(
        &mut self,
        target: &Spanned<Expr>,
        key: &Spanned<String>,
    ) -> Result<ExprFragment, Diagnostics> {
        let target = self.compile_expression(target, Position::Inner)?;

        Ok(ExprFragment {
            foldable: target.foldable,
            code: target
                .code
                .into_iter()
                .chain([Ir::ConstKey {
                    op: ConstKeyOp::HardIndex,
                    key: MapKey::from(key.node.clone()),
                }])
                .collect(),
        })
    }
}
