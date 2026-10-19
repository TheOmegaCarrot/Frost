#[cfg(test)]
mod tests;

use std::num::FpCategory;

use crate::bytecode::{Bytecode, Span};
use crate::vm::to_operand;
use crate::{FrostError, FrostFloat};
use frostlang_parse::ast::{Expr, Literal, SourceSpan, Spanned, UnaryOp};

use crate::compile::{
    Diagnostics,
    lower::{
        ExprFragment, FunctionBuilder, Op, Position, canonical_name, debug_span,
        fold::value_to_op,
        globals::{global_pure, global_slot},
    },
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_literal(
        &self,
        literal: &Literal,
        span: SourceSpan,
    ) -> Result<ExprFragment, Diagnostics> {
        let op = match literal {
            Literal::Null => Op::Ready(Bytecode::PushNull),
            Literal::Bool(b) => Op::Ready(match b {
                true => Bytecode::PushTrue,
                false => Bytecode::PushFalse,
            }),
            Literal::Int(i) => Op::Ready(Bytecode::PushInt(*i)),
            Literal::Float(f) => Op::Ready(Bytecode::PushFloat(self.validate_float(*f, span)?)),
            Literal::String(s) => Op::Const(s.as_str().into()),
            Literal::Bytes(bytes) => Op::Const(bytes.clone().into()),
        };
        Ok(ExprFragment {
            code: vec![op.at(debug_span(span))],
            foldable: true,
        })
    }

    fn validate_float(&self, f: f64, span: SourceSpan) -> Result<FrostFloat, Diagnostics> {
        FrostFloat::new(f).map_err(|err| {
            self.error(err.message().into())
                .code("invalid float".into())
                .label_primary(span, "here".to_owned())
                .into()
        })
    }

    pub(super) fn compile_name_lookup(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<ExprFragment, Diagnostics> {
        // Locals (including a function's captures, and shadowing any global)
        // win over globals; a name that is neither is a compile error.
        let name = canonical_name(name);
        if let Some(id) = self.locals.resolve(name) {
            self.locals.mark_used(id);
            // A compile-time-known binding is fold-eligible, and propagated as its
            // constant, loaded here at its use. A known value that cannot be a
            // constant, such as a function, is still loaded from the local, but a
            // fold can use it. So is a pending one (see `Locals::mark_pending`).
            let constant = self.locals.constant(id);
            let load = constant
                .and_then(|value| value_to_op(value.clone()))
                .unwrap_or(Op::LoadLocal(id));
            return Ok(ExprFragment {
                code: vec![load.at(debug_span(span))],
                foldable: constant.is_some() || self.locals.is_pending(id),
            });
        }
        if let Some(slot) = global_slot(name) {
            return Ok(self.load_global(slot, debug_span(span)));
        }
        Err(self
            .error(format!("`{name}` is not defined"))
            .code("unbound name".into())
            .label_primary(span, "not found in this scope".into())
            .into())
    }

    /// Load the global at `slot`. A pure global is fold-eligible: a call to it
    /// over foldable arguments can be evaluated at compile time. An impure one
    /// makes the whole function effectful.
    pub(super) fn load_global(&mut self, slot: usize, span: Span) -> ExprFragment {
        let pure = global_pure(slot);
        self.effectful |= !pure;
        ExprFragment {
            code: vec![Op::Ready(Bytecode::LoadGlobal(to_operand(slot))).at(span)],
            foldable: pure,
        }
    }

    pub(super) fn compile_unary(
        &mut self,
        op: &Spanned<UnaryOp>,
        operand: &Spanned<Expr>,
        span: Span,
    ) -> Result<ExprFragment, Diagnostics> {
        if op.node == UnaryOp::Negate {
            self.lint_consecutive_minus(op.span);
        }
        let operand = self.compile_expression(operand, Position::Inner)?;

        let operation = Op::Ready(match op.node {
            UnaryOp::Negate => Bytecode::Negate,
            UnaryOp::Not => Bytecode::LogicalNot,
        })
        .at(span);

        Ok(ExprFragment {
            code: operand.code.into_iter().chain([operation]).collect(),
            foldable: operand.foldable,
        })
    }
}
