//! Capture discovery: the pre-walk that finds a lambda's free names before any
//! opcode is emitted.
//!
//! It replicates the C++ oracle's `symbol_sequence()` analysis using the same
//! [`Locals`] resolver the codegen uses: walk the body in evaluation order,
//! `define` names as bindings introduce them, and treat any `resolve` miss that
//! is not a global as a capture. Scoping falls out of `Locals::enter`/`exit`, so
//! a `do` block absorbs its own definitions; a nested lambda is opaque, so its
//! own free names are replayed here as usages (a name the inner lambda needs
//! that this scope cannot supply becomes a capture of this scope too).
//!
//! The discovery order is evaluation order, which fixes the arbitrary-but-
//! consistent slot order of the captures. A usage of a name *before* its
//! definition in the same scope is a capture (a later definition shadows only
//! subsequent uses); this is deliberate and matches the oracle.

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use frost_parse::ast::{
    Binding, Destructure, Expr, FormatSegment, MatchArm, MatchPattern, SourceSpan, Spanned,
    Statement,
};

use super::globals::global_slot;
use super::locals::Locals;

/// The free names a lambda expression captures, in evaluation order, found with
/// a fresh scope seeded with the lambda's own parameters.
///
/// `lambda` must be an [`Expr::Lambda`] or [`Expr::AbbreviatedLambda`].
pub(super) fn find_captures(lambda: &Expr) -> Vec<String> {
    let mut scan = Scanner::new();
    match lambda {
        Expr::Lambda {
            params,
            variadic_param,
            self_name,
            body,
            return_expr,
        } => {
            for param in params {
                scan.define_binding(&param.node);
            }
            if let Some(variadic) = variadic_param {
                scan.define_binding(&variadic.node);
            }
            // The self-name is an internal definition (recursion), not a capture.
            if let Some(name) = self_name {
                scan.define(&name.node);
            }
            scan.block(body, return_expr);
        }
        Expr::AbbreviatedLambda {
            used_params,
            uses_rest,
            body,
        } => {
            // The implicit parameters are `$1..$n`, with `$` a runtime alias for
            // `$1` and `$$` the rest parameter.
            for i in 0..used_params.len() {
                scan.define(&format!("${}", i + 1));
            }
            if !used_params.is_empty() {
                scan.define("$");
            }
            if *uses_rest {
                scan.define("$$");
            }
            scan.expr(body);
        }
        _ => unreachable!("find_captures called on a non-lambda expression"),
    }
    scan.captures
}

/// Walks an expression tree in evaluation order, tracking in-scope names and
/// collecting the free ones.
struct Scanner {
    scope: Locals,
    captures: Vec<String>,
    seen: BTreeSet<String>,
}

impl Scanner {
    fn new() -> Self {
        Self {
            scope: Locals::new(),
            captures: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    /// Record a use of `name`: a capture if it is neither in scope nor a global.
    fn usage(&mut self, name: &str) {
        if self.scope.resolve(name).is_none()
            && global_slot(name).is_none()
            && self.seen.insert(name.to_owned())
        {
            self.captures.push(name.to_owned());
        }
    }

    /// Introduce `name` into the current scope. Duplicate-binding errors are the
    /// codegen pass's concern; capture discovery only needs the name in scope.
    fn define(&mut self, name: &str) {
        let _ = self.scope.define(name.to_owned(), SourceSpan::default(), false);
    }

    fn define_binding(&mut self, binding: &Binding) {
        if let Binding::Named(name) = binding {
            self.define(name);
        }
    }

    /// A statement sequence followed by a trailing value expression, in the
    /// current scope (used for a lambda body; `do` blocks add their own scope).
    fn block(&mut self, body: &[Spanned<Statement>], tail: &Spanned<Expr>) {
        for statement in body {
            self.statement(statement);
        }
        self.expr(tail);
    }

    fn statement(&mut self, statement: &Spanned<Statement>) {
        match &statement.node {
            // The right-hand side is evaluated before the binding exists, so a
            // use of the bound name in the rhs is free.
            Statement::Def {
                destructure, expr, ..
            } => {
                self.expr(expr);
                self.destructure(destructure);
            }
            Statement::Expr(expr) => self.expr(expr),
        }
    }

    fn expr(&mut self, expr: &Spanned<Expr>) {
        match &expr.node {
            Expr::Literal(_) => {}
            Expr::NameLookup(name) => self.usage(name),
            Expr::BinOp { left, right, .. } | Expr::Logical { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expr::UnaryOp { operand, .. } => self.expr(operand),
            Expr::If {
                condition,
                consequent,
                alternate,
            } => {
                self.expr(condition);
                self.expr(consequent);
                if let Some(alternate) = alternate {
                    self.expr(alternate);
                }
            }
            Expr::Do { body, value } => {
                self.scope.enter();
                self.block(body, value);
                self.scope.exit();
            }
            Expr::Call { callee, args } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
            }
            Expr::SoftIndex { target, key } => {
                self.expr(target);
                self.expr(key);
            }
            // The key of a hard index is a literal field name, not a usage.
            Expr::HardIndex { target, .. } => self.expr(target),
            Expr::Array(elements) => {
                for element in elements {
                    self.expr(element);
                }
            }
            Expr::Map(entries) => {
                for entry in entries {
                    self.expr(&entry.node.key);
                    self.expr(&entry.node.value);
                }
            }
            Expr::FormatString(segments) => {
                for segment in segments {
                    if let FormatSegment::Interpolation(inner) = segment {
                        self.expr(inner);
                    }
                }
            }
            // A nested lambda is opaque: its free names are what it demands of
            // this scope, so replay each as a usage here.
            Expr::Lambda { .. } | Expr::AbbreviatedLambda { .. } => {
                for name in find_captures(&expr.node) {
                    self.usage(&name);
                }
            }
            Expr::Filter {
                structure,
                operation,
            }
            | Expr::MapIter {
                structure,
                operation,
            }
            | Expr::Foreach {
                structure,
                operation,
            } => {
                self.expr(structure);
                self.expr(operation);
            }
            Expr::Reduce {
                structure,
                operation,
                init,
            } => {
                self.expr(structure);
                self.expr(operation);
                if let Some(init) = init {
                    self.expr(init);
                }
            }
            Expr::Match { target, arms } => {
                self.expr(target);
                for arm in arms {
                    self.match_arm(&arm.node);
                }
            }
        }
    }

    /// Each arm is its own scope: the pattern's bindings are visible to the
    /// guard and the result, then discarded before the next arm.
    fn match_arm(&mut self, arm: &MatchArm) {
        self.scope.enter();
        self.pattern(&arm.pattern);
        if let Some(guard) = &arm.guard {
            self.expr(guard);
        }
        self.expr(&arm.result);
        self.scope.exit();
    }

    fn pattern(&mut self, pattern: &Spanned<MatchPattern>) {
        match &pattern.node {
            MatchPattern::Binding { name, .. } => self.define_binding(&name.node),
            // A value pattern compares against an expression, e.g. `(existing)`.
            MatchPattern::Value(expr) => self.expr(expr),
            MatchPattern::Array { elements, rest } => {
                for element in elements {
                    self.pattern(element);
                }
                if let Some(rest) = rest {
                    self.define_binding(&rest.node);
                }
            }
            MatchPattern::Map {
                entries,
                bind_whole,
            } => {
                for entry in entries {
                    self.expr(&entry.node.key);
                    self.pattern(&entry.node.pattern);
                }
                if let Some(whole) = bind_whole {
                    self.define_binding(&whole.node);
                }
            }
            // Every alternative binds the same names; walking all is safe since
            // re-defining a name in scope is a no-op here.
            MatchPattern::Alternative(branches) => {
                for branch in branches {
                    self.pattern(branch);
                }
            }
        }
    }

    fn destructure(&mut self, destructure: &Spanned<Destructure>) {
        match &destructure.node {
            Destructure::Binding(binding) => self.define_binding(&binding.node),
            Destructure::Array { elements, rest } => {
                for element in elements {
                    self.destructure(element);
                }
                if let Some(rest) = rest {
                    self.define_binding(&rest.node);
                }
            }
            Destructure::Map {
                entries,
                bind_whole,
            } => {
                for entry in entries {
                    self.expr(&entry.node.key);
                    self.destructure(&entry.node.destructure);
                }
                if let Some(whole) = bind_whole {
                    self.define_binding(&whole.node);
                }
            }
        }
    }
}
