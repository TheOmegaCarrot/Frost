use std::iter;

use crate::bytecode::{Bytecode, Span};
use crate::vm::to_operand;
use frostlang_parse::ast::{Expr, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, Op, Position},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_call_expression(
        &mut self,
        callee: &Spanned<Expr>,
        args: &[Spanned<Expr>],
        position: Position,
        span: Span,
    ) -> Result<ExprFragment, Diagnostics> {
        let callee = self.compile_expression(callee, Position::Inner)?;
        let args = args
            .iter()
            .map(|expr| self.compile_expression(expr, Position::Inner))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(self.call(callee, args, position, span))
    }

    /// Call the already-compiled `callee` with the already-compiled `args`,
    /// which run in that order before the call. The call is derived from the
    /// source at `span`.
    pub(super) fn call(
        &self,
        callee: ExprFragment,
        args: Vec<ExprFragment>,
        position: Position,
        span: Span,
    ) -> ExprFragment {
        let arity = args.len();

        let (exprs, foldable) = self.fold_sibling_list(iter::once(callee).chain(args).collect());

        let call = match position {
            Position::Tail => Bytecode::TailCall(to_operand(arity)),
            Position::Inner => Bytecode::Call(to_operand(arity)),
        };

        ExprFragment {
            foldable,
            code: exprs
                .into_iter()
                .flat_map(|expr| expr.code)
                .chain([Op::Ready(call).at(span)])
                .collect(),
        }
    }
}
