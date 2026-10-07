use crate::bytecode::Bytecode;
use frostlang_parse::ast::{BinOp, Expr, LogicalOp, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, Ir, JumpType, Position, fold::constant_of},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_binop(
        &mut self,
        left: &Spanned<Expr>,
        op: &Spanned<BinOp>,
        right: &Spanned<Expr>,
    ) -> Result<ExprFragment, Diagnostics> {
        let lhs = self.compile_expression(left, Position::Inner)?;
        let rhs = self.compile_expression(right, Position::Inner)?;
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
        position: Position,
    ) -> Result<ExprFragment, Diagnostics> {
        // When the right operand runs, its value is the result: nothing follows
        // it but the join label.
        let lhs = self.compile_expression(left, Position::Inner)?;
        let rhs = self.compile_expression(right, position)?;
        let ([lhs, rhs], foldable) = self.fold_siblings([lhs, rhs]);

        // A constant left operand decides the test now: emit only the operand
        // the expression yields.
        if self.options.optimization_options.branch_eliminate
            && let Some(left_value) = constant_of(&lhs.code)
        {
            let left_decides = match op.node {
                LogicalOp::And => !left_value.is_truthy(),
                LogicalOp::Or => left_value.is_truthy(),
            };
            return Ok(if left_decides { lhs } else { rhs });
        }

        let past = self.next_label();

        let jump_over = Ir::Jump {
            label: past,
            kind: match op.node {
                LogicalOp::And => JumpType::PeekIfFalse,
                LogicalOp::Or => JumpType::PeekIfTrue,
            },
        };

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
