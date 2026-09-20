use frost_parse::ast::{Binding, Destructure, Expr, Spanned};
use frost_runtime::Bytecode;

use crate::{
    CompilerError, CompilerErrors,
    lower::{
        FunctionBuilder, Ir,
        fold::constant_of,
        locals::{LocalInfo, LocalKind},
    },
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
                    // With propagation on, a binding whose rhs is compile-time
                    // known records its value so name lookups can propagate it.
                    let constant = self
                        .options
                        .optimization_options
                        .constant_propagate
                        .then(|| constant_of(&expr_fragment.code))
                        .flatten();
                    let id = self
                        .locals
                        .define(LocalInfo {
                            name: name.clone(),
                            span: binding.span,
                            exported,
                            constant,
                            kind: LocalKind::Binding,
                        })
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
                        code: vec![Ir::DefLocal(id)],
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
