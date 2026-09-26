use frost_parse::ast::{Binding, Destructure, Expr, Spanned};
use frost_runtime::Bytecode;

use crate::{
    CompilerErrors,
    lower::{
        FunctionBuilder, Ir, Position, StatementFragment,
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
    ) -> Result<StatementFragment, CompilerErrors> {
        // A top-level binding is implicitly exported when the option is on.
        let exported = exported || self.exports_implicitly();

        // The rhs is a fold point: a def whose value is compile-time known binds
        // a folded constant.
        let expr_fragment = self.compile_expression(expr, Position::Inner)?;
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
                        .map_err(|original| self.duplicate_binding(name, binding.span, original))?;
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

        // Binding a value adds no runtime input, so the def is as foldable as its rhs.
        Ok(StatementFragment {
            code: expr_fragment
                .code
                .into_iter()
                .chain(destructure_fragment.code)
                .collect(),
            foldable: expr_fragment.foldable,
        })
    }
}
