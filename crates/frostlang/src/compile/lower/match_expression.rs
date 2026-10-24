//! `match`: try a value against each arm's pattern and guard in turn, and
//! evaluate the result of the first arm that matches.
//!
//! The target stays on the stack while the arms are tried. An arm marks the
//! stack, then tests a copy of the target. A mismatch anywhere jumps to the
//! arm's failure label, which rewinds the stack to the mark, dropping whatever
//! the pattern had laid out, and moves on to the next arm; the locals an
//! abandoned arm bound are never read. A matched arm drops its mark and the
//! target before its result, so the result is the last code its path runs.
//!
//! The catch-all, an arm with no guard whose pattern cannot fail, is never
//! abandoned, so it needs no mark and no copy: its pattern takes the target
//! itself. Any arm after it is an error, so it is always the last, and it ends
//! the match in place of the error raised when no arm matches.

use std::num::NonZeroU64;

use crate::Value;
use crate::bytecode::{Bytecode, Span};
use frostlang_parse::ast::{Expr, MatchArm, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{
        ExprFragment, FunctionBuilder, Ir, JumpType, Label, Op, Position, debug_span,
        pattern::{BindMode, Mismatch, PatternFragment, Unreachable, cannot_fail},
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

/// A compiled catch-all arm.
struct CatchAllFragment {
    pattern: PatternFragment,
    result: ExprFragment,
}

impl CatchAllFragment {
    fn foldable(&self) -> bool {
        self.pattern.foldable && self.result.foldable
    }

    /// `( t -- r )`, as the last code of the match.
    fn code(self) -> Vec<Ir> {
        let mut code = self.pattern.code;
        code.extend(self.result.code);
        code
    }
}

/// `( t -- )`: raise that no arm matches `t`. The raise is derived from the
/// `match` at `span`.
fn no_match_error(span: Span) -> Vec<Ir> {
    vec![
        Op::Const(Value::from(NO_MATCH)).at(span),
        Op::Ready(Bytecode::PeekDown(1)).at(span),
        Op::Ready(Bytecode::Concat(
            NonZeroU64::new(2).expect("two is nonzero"),
        ))
        .at(span),
        Op::Ready(Bytecode::ProduceError).at(span),
    ]
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
        let at = arms
            .iter()
            .position(|arm| matches_everything(&arm.node))
            .unwrap_or(arms.len());
        let (tried, rest) = arms.split_at(at);
        let (catch_all, dead) = match rest.split_first() {
            Some((catch_all, dead)) => (Some(catch_all), dead),
            None => (None, rest),
        };
        let arms = tried
            .iter()
            .map(|arm| self.in_scope(|this| this.compile_arm(arm, position)))
            .collect::<Result<Vec<_>, _>>()?;
        let catch_all_fragment = catch_all
            .map(|arm| self.in_scope(|this| this.compile_catch_all(arm, position)))
            .transpose()?;
        if let (Some(dead), Some(catch_all)) = (dead.first(), catch_all) {
            // Each arm's bindings are out of view again.
            let help = self.catch_all_help(&catch_all.node.pattern.node);
            return Err(self
                .unreachable(
                    Unreachable::Arm,
                    dead.node.pattern.span,
                    catch_all.node.pattern.span,
                    help,
                )
                .into());
        }

        // TODO: with a constant target, branch elimination could pick the arm at
        // compile time.
        let foldable = target.foldable
            && arms.iter().all(ArmFragment::foldable)
            && catch_all_fragment
                .as_ref()
                .is_none_or(CatchAllFragment::foldable);
        // The folding rule (see the `fold` module doc), over the target and each
        // arm's guard and result. A pattern's own expressions fold as it compiles.
        let (target, arms, catch_all_fragment) = if foldable {
            (target, arms, catch_all_fragment)
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
            let catch_all_fragment = catch_all_fragment.map(|fragment| CatchAllFragment {
                result: self.fold_if_eligible(fragment.result),
                ..fragment
            });
            (self.fold_if_eligible(target), arms, catch_all_fragment)
        };

        let ending = match catch_all_fragment {
            Some(fragment) => fragment.code(),
            None => no_match_error(span),
        };
        let end = self.next_label();
        Ok(ExprFragment {
            foldable,
            code: target
                .code
                .into_iter()
                .chain(arms.into_iter().flat_map(|arm| arm.code(end)))
                .chain(ending)
                .chain([Op::Label(end).at(span)])
                .collect(),
        })
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

    /// Compile the catch-all arm, in its own scope: its bindings are visible to
    /// its result only.
    fn compile_catch_all(
        &mut self,
        arm: &Spanned<MatchArm>,
        position: Position,
    ) -> Result<CatchAllFragment, Diagnostics> {
        // The pattern cannot fail, so this label is never placed: assembly
        // panics on a jump to it.
        let never = self.next_label();
        let pattern = self.compile_pattern(
            &arm.node.pattern,
            Mismatch::Jump(never),
            BindMode::Arm,
            &mut Vec::new(),
        )?;
        let result = self.compile_expression(&arm.node.result, position)?;
        Ok(CatchAllFragment { pattern, result })
    }
}

/// Whether `arm` matches every value: it has no guard, and its pattern cannot
/// fail.
fn matches_everything(arm: &MatchArm) -> bool {
    arm.guard.is_none() && cannot_fail(&arm.pattern.node)
}
