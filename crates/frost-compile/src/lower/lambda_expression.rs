use frost_parse::ast::{Expr, Spanned};

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder},
};

impl FunctionBuilder<'_> {
    /// `fn name(a, b, ...rest) -> { body; return_expr }` or `$(body)`.
    ///
    /// `lambda` must be an [`Expr::Lambda`] or [`Expr::AbbreviatedLambda`].
    pub(super) fn compile_lambda(
        &mut self,
        lambda: &Spanned<Expr>,
    ) -> Result<ExprFragment, CompilerErrors> {
        todo!()
    }
}
