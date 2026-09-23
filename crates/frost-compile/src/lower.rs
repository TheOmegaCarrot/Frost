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
    ast::{Expr, Program, SourceSpan, Spanned, Statement},
    parse_program,
};
use frost_runtime::{Arity, Bytecode, CompiledFunction, MapKey, Value};

use fold::FoldVm;
use locals::{LocalInfo, LocalKind, Locals};

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

/// A local's identity within a function, assigned when it is defined. Concrete
/// slots are assigned at assembly, so the IR refers to locals by id: a later
/// pass can add or drop a local without renumbering the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LocalId(usize);

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
    LoadLocal(LocalId),
    DefLocal(LocalId),
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
    source: &'a str,
    filename: &'a str,
    options: &'a CompilerOptions,
    // The warm VM for constant-folding, shared across the whole compilation.
    // `None` when there is no folding (e.g. a test that only assembles).
    fold_vm: Option<&'a FoldVm>,
    // True only for a script's top-level function. Gates implicit export, which
    // applies to top-level bindings, not those inside a nested function.
    top_level: bool,
}

impl FunctionBuilder<'_> {
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

/// Compile a standalone script: a top-level with no enclosing scope, so every
/// free name must resolve to a global or is an error.
pub fn compile_program(
    filename: &str,
    script: &str,
    options: CompilerOptions,
) -> Result<CompilerOutput, CompilerErrors> {
    compile_in_scope(filename, script, options, &[])
}

/// Compile a top-level nested within an enclosing scope: a free name found in
/// `outer_scope` becomes a capture, and its value is supplied at
/// [`close`](frost_runtime::TrustedProgram::close).
/// This is what an embedder uses to run a fragment
/// against an accumulated environment; `outer_scope` carries that environment's
/// names, and the empty slice is exactly [`compile_program`].
/// The order of names is irrelevant.
pub fn compile_in_scope(
    filename: &str,
    script: &str,
    options: CompilerOptions,
    outer_scope: &[&str],
) -> Result<CompilerOutput, CompilerErrors> {
    let ast = parse_program(filename, script)
        .map_err(|err| CompilerError::from_parse_error(&err, filename, script))?;

    let fold_vm = FoldVm::new();
    let mut fn_builder = FunctionBuilder {
        locals: Locals::new(),
        next_label: Label(0),
        name: "<main>".to_string(),
        arity: Arity::Exact(0),
        source: script,
        filename,
        options: &options,
        fold_vm: Some(&fold_vm),
        top_level: true,
    };
    fn_builder.seed_captures(&ast, outer_scope);

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
    /// Whether a binding compiled now is implicitly exported: the option is on,
    /// and this is a top-level binding of the top-level function (not one inside
    /// a nested scope or a lambda).
    fn exports_implicitly(&self) -> bool {
        self.top_level && self.options.implicit_export && self.locals.at_top_scope()
    }

    /// Reserve a capture for each free name of the program that the enclosing
    /// scope supplies. Only names actually used are captured; a free name absent
    /// from `outer_scope` is left to resolve as a global (or to error).
    fn seed_captures(&mut self, ast: &Program, outer_scope: &[&str]) {
        for name in prewalk::free_names_of_program(&ast.statements) {
            if outer_scope.contains(&name.as_str()) {
                self.locals
                    .define(LocalInfo {
                        name,
                        span: SourceSpan::default(),
                        exported: false,
                        constant: None,
                        kind: LocalKind::Capture,
                    })
                    .expect("free names are distinct, so no capture collides");
            }
        }
    }
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
            Expr::Logical { left, op, right } => self.compile_logical(left, op, right),
            Expr::UnaryOp { op, operand } => self.compile_unary(op, operand),
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
