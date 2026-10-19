mod assemble;
mod binary_operations;
mod call_expression;
mod def;
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
mod pattern;
mod prewalk;
mod simple_expressions;
mod structure_literals;

use std::ffi::OsStr;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::compile::{
    CompilerOptions, CompilerOutput, Diagnostic, Diagnostics, OptimizationOptions,
};

use crate::bytecode::{Bytecode, CompiledFunction, SourceLines, Span};
use crate::{Arity, MapKey, Value};
use frostlang_parse::{
    ast::{Expr, SourceSpan, Spanned, Statement, TypeConstraint},
    parse_program_with_limits,
};

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

/// An operation on a Map with a key known at compile time, which assembly
/// places in the function's key constants.
#[derive(Clone, Copy, Debug)]
enum ConstKeyOp {
    /// `HardIndexMap`: `( m -- v )`.
    HardIndex,
    /// `TestConstKey`: `( m -- m b )`.
    Test,
    /// `ExtractConstKey`: `( m -- m v )`.
    Extract,
}

/// A local's identity within a function, assigned when it is defined. Concrete
/// slots are assigned at assembly, so the IR refers to locals by id: a later
/// pass can add or drop a local without renumbering the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct LocalId(usize);

/// One IR instruction, and the source it is derived from: the innermost AST node
/// that unambiguously produced it.
#[derive(Clone, Debug)]
struct Ir {
    op: Op,
    span: Span,
}

/// What an IR instruction does. An [`Op::Label`] is zero-width: it emits
/// nothing, so its span only widens a fragment's [`code_span`].
#[derive(Clone, Debug)]
enum Op {
    Ready(Bytecode),
    Jump {
        kind: JumpType,
        label: Label,
    },
    Label(Label),
    Const(Value),
    ConstKey {
        op: ConstKeyOp,
        key: MapKey,
    },
    LoadLocal(LocalId),
    /// A `LoadLocal` that moves the value out of its slot; see the
    /// `consume_locals` pass.
    ConsumeLocal(LocalId),
    DefLocal(LocalId),
    /// Create a closure over a nested function. Both forms are kept: `compiled`
    /// is what assembly pools, `lowered` is the IR it was assembled from.
    /// Build with [`Op::closure`] so the two always match.
    Closure {
        lowered: Arc<LoweredFunction>,
        compiled: Arc<CompiledFunction>,
    },
}

impl Op {
    /// The closure-creation op for a nested function, assembled once here.
    fn closure(lowered: LoweredFunction) -> Self {
        let compiled = lowered.assemble();
        Op::Closure {
            lowered: Arc::new(lowered),
            compiled,
        }
    }

    /// This op, derived from the source at `span`.
    fn at(self, span: Span) -> Ir {
        Ir { op: self, span }
    }

    /// This op, for a test whose IR has no source. Unlike [`UNEMITTED`] IR, a
    /// test's IR is emitted, just without debug info.
    #[cfg(test)]
    fn sourceless(self) -> Ir {
        self.at(Span { start: 0, end: 0 })
    }
}

/// The debug-info span of a source span. Offsets past `u32` saturate: debug info
/// is inexact past the first 4 GiB of source.
fn debug_span(span: SourceSpan) -> Span {
    let offset = |offset: usize| u32::try_from(offset).unwrap_or(u32::MAX);
    Span {
        start: offset(span.start),
        end: offset(span.end),
    }
}

/// The span of IR that is evaluated at compile time but never emitted, so has
/// no debug info to read it.
const UNEMITTED: Span = Span { start: 0, end: 0 };

/// The filled union of the spans of `code`, or `None` if it is empty.
fn code_span(code: &[Ir]) -> Option<Span> {
    code.iter().map(|ir| ir.span).reduce(Span::filled_union)
}

/// A function after lowering, before assembly: its IR and everything needed to
/// assemble, inspect, or rewrite it.
#[derive(Debug)]
struct LoweredFunction {
    name: String,
    origin: Option<Arc<str>>,
    arity: Arity,
    code: Vec<Ir>,
    locals: Locals,
    // Bounds the label ids `code` references (ids `< num_labels`).
    num_labels: usize,
    // See `FunctionBuilder::effectful`.
    effectful: bool,
    // See `OptimizationOptions::deduplicate_constants`.
    deduplicate_constants: bool,
    // The compilation's line table, if it keeps debug info.
    debug_lines: Option<Arc<SourceLines>>,
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

/// The shared source of a test's builder, whose `source` is always empty.
#[cfg(test)]
static NO_SOURCE: OnceLock<Arc<str>> = OnceLock::new();

#[derive(Debug)]
struct FunctionBuilder<'a> {
    locals: Locals,
    next_label: Label,
    name: String,
    arity: Arity,
    source: &'a str,
    // `source` as the compilation's diagnostics share it, made at the first
    // diagnostic so that a compilation with none never copies the source.
    shared_source: &'a OnceLock<Arc<str>>,
    filename: &'a str,
    // Every function of one compilation shares its origin.
    origin: Option<Arc<str>>,
    // Every function of one compilation shares its line table: `Some` if the
    // compilation keeps debug info.
    debug_lines: Option<Arc<SourceLines>>,
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
    // The lints found in this function so far, nested functions' included.
    lints: Vec<Diagnostic>,
    // If this function is an abbreviated lambda not yet linted for holding a
    // lambda, its span.
    abbreviated: Option<SourceSpan>,
}

impl<'a> FunctionBuilder<'a> {
    /// A builder for a function nested in this one, with `captures` seated in
    /// order and `hoisted` captures built in (see [`Locals::with_captures`]). It
    /// shares this builder's source, origin, options, and fold VM, but holds no
    /// borrow of this builder itself.
    fn child(
        &self,
        name: String,
        arity: Arity,
        captures: Vec<(String, Option<Value>)>,
        hoisted: Vec<(String, Value)>,
    ) -> FunctionBuilder<'a> {
        FunctionBuilder {
            locals: Locals::with_captures(captures, hoisted),
            next_label: Label(0),
            name,
            arity,
            source: self.source,
            shared_source: self.shared_source,
            filename: self.filename,
            origin: self.origin.clone(),
            debug_lines: self.debug_lines.clone(),
            options: self.options,
            fold_vm: self.fold_vm,
            top_level: false,
            effectful: false,
            lints: Vec::new(),
            abbreviated: None,
        }
    }

    fn next_label(&mut self) -> Label {
        let label = self.next_label;
        self.next_label.0 += 1;
        label
    }

    /// Give `diagnostic` this compilation's source, which every diagnostic of
    /// the compilation shares, copied from `source` at the first.
    fn with_source(&self, diagnostic: Diagnostic) -> Diagnostic {
        let shared = self.shared_source.get_or_init(|| Arc::from(self.source));
        diagnostic.source(self.filename.to_owned(), Arc::clone(shared))
    }

    /// Start a hard-error diagnostic already carrying this function's source.
    /// Routing every error through here keeps any from being emitted sourceless.
    fn error(&self, message: String) -> Diagnostic {
        self.with_source(Diagnostic::error(message))
    }

    /// Whether to look for lints. A check costs nothing when this is false, so
    /// every check is behind it.
    fn lints_on(&self) -> bool {
        self.options.lints
    }

    /// Start a warning for the lint `code`, already carrying this function's
    /// source, for [`lint`](Self::lint) to report.
    fn warning(&self, code: &str, message: String) -> Diagnostic {
        self.with_source(Diagnostic::warning(message).code(code.to_owned()))
    }

    /// Start advice for the lint `code`, already carrying this function's
    /// source, for [`lint`](Self::lint) to report.
    fn advice(&self, code: &str, message: String) -> Diagnostic {
        self.with_source(Diagnostic::advice(message).code(code.to_owned()))
    }

    /// Report `lint`. Unlike an error, it does not stop compilation.
    fn lint(&mut self, lint: Diagnostic) {
        self.lints.push(lint);
    }

    /// Lint each binding this function never reads.
    fn lint_unused_bindings(&mut self) {
        let unused: Vec<(String, SourceSpan)> = self
            .locals
            .unused_bindings()
            .map(|info| (info.name.clone(), info.span))
            .collect();
        for (name, span) in unused {
            let lint = self
                .warning("unused binding", format!("`{name}` is never used"))
                .label_primary(span, "bound here".into())
                .help("remove it, or bind `_` instead".into());
            self.lint(lint);
        }
    }

    /// Lint `name`, bound at `span`, if it is a type's name: in a match pattern,
    /// it binds every value rather than testing the type.
    fn lint_type_name_binding(&mut self, name: &str, span: SourceSpan) {
        if !self.lints_on() || TypeConstraint::from_name(name).is_none() {
            return;
        }
        let lint = self
            .warning(
                "type name binding",
                format!("`{name}` is a type name, but here it binds a name"),
            )
            .label_primary(span, "binds a name".into())
            .help(format!(
                "to test for the type in a match pattern, write `_ is {name}`; \
                 otherwise, choose another name"
            ));
        self.lint(lint);
    }

    /// Lint the `-` operator at `op` if it starts a run of `-` with no space
    /// between, which reads as another language's decrement.
    fn lint_consecutive_minus(&mut self, op: SourceSpan) {
        if !self.lints_on() {
            return;
        }
        // Only an operator writes a `-`: no other token ends with one, and none
        // but an operator starts with one.
        let starts_run = !self.source[..op.start].ends_with('-');
        let run = self.source[op.start..]
            .bytes()
            .take_while(|&byte| byte == b'-')
            .count();
        if !starts_run || run < 2 {
            return;
        }
        let lint = self
            .warning(
                "consecutive minus",
                "Frost has no `--` decrement; these are separate `-` operators".to_owned(),
            )
            .label_primary(
                SourceSpan::from(op.start..op.start + run),
                "separate `-` operators".into(),
            )
            .help(
                "for a double negation or subtracting a negation, \
                 put a space between them, as in `- -x` or `a - -b`"
                    .into(),
            );
        self.lint(lint);
    }

    /// The error for binding `name` at `span` in a scope that already binds it
    /// at `original`.
    fn duplicate_binding(&self, name: &str, span: SourceSpan, original: SourceSpan) -> Diagnostic {
        self.error(format!("`{name}` is already bound"))
            .code("duplicate binding".into())
            .label_primary(span, "redefined here".into())
            .related(
                Diagnostic::advice(format!("`{name}` was first bound here"))
                    .label(original, "original binding".into()),
            )
    }
}

/// Compile a standalone script: a top-level with no enclosing scope, so every
/// free name must resolve to a global or is an error.
///
/// `filename` names the script in diagnostics, and its base name is the
/// [`origin`](crate::BacktraceFrame::origin) of every function compiled from it.
pub fn compile_program(
    filename: &str,
    script: &str,
    options: CompilerOptions,
) -> Result<CompilerOutput, Diagnostics> {
    compile_in_scope(filename, script, options, &[])
}

/// Compile a top-level nested within an enclosing scope: a free name found in
/// `outer_scope` becomes a capture, and its value is supplied at
/// [`close`](crate::WellFormedProgram::close).
/// Use this to run a fragment against an accumulated environment,
/// passing that environment's names, in any order, as `outer_scope`.
/// With an empty `outer_scope`, this is exactly [`compile_program`].
pub fn compile_in_scope(
    filename: &str,
    script: &str,
    options: CompilerOptions,
    outer_scope: &[&str],
) -> Result<CompilerOutput, Diagnostics> {
    let ast = parse_program_with_limits(filename, script, options.parse_limits)
        .map_err(|err| Diagnostic::from_parse_error(&err, filename, script))?;

    // A free name the enclosing scope supplies becomes a capture. Only names
    // actually used are captured; any other free name is left to resolve as a
    // global (or to error).
    // The host supplies their values at runtime: none is known now.
    let captures = prewalk::free_names_of_program(&ast.statements)
        .into_iter()
        .filter(|name| outer_scope.contains(&name.as_str()))
        .map(|name| (name, None));

    let fold_vm = FoldVm::new();
    let shared_source = OnceLock::new();
    let mut fn_builder = FunctionBuilder {
        locals: Locals::with_captures(captures, []),
        next_label: Label(0),
        name: "<main>".to_string(),
        arity: Arity::Exact(0),
        source: script,
        shared_source: &shared_source,
        filename,
        origin: Path::new(filename)
            .file_name()
            .and_then(OsStr::to_str)
            .map(Arc::from),
        debug_lines: options
            .debug_info
            .then(|| Arc::new(SourceLines::new(script))),
        options: &options,
        fold_vm: Some(&fold_vm),
        top_level: true,
        effectful: false,
        lints: Vec::new(),
        abbreviated: None,
    };

    // Consume the top-level's own function value (see the calling convention on
    // `Bytecode`), which the whole script is.
    let script_span = debug_span(SourceSpan::from(0..script.len()));
    let mut ir: Vec<Ir> = vec![Op::Ready(Bytecode::Pop).at(script_span)];

    if let Some((tail, body)) = ast.statements.split_last() {
        for stmt in body {
            ir.extend(fn_builder.compile_statement(stmt, Position::Inner)?.code);
        }
        ir.extend(fn_builder.compile_statement(tail, Position::Tail)?.code);
    }

    let (lowered, mut lints) = fn_builder.finish(ir);
    // Each function's lints come after its nested functions', so put them back
    // in source order.
    lints.sort_by_key(Diagnostic::offset);

    Ok(CompilerOutput {
        code: lowered.assemble().assert_well_formed(),
        lints: Diagnostics::of(lints),
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
    ) -> Result<StatementFragment, Diagnostics> {
        match &stmt.node {
            Statement::Def {
                exported,
                pattern,
                expr,
            } => self.compile_def(expr, pattern, *exported),
            // A bare expression is a fold point. In tail position its value is
            // kept; otherwise it is evaluated for effect and dropped.
            Statement::Expr(expr) => {
                let expr_fragment = self.compile_expression(expr, position)?;
                let mut folded = self.fold_if_eligible(expr_fragment);
                if position == Position::Inner {
                    folded
                        .code
                        .push(Op::Ready(Bytecode::Pop).at(debug_span(stmt.span)));
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
    ) -> Result<ExprFragment, Diagnostics> {
        // The span of the code this node emits itself; its children's code
        // carries their own.
        let span = debug_span(expr.span);
        match &expr.node {
            Expr::Literal(literal) => self.compile_literal(literal, expr.span),
            Expr::NameLookup(name) => self.compile_name_lookup(name, expr.span),
            Expr::BinOp { left, op, right } => self.compile_binop(left, op, right, span),
            Expr::Logical { left, op, right } => {
                self.compile_logical(left, op, right, position, span)
            }
            Expr::UnaryOp { op, operand } => self.compile_unary(op, operand, span),
            Expr::If {
                condition,
                consequent,
                alternate,
            } => self.compile_if_expression(condition, consequent, alternate, position, span),
            Expr::Do { body, value } => self.compile_do_expression(body, value, position),
            Expr::Call { callee, args } => {
                self.compile_call_expression(callee, args, position, span)
            }
            Expr::SoftIndex { target, key } => self.compile_soft_index(target, key, span),
            Expr::HardIndex { target, key } => self.compile_hard_index(target, key, span),
            Expr::Array(elements) => self.compile_array_literal(elements, span),
            Expr::Map(entries) => self.compile_map_literal(entries, span),
            Expr::FormatString(segments) => self.compile_format_string(segments, span),
            Expr::Lambda { .. } | Expr::AbbreviatedLambda { .. } => self.compile_lambda(expr),
            Expr::Filter {
                structure,
                operation,
            } => self.compile_iteration(
                Iteration::Filter,
                structure,
                operation,
                None,
                position,
                span,
            ),
            Expr::MapIter {
                structure,
                operation,
            } => self.compile_iteration(Iteration::Map, structure, operation, None, position, span),
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
                span,
            ),
            Expr::Foreach {
                structure,
                operation,
            } => self.compile_iteration(
                Iteration::Foreach,
                structure,
                operation,
                None,
                position,
                span,
            ),
            Expr::Match { target, arms } => {
                self.compile_match_expression(target, arms, position, span)
            }
        }
    }
}
