mod assemble;
mod def;
mod globals;
mod locals;
mod prewalk;
mod simple_expressions;

use std::sync::Arc;

use crate::{CompilerError, CompilerErrors, CompilerOptions, CompilerOutput, OptimizationOptions};

use frost_parse::{
    ast::{Expr, Program, Spanned, Statement},
    parse_program,
};
use frost_runtime::{Arity, Bytecode, CompiledFunction, MapKey, Value};

use locals::Locals;

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
    locals: Locals,
    next_label: Label,
    name: String,
    arity: Arity,
    num_captures: usize,
    source: &'a str,
    filename: &'a str,
    options: &'a CompilerOptions,
}

impl<'a> FunctionBuilder<'a> {
    fn new(
        options: &'a CompilerOptions,
        name: String,
        filename: &'a str,
        source: &'a str,
        arity: Arity,
    ) -> Self {
        Self {
            locals: Locals::new(),
            next_label: Label(0),
            name,
            arity,
            num_captures: 0,
            source,
            filename,
            options,
        }
    }

    fn next_label(&mut self) -> Label {
        let label = self.next_label;
        self.next_label.0 += 1;
        label
    }

    /// Start a hard-error diagnostic already carrying this function's source.
    /// Routing every error through here keeps any from being emitted sourceless.
    fn error(&self, message: String) -> CompilerError {
        CompilerError::error(message).source(self.filename.to_owned(), self.source.to_owned())
    }
}

pub fn compile_program(
    filename: &str,
    script: &str,
    options: CompilerOptions,
) -> Result<CompilerOutput, CompilerErrors> {
    let ast = parse_program(filename, script)
        .map_err(|err| CompilerError::from_parse_error(&err, filename, script))?;

    let mut fn_builder = FunctionBuilder::new(
        &options,
        "<main>".to_string(),
        filename,
        script,
        Arity::Exact(0),
    );

    // The runtime starts by pushing the top-level function itself to the stack
    // Pop it
    let mut ir: Vec<Ir> = vec![Ir::Ready(Bytecode::Pop)];

    if let Some((tail, body)) = ast.statements.split_last() {
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

        match &stmt.node {
            Statement::Def {
                exported,
                destructure,
                expr,
            } => self.compile_def(expr, destructure, *exported),
            Statement::Expr(expr) => {
                let mut fragment = self.compile_expression(expr)?;
                fragment.code.push(Ir::Ready(Bytecode::Pop));
                Ok(fragment)
            }
        }
    }

    fn compile_expression(&mut self, expr: &Spanned<Expr>) -> Result<IrFragment, CompilerErrors> {
        match &expr.node {
            Expr::Literal(literal) => self.compile_literal(literal, expr.span),
            Expr::NameLookup(name) => self.compile_name_lookup(name, expr.span),
            Expr::BinOp { left, op, right } => todo!(),
            Expr::Logical { left, op, right } => todo!(),
            Expr::UnaryOp { op, operand } => todo!(),
            Expr::If {
                condition,
                consequent,
                alternate,
            } => todo!(),
            Expr::Do { body, value } => todo!(),
            Expr::Call { callee, args } => todo!(),
            Expr::SoftIndex { target, key } => todo!(),
            Expr::HardIndex { target, key } => todo!(),
            Expr::Array(spanneds) => todo!(),
            Expr::Map(spanneds) => todo!(),
            Expr::FormatString(format_segments) => todo!(),
            Expr::Lambda {
                params,
                variadic_param,
                self_name,
                body,
                return_expr,
            } => todo!(),
            Expr::AbbreviatedLambda {
                used_params,
                uses_rest,
                body,
            } => todo!(),
            Expr::Filter {
                structure,
                operation,
            } => todo!(),
            Expr::MapIter {
                structure,
                operation,
            } => todo!(),
            Expr::Reduce {
                structure,
                operation,
                init,
            } => todo!(),
            Expr::Foreach {
                structure,
                operation,
            } => todo!(),
            Expr::Match { target, arms } => todo!(),
        }
    }
}
