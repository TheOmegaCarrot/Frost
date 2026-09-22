use std::iter;

use frost_parse::ast::{BinOp, Expr, Spanned};
use frost_runtime::Bytecode;

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder, Ir},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_binop(
        &mut self,
        left: &Spanned<Expr>,
        op: &Spanned<BinOp>,
        right: &Spanned<Expr>,
    ) -> Result<ExprFragment, CompilerErrors> {
        let lhs = self.compile_expression(left)?;
        let rhs = self.compile_expression(right)?;
        let ([lhs, rhs], foldable) = self.fold_siblings([lhs, rhs]);

        let operation = Ir::Ready(match op.node {
            BinOp::Add => Bytecode::Add,
            BinOp::Sub => Bytecode::Subtract,
            BinOp::Mul => Bytecode::Multiply,
            BinOp::Div => Bytecode::Divide,
            BinOp::Mod => Bytecode::Modulus,
            BinOp::Eq => Bytecode::CompareEqual,
            BinOp::Neq => Bytecode::CompareNotEqual,
            BinOp::Lt => Bytecode::CompareLessThan,
            BinOp::Lte => Bytecode::CompareLessThanOrEqual,
            BinOp::Gt => Bytecode::CompareGreaterThan,
            BinOp::Gte => Bytecode::CompareGreaterThanOrEqual,
        });

        Ok(ExprFragment {
            code: lhs
                .code
                .into_iter()
                .chain(rhs.code)
                .chain(iter::once(operation))
                .collect(),
            foldable,
        })
    }
}
