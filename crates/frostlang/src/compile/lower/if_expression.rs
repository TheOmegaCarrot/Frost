use crate::bytecode::Bytecode;
use frostlang_parse::ast::{Expr, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, Ir, JumpType, Position, fold::constant_of},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_if_expression(
        &mut self,
        condition: &Spanned<Expr>,
        consequent: &Spanned<Expr>,
        alternate: &Option<Box<Spanned<Expr>>>,
        position: Position,
    ) -> Result<ExprFragment, Diagnostics> {
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

        // The folding rule of `fold_siblings`, applied a child at a time so that
        // the branch a constant condition discards is never folded.
        let all_foldable = condition.foldable && consequent.foldable && alternate.foldable;
        let fold = |child| {
            if all_foldable {
                child
            } else {
                self.fold_if_eligible(child)
            }
        };

        let condition = fold(condition);
        if self.options.optimization_options.branch_eliminate
            && let Some(condition_value) = constant_of(&condition.code)
        {
            return Ok(fold(if condition_value.is_truthy() {
                consequent
            } else {
                alternate
            }));
        }
        let consequent = fold(consequent);
        let alternate = fold(alternate);

        let to_alternate = self.next_label();
        let past_alternate = self.next_label();

        Ok(ExprFragment {
            foldable: all_foldable,
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
