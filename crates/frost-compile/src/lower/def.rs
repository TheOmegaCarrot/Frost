use frost_parse::ast::{Destructure, Expr, Spanned};

use crate::{
    CompilerErrors,
    lower::{FunctionBuilder, Position, StatementFragment, fold::constant_of},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_def(
        &mut self,
        expr: &Spanned<Expr>,
        destructure: &Spanned<Destructure>,
        exported: bool,
    ) -> Result<StatementFragment, CompilerErrors> {
        // A top-level binding is implicitly exported when the option is on.
        let exported = exported || self.exports_implicitly();

        // The rhs is a fold point: a def whose value is compile-time known binds
        // a folded constant.
        let expr_fragment = self.compile_expression(expr, Position::Inner)?;
        let expr_fragment = self.fold_if_eligible(expr_fragment);

        // With propagation on, a binding whose rhs is compile-time known records
        // its value so name lookups can propagate it.
        let constant = self
            .options
            .optimization_options
            .constant_propagate
            .then(|| constant_of(&expr_fragment.code))
            .flatten();
        let destructure_fragment = self.compile_destructure(destructure, exported, constant)?;

        Ok(StatementFragment {
            foldable: expr_fragment.foldable && destructure_fragment.foldable,
            code: expr_fragment
                .code
                .into_iter()
                .chain(destructure_fragment.code)
                .collect(),
        })
    }
}
