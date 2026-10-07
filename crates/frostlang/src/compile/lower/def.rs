use frostlang_parse::ast::{Destructure, Expr, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{FunctionBuilder, Position, StatementFragment},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_def(
        &mut self,
        expr: &Spanned<Expr>,
        destructure: &Spanned<Destructure>,
        exported: bool,
    ) -> Result<StatementFragment, Diagnostics> {
        // A top-level binding is implicitly exported when the option is on.
        let exported = exported || self.exports_implicitly();

        // The rhs is a fold point: a def whose value is compile-time known binds
        // a folded constant. With propagation on, the binding also records that
        // value so name lookups can propagate it.
        let expr_fragment = self.compile_expression(expr, Position::Inner)?;
        let (expr_fragment, constant) = if self.options.optimization_options.constant_propagate {
            self.fold_binding_value(expr_fragment)
        } else {
            (self.fold_if_eligible(expr_fragment), None)
        };
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
