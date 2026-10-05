use frostlang_parse::ast::{Expr, Spanned};
use frostlang_runtime::Bytecode;

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder, Ir, JumpType, Position, fold::constant_of},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_if_expression(
        &mut self,
        condition: &Spanned<Expr>,
        consequent: &Spanned<Expr>,
        alternate: &Option<Box<Spanned<Expr>>>,
        position: Position,
    ) -> Result<ExprFragment, CompilerErrors> {
        // Each branch is the last code its path runs: the consequent is followed
        // only by the jump past the alternate, the alternate only by the end.
        let condition = self.compile_expression(condition, Position::Inner)?;
        let consequent = self.compile_expression(consequent, position)?;
        let alternate = alternate
            .as_deref()
            .map(|expr| self.compile_expression(expr, position))
            .transpose()?
            .unwrap_or(ExprFragment {
                code: vec![Ir::Ready(Bytecode::PushNull)],
                foldable: true,
            });

        let ([condition, consequent, alternate], foldable) =
            self.fold_siblings([condition, consequent, alternate]);

        if self.options.optimization_options.branch_eliminate
            && let Some(condition_value) = constant_of(&condition.code)
        {
            // TODO: possibly wasting a fold of the discarded branch
            return Ok(if condition_value.is_truthy() {
                consequent
            } else {
                alternate
            });
        }

        let to_alternate = self.next_label();
        let past_alternate = self.next_label();

        Ok(ExprFragment {
            foldable,
            code: condition
                .code
                .into_iter()
                .chain([Ir::Jump {
                    label: to_alternate,
                    kind: JumpType::IfFalse,
                }])
                .chain(consequent.code)
                .chain([
                    Ir::Jump {
                        label: past_alternate,
                        kind: JumpType::Unconditional,
                    },
                    Ir::Label(to_alternate),
                ])
                .chain(alternate.code)
                .chain([Ir::Label(past_alternate)])
                .collect(),
        })
    }
}
