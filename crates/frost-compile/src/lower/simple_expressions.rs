use std::num::FpCategory;

use frost_parse::ast::{Literal, SourceSpan};
use frost_runtime::{Bytecode, FrostError, FrostFloat};

use crate::{
    CompilerErrors,
    lower::{FunctionBuilder, Ir, IrFragment},
};

impl FunctionBuilder<'_> {
    pub fn compile_literal(
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
}
