use frost_parse::ast::{Expr, Spanned};

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_binop(
        &self,
        left: &Spanned<Expr>,
        op: &Spanned<frost_parse::ast::BinOp>,
        right: &Spanned<Expr>,
    ) -> Result<ExprFragment, CompilerErrors> {
        todo!()
    }
}
