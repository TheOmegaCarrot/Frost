//! `match`: try a value against each arm's pattern and guard in turn, and
//! evaluate the result of the first arm that matches.
//!
//! The target stays on the stack while the arms are tried. An arm marks the
//! stack, then tests a copy of the target. A mismatch anywhere jumps to the
//! arm's failure label, which rewinds the stack to the mark, dropping whatever
//! the pattern had laid out, and moves on to the next arm; the locals an
//! abandoned arm bound are never read. A matched arm drops its mark and the
//! target before its result, so the result is the last code its path runs.

use std::num::NonZeroU64;

use crate::Value;
use crate::bytecode::{Bytecode, Span};
use frostlang_parse::ast::{Expr, MatchArm, Pattern, Spanned};

use crate::compile::{
    Diagnostic, Diagnostics,
    lower::{
        ExprFragment, FunctionBuilder, Ir, JumpType, Label, Op, Position, debug_span,
        pattern::{BindMode, Mismatch, PatternFragment},
    },
};

/// The message raised when no arm matches, followed by the value.
const NO_MATCH: &str = "No match arm matches the value: ";

/// A compiled arm, the label its mismatches jump to, and the arm's span, which
/// trying the arm is derived from.
struct ArmFragment {
    fail: Label,
    pattern: PatternFragment,
    guard: Option<Guard>,
    result: ExprFragment,
    span: Span,
}

/// A compiled guard, and its span, which its check is derived from.
struct Guard {
    fragment: ExprFragment,
    span: Span,
}

impl ArmFragment {
    fn foldable(&self) -> bool {
        self.pattern.foldable
            && self
                .guard
                .as_ref()
                .is_none_or(|guard| guard.fragment.foldable)
            && self.result.foldable
    }

    /// `( t -- r )` on to the end of the match if the arm matches, else
    /// `( t -- t )` on to the next arm.
    fn code(self, end: Label) -> impl Iterator<Item = Ir> {
        let Self {
            fail,
            pattern,
            guard,
            result,
            span,
        } = self;
        let guard = guard.into_iter().flat_map(move |guard| {
            guard.fragment.code.into_iter().chain([Op::Jump {
                kind: JumpType::IfFalse,
                label: fail,
            }
            .at(guard.span)])
        });
        [Bytecode::MarkStack, Bytecode::Dup]
            .map(|op| Op::Ready(op).at(span))
            .into_iter()
            .chain(pattern.code)
            .chain(guard)
            .chain([Bytecode::DropMark, Bytecode::Pop].map(|op| Op::Ready(op).at(span)))
            .chain(result.code)
            .chain([
                Op::Jump {
                    kind: JumpType::Unconditional,
                    label: end,
                }
                .at(span),
                Op::Label(fail).at(span),
                Op::Ready(Bytecode::RewindToMark).at(span),
            ])
    }
}

impl FunctionBuilder<'_> {
    pub(super) fn compile_match_expression(
        &mut self,
        target: &Spanned<Expr>,
        arms: &[Spanned<MatchArm>],
        position: Position,
        span: Span,
    ) -> Result<ExprFragment, Diagnostics> {
        let target = self.compile_expression(target, Position::Inner)?;
        if self.lints_on() {
            self.lint_unreachable_arms(arms);
        }
        let arms = arms
            .iter()
            .map(|arm| self.in_scope(|this| this.compile_arm(arm, position)))
            .collect::<Result<Vec<_>, _>>()?;

        // TODO: with a constant target, branch elimination could pick the arm at
        // compile time.
        let foldable = target.foldable && arms.iter().all(ArmFragment::foldable);
        // The folding rule (see the `fold` module doc), over the target and each
        // arm's guard and result. A pattern's own expressions fold as it compiles.
        let (target, arms) = if foldable {
            (target, arms)
        } else {
            let arms = arms
                .into_iter()
                .map(|arm| ArmFragment {
                    guard: arm.guard.map(|guard| Guard {
                        fragment: self.fold_if_eligible(guard.fragment),
                        ..guard
                    }),
                    result: self.fold_if_eligible(arm.result),
                    ..arm
                })
                .collect();
            (self.fold_if_eligible(target), arms)
        };

        let end = self.next_label();
        Ok(ExprFragment {
            foldable,
            code: target
                .code
                .into_iter()
                .chain(arms.into_iter().flat_map(|arm| arm.code(end)))
                .chain([
                    Op::Const(Value::from(NO_MATCH)).at(span),
                    Op::Ready(Bytecode::PeekDown(1)).at(span),
                    Op::Ready(Bytecode::Concat(
                        NonZeroU64::new(2).expect("two is nonzero"),
                    ))
                    .at(span),
                    Op::Ready(Bytecode::ProduceError).at(span),
                    Op::Label(end).at(span),
                ])
                .collect(),
        })
    }

    /// Lint each arm after the first that matches every value, which no value
    /// can reach.
    fn lint_unreachable_arms(&mut self, arms: &[Spanned<MatchArm>]) {
        let Some(catch_all) = arms.iter().position(|arm| matches_everything(&arm.node)) else {
            return;
        };
        let catch_all_span = arms[catch_all].node.pattern.span;
        for arm in &arms[catch_all + 1..] {
            let lint = self
                .warning(
                    "unreachable match arm",
                    "this match arm is never reached".to_owned(),
                )
                .label_primary(arm.node.pattern.span, "never tried".into())
                .related(
                    Diagnostic::advice("an earlier arm matches every value".to_owned())
                        .label(catch_all_span, "matches every value".into()),
                )
                .help("remove the arm, or move it before the one that matches everything".into());
            self.lint(lint);
        }
    }

    /// Compile an arm, in its own scope: its bindings are visible to its guard
    /// and result only.
    fn compile_arm(
        &mut self,
        arm: &Spanned<MatchArm>,
        position: Position,
    ) -> Result<ArmFragment, Diagnostics> {
        let fail = self.next_label();
        let pattern = self.compile_pattern(
            &arm.node.pattern,
            Mismatch::Jump(fail),
            BindMode::Arm,
            &mut Vec::new(),
        )?;
        let guard = arm
            .node
            .guard
            .as_ref()
            .map(|guard| {
                self.compile_expression(guard, Position::Inner)
                    .map(|fragment| Guard {
                        fragment,
                        span: debug_span(guard.span),
                    })
            })
            .transpose()?;
        let result = self.compile_expression(&arm.node.result, position)?;
        Ok(ArmFragment {
            fail,
            pattern,
            guard,
            result,
            span: debug_span(arm.span),
        })
    }
}

/// Whether `arm` matches every value: it has no guard, and its pattern cannot
/// fail.
fn matches_everything(arm: &MatchArm) -> bool {
    arm.guard.is_none() && cannot_fail(&arm.pattern.node)
}

/// Whether `pattern` matches every value: it is a name or `_` with no type
/// constraint, or an alternative with such a branch.
fn cannot_fail(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Binding {
            type_constraint, ..
        } => type_constraint.is_none(),
        Pattern::Alternative(branches) => branches.iter().any(|branch| cannot_fail(&branch.node)),
        Pattern::Value(_) | Pattern::Array { .. } | Pattern::Map { .. } => false,
    }
}
