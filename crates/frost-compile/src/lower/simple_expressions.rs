#[cfg(test)]
mod tests;

use std::num::FpCategory;

use frost_parse::ast::{Expr, Literal, SourceSpan, Spanned, UnaryOp};
use frost_runtime::{Bytecode, FrostError, FrostFloat};

use crate::{
    CompilerErrors,
    lower::{
        ExprFragment, FunctionBuilder, Ir,
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
        &self,
        name: &str,
        span: SourceSpan,
    ) -> Result<ExprFragment, CompilerErrors> {
        // Locals (including a lambda's seeded captures, and shadowing any global)
        // win over globals; an unresolved name is a compile error. Inside a
        // lambda every free name was reserved as a capture by the pre-walk, so
        // this error only fires at the top level.
        if let Some(id) = self.locals.resolve(name) {
            // A compile-time-known binding is propagated as its constant, and so
            // is itself fold-eligible; otherwise it is an ordinary local load.
            return Ok(match self.locals.constant(id) {
                Some(value) => ExprFragment {
                    code: vec![
                        value_to_ir(value.clone()).expect("a stored constant is representable"),
                    ],
                    foldable: true,
                },
                None => ExprFragment {
                    code: vec![Ir::LoadLocal(id)],
                    foldable: false,
                },
            });
        }
        if let Some(slot) = global_slot(name) {
            // A pure global is fold-eligible: a call to it over foldable
            // arguments can be evaluated at compile time.
            return Ok(ExprFragment {
                code: vec![Ir::Ready(Bytecode::LoadGlobal(slot))],
                foldable: global_pure(slot),
            });
        }
        Err(self
            .error(format!("`{name}` is not defined"))
            .code("unbound name".into())
            .label_primary(span, "not found in this scope".into())
            .into())
    }

    pub(super) fn compile_unary(
        &mut self,
        op: &Spanned<UnaryOp>,
        operand: &Spanned<Expr>,
    ) -> Result<ExprFragment, CompilerErrors> {
        let operand = self.compile_expression(operand)?;

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
