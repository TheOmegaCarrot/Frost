use std::num::NonZeroUsize;

use frostlang_parse::ast::FormatSegment;
use frostlang_runtime::{Bytecode, Value};

use crate::{
    Diagnostics,
    lower::{ExprFragment, FunctionBuilder, Ir, Position},
};

impl FunctionBuilder<'_> {
    /// `$'hello, ${name}'`
    pub(super) fn compile_format_string(
        &mut self,
        segments: &[FormatSegment],
    ) -> Result<ExprFragment, Diagnostics> {
        let segment_count = segments.len();

        if segment_count == 0 {
            return Ok(ExprFragment {
                code: vec![Ir::Const(Value::from(""))],
                foldable: true,
            });
        }

        let segment_exprs = segments
            .iter()
            .map(|segment| match segment {
                FormatSegment::Literal(str) => Ok(ExprFragment {
                    code: vec![Ir::Const(Value::from(str.clone()))],
                    foldable: true,
                }),
                FormatSegment::Interpolation(expr) => {
                    self.compile_expression(expr, Position::Inner)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let (segment_exprs, foldable) = self.fold_sibling_list(segment_exprs);

        Ok(ExprFragment {
            foldable,
            code: segment_exprs
                .into_iter()
                .flat_map(|segment| segment.code)
                .chain([Ir::Ready(Bytecode::Concat(
                    NonZeroUsize::try_from(segment_count).expect("0 segment case can't reach here"),
                ))])
                .collect(),
        })
    }
}
