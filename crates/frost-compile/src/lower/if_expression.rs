use frost_parse::ast::{Expr, Spanned};
use frost_runtime::Bytecode;

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder, Ir, JumpType},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_if_expression(
        &mut self,
        condition: &Spanned<Expr>,
        consequent: &Spanned<Expr>,
        alternate: &Option<Box<Spanned<Expr>>>,
    ) -> Result<ExprFragment, CompilerErrors> {
        let condition = self.compile_expression(condition)?;
        let consequent = self.compile_expression(consequent)?;
        let alternate = alternate
            .as_deref()
            .map(|expr| self.compile_expression(expr))
            .transpose()?
            .unwrap_or(ExprFragment {
                code: vec![Ir::Ready(Bytecode::PushNull)],
                foldable: true,
            });

        let ([condition, consequent, alternate], foldable) =
            self.fold_siblings([condition, consequent, alternate]);

        // TODO: branch elimination handling

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
