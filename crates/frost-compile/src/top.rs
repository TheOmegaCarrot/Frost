mod assemble;

use std::sync::Arc;

use crate::{CompilerErrors, CompilerOptions, CompilerOutput, OptimizationOptions};

use frost_parse::ast::{Expr, Program, Spanned, Statement};
use frost_runtime::{Arity, Bytecode, CompiledFunction, MapKey, NameEntry, Value};

#[derive(Debug)]
enum JumpType {
    Unconditional,
    IfTrue,
    IfFalse,
    PeekIfTrue,
    PeekIfFalse,
}

#[derive(Clone, Copy, Debug)]
struct Label(usize);

#[derive(Debug)]
enum Ir {
    Ready(Bytecode),
    Jump {
        kind: JumpType,
        label: Label,
    },
    Label(Label),
    Const(Value),
    KeyIndex(MapKey),
    Closure {
        function: Arc<CompiledFunction>,
        num_captures: u32,
    },
}

#[derive(Debug)]
struct IrFragment {
    code: Vec<Ir>,
}

#[derive(Debug)]
struct FunctionBuilder<'a> {
    name_table: Vec<NameEntry>,
    next_label: Label,
    name: String,
    arity: Arity,
    num_captures: usize,
    options: &'a CompilerOptions,
}

impl<'a> FunctionBuilder<'a> {
    fn new(options: &'a CompilerOptions, name: String, arity: Arity) -> Self {
        Self {
            name_table: Vec::new(),
            next_label: Label(0),
            name,
            arity,
            num_captures: 0,
            options,
        }
    }

    fn next_label(&mut self) -> Label {
        let label = self.next_label;
        self.next_label.0 += 1;
        label
    }
}

pub fn compile_program(
    filename: &str,
    script: Program,
    options: CompilerOptions,
) -> Result<CompilerOutput, CompilerErrors> {
    let mut fn_builder = FunctionBuilder::new(&options, "<main>".to_string(), Arity::Exact(0));

    let mut ir: Vec<Ir> = Vec::new();

    if let Some((tail, body)) = script.statements.split_last() {
        for stmt in body {
            ir.extend(fn_builder.compile_statement(stmt)?.code);
        }

        match &tail.node {
            Statement::Expr(expr) => ir.extend(fn_builder.compile_expression(expr)?.code),
            Statement::Def { .. } => ir.extend(fn_builder.compile_statement(tail)?.code),
        }
    }

    let func = fn_builder.assemble(ir);

    Ok(CompilerOutput {
        code: func.assert_trusted(),
    })
}

impl FunctionBuilder<'_> {
    // IrFragment's contract: net zero stack effect
    fn compile_statement(
        &mut self,
        stmt: &Spanned<Statement>,
    ) -> Result<IrFragment, CompilerErrors> {
        // match and dispatch

        todo!()
    }

    fn compile_expression(&mut self, expr: &Spanned<Expr>) -> Result<IrFragment, CompilerErrors> {
        todo!()
    }
}
