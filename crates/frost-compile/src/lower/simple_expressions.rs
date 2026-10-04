#[cfg(test)]
mod tests;

use std::num::FpCategory;

use frost_parse::ast::{Expr, Literal, SourceSpan, Spanned, UnaryOp};
use frost_runtime::{Bytecode, FrostError, FrostFloat};

use crate::{
    CompilerErrors,
    lower::{
        ExprFragment, FunctionBuilder, Ir, Position, canonical_name,
        fold::value_to_ir,
        globals::{global_pure, global_slot},
    },
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_literal(
        &self,
        literal: &Literal,
        span: SourceSpan,
    ) -> Result<ExprFragment, CompilerErrors> {
        Ok(ExprFragment {
            code: vec![match literal {
                Literal::Null => Ir::Ready(Bytecode::PushNull),
                Literal::Bool(b) => Ir::Ready(match b {
                    true => Bytecode::PushTrue,
                    false => Bytecode::PushFalse,
                }),
                Literal::Int(i) => Ir::Ready(Bytecode::PushInt(*i)),
                Literal::Float(f) => Ir::Ready(Bytecode::PushFloat(self.validate_float(*f, span)?)),
                Literal::String(s) => Ir::Const(s.as_str().into()),
                Literal::Bytes(bytes) => Ir::Const(bytes.clone().into()),
            }],
            foldable: true,
        })
    }

    fn validate_float(&self, f: f64, span: SourceSpan) -> Result<FrostFloat, CompilerErrors> {
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
    ) -> Result<ExprFragment, CompilerErrors> {
        // Locals (including a function's captures, and shadowing any global)
        // win over globals; a name that is neither is a compile error.
        let name = canonical_name(name);
        if let Some(id) = self.locals.resolve(name) {
            // A compile-time-known binding is fold-eligible, and propagated as its
            // constant. A known value that cannot be a constant, such as a
            // function, is still loaded from the local, but a fold can use it.
            let constant = self.locals.constant(id);
            let load = constant
                .and_then(|value| value_to_ir(value.clone()))
                .unwrap_or(Ir::LoadLocal(id));
            return Ok(ExprFragment {
                code: vec![load],
                foldable: constant.is_some(),
            });
        }
        if let Some(slot) = global_slot(name) {
            return Ok(self.load_global(slot));
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
    pub(super) fn load_global(&mut self, slot: usize) -> ExprFragment {
        let pure = global_pure(slot);
        self.effectful |= !pure;
        ExprFragment {
            code: vec![Ir::Ready(Bytecode::LoadGlobal(slot))],
            foldable: pure,
        }
    }

    pub(super) fn compile_unary(
        &mut self,
        op: &Spanned<UnaryOp>,
        operand: &Spanned<Expr>,
    ) -> Result<ExprFragment, CompilerErrors> {
        let operand = self.compile_expression(operand, Position::Inner)?;

        let operation = Ir::Ready(match op.node {
            UnaryOp::Negate => Bytecode::Negate,
            UnaryOp::Not => Bytecode::LogicalNot,
        });

        Ok(ExprFragment {
            code: operand.code.into_iter().chain([operation]).collect(),
            foldable: operand.foldable,
        })
    }
}
