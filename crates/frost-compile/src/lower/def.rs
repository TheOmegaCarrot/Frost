use frost_parse::ast::{Binding, Destructure, Expr, Spanned};
use frost_runtime::Bytecode;

use crate::{
    CompilerError, CompilerErrors,
    lower::{FunctionBuilder, Ir},
};

struct DestructureFragment {
    code: Vec<Ir>,
}

impl FunctionBuilder<'_> {
    pub(super) fn compile_def(
        &mut self,
        expr: &Spanned<Expr>,
        destructure: &Spanned<Destructure>,
        exported: bool,
    ) -> Result<Vec<Ir>, CompilerErrors> {
        // The rhs is a fold point: a def whose value is compile-time known binds
        // a folded constant.
        let expr_fragment = self.compile_expression(expr)?;
        let expr_fragment = self.fold_if_eligible(expr_fragment);

        let destructure_fragment = match &destructure.node {
            Destructure::Binding(binding) => match &binding.node {
                Binding::Named(name) => {
                    let slot = self
                        .locals
                        .define(name.clone(), binding.span, exported)
                        .map_err(|original| {
                            self.error(format!("`{name}` is already bound"))
                                .code("duplicate binding".into())
                                .label_primary(binding.span, "redefined here".into())
                                .related(
                                    CompilerError::advice(format!("`{name}` was first bound here"))
                                        .label(original, "original binding".into()),
                                )
                        })?;
                    DestructureFragment {
                        code: vec![Ir::Ready(Bytecode::DefLocal(slot))],
                    }
                }
                // A discard evaluates the expression for its effect, then drops it.
                Binding::Discarded => DestructureFragment {
                    code: vec![Ir::Ready(Bytecode::Pop)],
                },
            },
            Destructure::Array { elements, rest } => todo!(),
            Destructure::Map {
                entries,
                bind_whole,
            } => todo!(),
        };

        let mut stmt_code = expr_fragment.code;
        stmt_code.extend(destructure_fragment.code);

        Ok(stmt_code)
    }
}
