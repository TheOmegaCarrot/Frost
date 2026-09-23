use frost_parse::ast::{BinOp, Expr, LogicalOp, Spanned};
use frost_runtime::Bytecode;

use crate::{
    CompilerErrors,
    lower::{ExprFragment, FunctionBuilder, Ir, JumpType},
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
                .chain([operation])
                .collect(),
            foldable,
        })
    }

    pub(super) fn compile_logical(
        &mut self,
        left: &Spanned<Expr>,
        op: &Spanned<LogicalOp>,
        right: &Spanned<Expr>,
    ) -> Result<ExprFragment, CompilerErrors> {
        let lhs = self.compile_expression(left)?;
        let rhs = self.compile_expression(right)?;
        let ([lhs, rhs], foldable) = self.fold_siblings([lhs, rhs]);

        let past = self.next_label();

        let jump_over = Ir::Jump {
            label: past,
            kind: match op.node {
                LogicalOp::And => JumpType::PeekIfFalse,
                LogicalOp::Or => JumpType::PeekIfTrue,
            },
        };

        // TODO: separate optimization: branch elimination
        // If lhs is a known constant, its truthiness can be determined at compile time,
        // and thus drop the test, and *just* insert the lhs or rhs as appropriate.
        //
        // Nontrivial cases would require unconditionally folding lhs,
        // even if it's not the maximal subtree, but that's ok.

        Ok(ExprFragment {
            code: lhs
                .code
                .into_iter()
                .chain([jump_over, Ir::Ready(Bytecode::Pop)])
                .chain(rhs.code)
                .chain([Ir::Label(past)])
                .collect(),
            foldable,
        })
    }
}
