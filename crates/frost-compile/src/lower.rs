mod assemble;
mod binary_operations;
mod call_expression;
mod def;
mod destructure;
mod do_expression;
mod fold;
mod format_string;
mod globals;
mod if_expression;
mod index_expressions;
mod iterative_expressions;
mod lambda_expression;
mod locals;
mod match_expression;
mod passes;
mod prewalk;
mod simple_expressions;
mod structure_literals;

use std::sync::Arc;

use crate::{CompilerError, CompilerErrors, CompilerOptions, CompilerOutput, OptimizationOptions};

use frost_parse::{
    ast::{Expr, SourceSpan, Spanned, Statement},
    parse_program,
};
use frost_runtime::{Arity, Bytecode, CompiledFunction, MapKey, Value};

use fold::FoldVm;
use iterative_expressions::Iteration;
use locals::Locals;

/// A name as scope resolution sees it. `$` is shorthand for `$1`, so every use
/// of `$` resolves as `$1`.
fn canonical_name(name: &str) -> &str {
    if name == "$" { "$1" } else { name }
}

#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// A `LoadLocal` that moves the value out of its slot; see the
    /// `consume_locals` pass.
    ConsumeLocal(LocalId),
    DefLocal(LocalId),
    /// Create a closure over a nested function. Both forms are kept: `compiled`
    /// is what assembly pools, `lowered` is the IR it was assembled from.
    /// Build with [`Ir::closure`] so the two always match.
    Closure {
        lowered: Arc<LoweredFunction>,
        compiled: Arc<CompiledFunction>,
    },
}

impl Ir {
    /// The closure-creation op for a nested function, assembled once here.
    fn closure(lowered: LoweredFunction) -> Self {
        let compiled = lowered.assemble();
        Ir::Closure {
            lowered: Arc::new(lowered),
            compiled,
        }
    }
}

/// A function after lowering, before assembly: its IR and everything needed to
/// assemble, inspect, or rewrite it.
#[derive(Debug)]
struct LoweredFunction {
    name: String,
    arity: Arity,
    code: Vec<Ir>,
    locals: Locals,
    // Bounds the label ids `code` references (ids `< num_labels`).
    num_labels: usize,
    // See `FunctionBuilder::effectful`.
    effectful: bool,
}

#[derive(Debug)]
struct ExprFragment {
    code: Vec<Ir>,
    foldable: bool,
}

/// A compiled statement. Unlike an [`ExprFragment`] it leaves nothing on the
/// stack (bar a tail expression statement), so it is never folded on its own;
/// `foldable` only lets an enclosing block fold as a whole.
#[derive(Debug)]
struct StatementFragment {
    code: Vec<Ir>,
    foldable: bool,
}

/// Where an expression's code sits within its function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Position {
    /// Nothing in the function executes after this expression: its value is
    /// the function's result, so a call here is a tail call.
    Tail,
    /// Some code of the function executes after this expression.
    Inner,
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
    // Whether this function's body loads an impure global. Every effect starts
    // at one, so a function that loads none is safe to call during a fold.
    effectful: bool,
}

impl<'a> FunctionBuilder<'a> {
    /// A builder for a function nested in this one, with `captures` seated in
    /// order and `hoisted` captures built in. It shares this builder's source,
    /// options, and fold VM, but holds no borrow of this builder itself.
    fn child(
        &self,
        name: String,
        arity: Arity,
        captures: Vec<String>,
        hoisted: Vec<(String, Value)>,
    ) -> FunctionBuilder<'a> {
        FunctionBuilder {
            locals: Locals::with_captures(captures, hoisted),
            next_label: Label(0),
            name,
            arity,
            source: self.source,
            filename: self.filename,
            options: self.options,
            fold_vm: self.fold_vm,
            top_level: false,
            effectful: false,
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

    /// The error for binding `name` at `span` in a scope that already binds it
    /// at `original`.
    fn duplicate_binding(
        &self,
        name: &str,
        span: SourceSpan,
        original: SourceSpan,
    ) -> CompilerError {
        self.error(format!("`{name}` is already bound"))
            .code("duplicate binding".into())
            .label_primary(span, "redefined here".into())
            .related(
                CompilerError::advice(format!("`{name}` was first bound here"))
                    .label(original, "original binding".into()),
            )
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
/// Use this to run a fragment against an accumulated environment,
/// passing that environment's names, in any order, as `outer_scope`.
/// With an empty `outer_scope`, this is exactly [`compile_program`].
pub fn compile_in_scope(
    filename: &str,
    script: &str,
    options: CompilerOptions,
    outer_scope: &[&str],
) -> Result<CompilerOutput, CompilerErrors> {
    let ast = parse_program(filename, script)
        .map_err(|err| CompilerError::from_parse_error(&err, filename, script))?;

    // A free name the enclosing scope supplies becomes a capture. Only names
    // actually used are captured; any other free name is left to resolve as a
    // global (or to error).
    let captures = prewalk::free_names_of_program(&ast.statements)
        .into_iter()
        .filter(|name| outer_scope.contains(&name.as_str()));

    let fold_vm = FoldVm::new();
    let mut fn_builder = FunctionBuilder {
        // The host supplies a script's captures at runtime: none is known now.
        locals: Locals::with_captures(captures, []),
        next_label: Label(0),
        name: "<main>".to_string(),
        arity: Arity::Exact(0),
        source: script,
        filename,
        options: &options,
        fold_vm: Some(&fold_vm),
        top_level: true,
        effectful: false,
    };

    // Consume the top-level's own function value (see the calling convention on `Bytecode`).
    let mut ir: Vec<Ir> = vec![Ir::Ready(Bytecode::Pop)];

    if let Some((tail, body)) = ast.statements.split_last() {
        for stmt in body {
            ir.extend(fn_builder.compile_statement(stmt, Position::Inner)?.code);
        }
        ir.extend(fn_builder.compile_statement(tail, Position::Tail)?.code);
    }

    let func = fn_builder.finish(ir).assemble();

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

    /// Run `compile` in a fresh nested scope (a `do` block or a `match` arm),
    /// closing it afterward whether `compile` succeeds or fails.
    fn in_scope<T>(&mut self, compile: impl FnOnce(&mut Self) -> T) -> T {
        self.locals.enter();
        let result = compile(self);
        self.locals.exit();
        result
    }
}

impl FunctionBuilder<'_> {
    /// Compile a statement to its complete code. Net-zero, except an expression
    /// statement in [`Position::Tail`], which is its function's last statement
    /// and leaves its value as the result.
    fn compile_statement(
        &mut self,
        stmt: &Spanned<Statement>,
        position: Position,
    ) -> Result<StatementFragment, CompilerErrors> {
        match &stmt.node {
            Statement::Def {
                exported,
                destructure,
                expr,
            } => self.compile_def(expr, destructure, *exported),
            // A bare expression is a fold point. In tail position its value is
            // kept; otherwise it is evaluated for effect and dropped.
            Statement::Expr(expr) => {
                let expr_fragment = self.compile_expression(expr, position)?;
                let mut folded = self.fold_if_eligible(expr_fragment);
                if position == Position::Inner {
                    folded.code.push(Ir::Ready(Bytecode::Pop));
                }
                Ok(StatementFragment {
                    code: folded.code,
                    foldable: folded.foldable,
                })
            }
        }
    }

    /// Compile an expression to code leaving its value on the stack.
    ///
    /// A node may pass [`Position::Tail`] on to a child only if that child's
    /// code is the last the node executes; if the node emits anything after it,
    /// the child is [`Position::Inner`].
    fn compile_expression(
        &mut self,
        expr: &Spanned<Expr>,
        position: Position,
    ) -> Result<ExprFragment, CompilerErrors> {
        match &expr.node {
            Expr::Literal(literal) => self.compile_literal(literal, expr.span),
            Expr::NameLookup(name) => self.compile_name_lookup(name, expr.span),
            Expr::BinOp { left, op, right } => self.compile_binop(left, op, right),
            Expr::Logical { left, op, right } => self.compile_logical(left, op, right, position),
            Expr::UnaryOp { op, operand } => self.compile_unary(op, operand),
            Expr::If {
                condition,
                consequent,
                alternate,
            } => self.compile_if_expression(condition, consequent, alternate, position),
            Expr::Do { body, value } => self.compile_do_expression(body, value, position),
            Expr::Call { callee, args } => self.compile_call_expression(callee, args, position),
            Expr::SoftIndex { target, key } => self.compile_soft_index(target, key),
            Expr::HardIndex { target, key } => self.compile_hard_index(target, key),
            Expr::Array(elements) => self.compile_array_literal(elements),
            Expr::Map(entries) => self.compile_map_literal(entries),
            Expr::FormatString(segments) => self.compile_format_string(segments),
            Expr::Lambda { .. } | Expr::AbbreviatedLambda { .. } => self.compile_lambda(expr),
            Expr::Filter {
                structure,
                operation,
            } => self.compile_iteration(Iteration::Filter, structure, operation, None, position),
            Expr::MapIter {
                structure,
                operation,
            } => self.compile_iteration(Iteration::Map, structure, operation, None, position),
            Expr::Reduce {
                structure,
                operation,
                init,
            } => self.compile_iteration(
                Iteration::Reduce,
                structure,
                operation,
                init.as_deref(),
                position,
            ),
            Expr::Foreach {
                structure,
                operation,
            } => self.compile_iteration(Iteration::Foreach, structure, operation, None, position),
            Expr::Match { target, arms } => self.compile_match_expression(target, arms, position),
        }
    }
}
