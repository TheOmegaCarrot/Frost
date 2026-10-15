use frostlang_parse::ast::{Binding, Expr, Pattern, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{
        FunctionBuilder, Ir, Position, StatementFragment,
        fold::value_to_ir,
        pattern::{BindMode, Mismatch},
    },
};

impl FunctionBuilder<'_> {
    /// `def pattern = expr`.
    ///
    /// A def whose value is compile-time known, and whose pattern uses no other
    /// runtime input, is matched now, by running it on the fold VM. With
    /// propagation on, each name records the part of the value it took. With
    /// folding on, a match whose parts can all be constants compiles to just
    /// binding them. If the pattern raises, the def is left for runtime.
    pub(super) fn compile_def(
        &mut self,
        expr: &Spanned<Expr>,
        pattern: &Spanned<Pattern>,
        exported: bool,
    ) -> Result<StatementFragment, Diagnostics> {
        // A top-level binding is implicitly exported when the option is on.
        let exported = exported || self.exports_implicitly();
        let fold = self.options.optimization_options.constant_fold;
        let propagate = self.options.optimization_options.constant_propagate;

        // The rhs is a fold point.
        let expr_fragment = self.compile_expression(expr, Position::Inner)?;
        let (expr_fragment, known) = self.fold_binding_value(expr_fragment);

        // A plain name takes the whole value, so it needs no run. Any other
        // pattern's names are pending until it has run: a part of the pattern
        // that reads one, as a computed key may, still uses no runtime input.
        let plain = matches!(
            pattern.node,
            Pattern::Binding {
                type_constraint: None,
                ..
            }
        );
        if !binds_a_name(&pattern.node) {
            return Err(self
                .error("this `def` binds no name".to_owned())
                .code("binds nothing".into())
                .label_primary(pattern.span, "no name bound here".into())
                .help("to check a value without binding it, use `assert` or `match`".into())
                .into());
        }
        let pending = known.is_some() && !plain && (fold || propagate);
        let mut bound = Vec::new();
        let pattern_fragment = self.compile_pattern(
            pattern,
            Mismatch::Raise,
            BindMode::Def { exported, pending },
            &mut bound,
        )?;

        let ids: Vec<_> = bound.iter().map(|bound| bound.id).collect();
        let matched = match known {
            Some(value) if plain => Some(vec![value]),
            Some(_) if pending && pattern_fragment.foldable => {
                let code = expr_fragment
                    .code
                    .iter()
                    .chain(&pattern_fragment.code)
                    .cloned()
                    .collect();
                self.fold_bindings(code, &ids)
            }
            _ => None,
        };
        self.locals.settle(&ids);

        let mut code = None;
        if let Some(values) = matched {
            if fold && !plain {
                code = values
                    .iter()
                    .zip(&ids)
                    .map(|(value, id)| Some([value_to_ir(value.clone())?, Ir::DefLocal(*id)]))
                    .collect::<Option<Vec<_>>>()
                    .map(|binds| binds.into_iter().flatten().collect());
            }
            if propagate {
                for (id, value) in ids.into_iter().zip(values) {
                    self.locals.set_constant(id, value);
                }
            }
        }

        Ok(match code {
            Some(code) => StatementFragment {
                code,
                foldable: true,
            },
            None => StatementFragment {
                foldable: expr_fragment.foldable && pattern_fragment.foldable,
                code: expr_fragment
                    .code
                    .into_iter()
                    .chain(pattern_fragment.code)
                    .collect(),
            },
        })
    }
}

/// Whether `pattern` binds a name anywhere. In an alternative, any branch
/// binding one is enough here: one that binds different names is its own error.
fn binds_a_name(pattern: &Pattern) -> bool {
    let named = |binding: &Spanned<Binding>| matches!(binding.node, Binding::Named(_));
    match pattern {
        Pattern::Binding { name, .. } => named(name),
        Pattern::Value(_) => false,
        Pattern::Array { elements, rest } => {
            elements.iter().any(|element| binds_a_name(&element.node))
                || rest.as_ref().is_some_and(named)
        }
        Pattern::Map {
            entries,
            bind_whole,
        } => {
            entries
                .iter()
                .any(|entry| binds_a_name(&entry.node.pattern.node))
                || bind_whole.as_ref().is_some_and(named)
        }
        Pattern::Alternative(branches) => branches.iter().any(|branch| binds_a_name(&branch.node)),
    }
}
