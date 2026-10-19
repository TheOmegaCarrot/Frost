use crate::bytecode::{Bytecode, Span};
use frostlang_parse::ast::{Expr, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, JumpType, Op, Position, fold::constant_of},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_if_expression(
        &mut self,
        condition: &Spanned<Expr>,
        consequent: &Spanned<Expr>,
        alternate: &Option<Box<Spanned<Expr>>>,
        position: Position,
        span: Span,
    ) -> Result<ExprFragment, Diagnostics> {
        // Each branch is the last code its path runs: the consequent is followed
        // only by the jump past the alternate, the alternate only by the end.
        let condition = self.compile_expression(condition, Position::Inner)?;
        let consequent = self.compile_expression(consequent, position)?;
        let alternate = alternate
            .as_deref()
            .map(|expr| self.compile_expression(expr, position))
            .transpose()?
            // An omitted `else` is the `if`'s own.
            .unwrap_or(ExprFragment {
                code: vec![Op::Ready(Bytecode::PushNull).at(span)],
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
                .chain([Op::Jump {
                    label: to_alternate,
                    kind: JumpType::IfFalse,
                }
                .at(span)])
                .chain(consequent.code)
                .chain([
                    Op::Jump {
                        label: past_alternate,
                        kind: JumpType::Unconditional,
                    }
                    .at(span),
                    Op::Label(to_alternate).at(span),
                ])
                .chain(alternate.code)
                .chain([Op::Label(past_alternate).at(span)])
                .collect(),
        })
    }
}
