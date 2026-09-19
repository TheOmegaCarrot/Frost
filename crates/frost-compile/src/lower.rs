mod assemble;
mod binary_operations;
mod def;
mod fold;
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

use fold::FoldVm;
use locals::Locals;

#[derive(Clone, Debug)]
enum JumpType {
    Unconditional,
    IfTrue,
    IfFalse,
    PeekIfTrue,
    PeekIfFalse,
}

#[derive(Clone, Copy, Debug)]
struct Label(usize);

#[derive(Clone, Debug)]
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
struct ExprFragment {
    code: Vec<Ir>,
    foldable: bool,
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
    // The warm VM for constant-folding, shared across the whole compilation.
    // `None` when there is no folding (e.g. a test that only assembles).
    fold_vm: Option<&'a FoldVm>,
}

impl<'a> FunctionBuilder<'a> {
    fn new(
        options: &'a CompilerOptions,
        name: String,
        filename: &'a str,
        source: &'a str,
        fold_vm: Option<&'a FoldVm>,
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
            fold_vm,
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

    let fold_vm = FoldVm::new();
    let mut fn_builder = FunctionBuilder::new(
        &options,
        "<main>".to_string(),
        filename,
        script,
        Some(&fold_vm),
        Arity::Exact(0),
    );

    // The runtime starts by pushing the top-level function itself to the stack
    // Pop it
    let mut ir: Vec<Ir> = vec![Ir::Ready(Bytecode::Pop)];

    if let Some((tail, body)) = ast.statements.split_last() {
        for stmt in body {
            ir.extend(fn_builder.compile_statement(stmt, false)?);
        }
        // The final statement is in tail position: an expression there is the
        // program's result value, not dropped.
        ir.extend(fn_builder.compile_statement(tail, true)?);
    }

    let func = fn_builder.assemble(ir);

    Ok(CompilerOutput {
        code: func.assert_trusted(),
    })
}

impl FunctionBuilder<'_> {
    /// Compile a statement to its complete code. Net-zero, except a `tail`
    /// expression statement, which leaves its value (the block's result).
    fn compile_statement(
        &mut self,
        stmt: &Spanned<Statement>,
        tail: bool,
    ) -> Result<Vec<Ir>, CompilerErrors> {
        match &stmt.node {
            Statement::Def {
                exported,
                destructure,
                expr,
            } => self.compile_def(expr, destructure, *exported),
            // A bare expression is a fold point. In tail position its value is
            // kept; otherwise it is evaluated for effect and dropped.
            Statement::Expr(expr) => {
                let expr_fragment = self.compile_expression(expr)?;
                let mut folded = self.fold_if_eligible(expr_fragment);
                if !tail {
                    folded.code.push(Ir::Ready(Bytecode::Pop));
                }
                Ok(folded.code)
            }
        }
    }

    fn compile_expression(&mut self, expr: &Spanned<Expr>) -> Result<ExprFragment, CompilerErrors> {
        match &expr.node {
            Expr::Literal(literal) => self.compile_literal(literal, expr.span),
            Expr::NameLookup(name) => self.compile_name_lookup(name, expr.span),
            Expr::BinOp { left, op, right } => self.compile_binop(left, op, right),
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
