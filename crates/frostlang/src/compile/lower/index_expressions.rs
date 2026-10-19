use crate::MapKey;
use crate::bytecode::{Bytecode, Span};
use frostlang_parse::ast::{Expr, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{ConstKeyOp, ExprFragment, FunctionBuilder, Op, Position},
};

impl FunctionBuilder<'_> {
    /// `target[key]`
    pub(super) fn compile_soft_index(
        &mut self,
        target: &Spanned<Expr>,
        key: &Spanned<Expr>,
        span: Span,
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
                .chain([Op::Ready(Bytecode::SoftIndexStructure).at(span)])
                .collect(),
        })
    }

    /// `target.key`
    pub(super) fn compile_hard_index(
        &mut self,
        target: &Spanned<Expr>,
        key: &Spanned<String>,
        span: Span,
    ) -> Result<ExprFragment, Diagnostics> {
        let target = self.compile_expression(target, Position::Inner)?;

        Ok(ExprFragment {
            foldable: target.foldable,
            code: target
                .code
                .into_iter()
                .chain([Op::ConstKey {
                    op: ConstKeyOp::HardIndex,
                    key: MapKey::from(key.node.clone()),
                }
                .at(span)])
                .collect(),
        })
    }
}
