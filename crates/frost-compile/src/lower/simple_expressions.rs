use std::num::FpCategory;

use frost_parse::ast::{Literal, SourceSpan};
use frost_runtime::{Bytecode, FrostError, FrostFloat};

use crate::{
    CompilerErrors,
    lower::{FunctionBuilder, Ir, IrFragment},
};

impl FunctionBuilder<'_> {
    pub(super) fn compile_literal(
        &self,
        literal: &Literal,
        span: SourceSpan,
    ) -> Result<IrFragment, CompilerErrors> {
        Ok(IrFragment {
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
    ) -> Result<IrFragment, CompilerErrors> {
        // Locals (including a lambda's seeded captures, and shadowing any global)
        // win over globals; an unresolved name is a compile error. Inside a
        // lambda every free name was reserved as a capture by the pre-walk, so
        // this error only fires at the top level.
        let load = if let Some(slot) = self.locals.resolve(name) {
            Bytecode::LoadLocal(slot)
        } else if let Some(slot) = super::globals::global_slot(name) {
            Bytecode::LoadGlobal(slot)
        } else {
            return Err(self
                .error(format!("`{name}` is not defined"))
                .code("unbound name".into())
                .label_primary(span, "not found in this scope".into())
                .into());
        };
        Ok(IrFragment {
            code: vec![Ir::Ready(load)],
        })
    }
}
