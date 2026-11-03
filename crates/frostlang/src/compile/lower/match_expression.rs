//! `match`: try a value against each arm's pattern and guard in turn, and
//! evaluate the result of the first arm that matches.
//!
//! The target stays on the stack while the arms are tried. An arm marks the
//! stack, then tests a copy of the target. A mismatch anywhere jumps to the
//! arm's failure label, which rewinds the stack to the mark, dropping whatever
//! the pattern had laid out, and clears the locals the pattern and guard bound,
//! releasing their values, before the next arm is tried. A matched arm drops
//! its mark and the target before its result, so in tail position the result
//! is the last code its path runs; elsewhere, a clear of all the arm's locals
//! follows it.
//!
//! With [`compile_patterns`](crate::compile::OptimizationOptions::compile_patterns),
//! an arm whose pattern can be held as data instead tests the target in place
//! with one `MatchPattern`. That lays nothing out on the stack, so the arm
//! needs no mark and no copy: a matched arm drops only the target, and a
//! failed one only clears its locals. An arm whose pattern is a lone name or
//! `_`, tried only for its guard, needs no test at all, only the copy of the
//! target that its name binds.
//!
//! The catch-all, an arm with no guard whose pattern cannot fail, is never
//! abandoned, so it needs no mark and no copy: its pattern takes the target
//! itself. Any arm after it is an error, so it is always the last, and it ends
//! the match in place of the error raised when no arm matches.

use std::num::NonZeroU64;

use crate::Value;
use crate::bytecode::{Bytecode, CompiledPattern, Span};
use frostlang_parse::ast::{Expr, MatchArm, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{
        ExprFragment, FunctionBuilder, Ir, JumpType, Label, Op, Position, clear_locals, debug_span,
        lowered_pattern::LoweredPattern,
        pattern::{
            BindMode, Mismatch, PatternBindings, PatternContext, PatternFragment, Unreachable,
            cannot_fail,
        },
    },
};

/// The message raised when no arm matches, followed by the value.
const NO_MATCH: &str = "No match arm matches the value: ";

/// A compiled arm, the label its mismatches jump to, and the arm's span, which
/// trying the arm is derived from.
struct ArmFragment {
    fail: Label,
    test: ArmTest,
    /// See [`PatternFragment::foldable`].
    pattern_foldable: bool,
    guard: Option<Guard>,
    result: ExprFragment,
    /// The clear of the locals its pattern and guard bound, on its failure
    /// path.
    clear_on_failure: Option<Ir>,
    /// The clear of all its locals, after its result.
    clear_after_result: Option<Ir>,
    span: Span,
}

/// How a tried arm tests the target against its pattern, if at all.
enum ArmTest {
    /// The pattern's own instructions, testing a copy of the target under a
    /// mark.
    Instructions(Vec<Ir>),
    /// `MatchPattern`, testing the target in place, derived from the pattern
    /// at `span`.
    MatchPattern { pattern: LoweredPattern, span: Span },
    /// No test, for the pattern at `span` that is a name or `_` with no type
    /// constraint: it matches every value, so the arm is tried only for its
    /// guard. A copy of the target, then the pattern's own `store` of it,
    /// leave the stack as they found it, so the arm needs no mark.
    Untested { store: Vec<Ir>, span: Span },
}

/// A compiled guard, and its span, which its check is derived from.
struct Guard {
    fragment: ExprFragment,
    span: Span,
}

impl ArmFragment {
    fn foldable(&self) -> bool {
        self.pattern_foldable
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
            test,
            pattern_foldable: _,
            guard,
            result,
            clear_on_failure,
            clear_after_result,
            span,
        } = self;
        let ready = |ops: &[Bytecode]| -> Vec<Ir> {
            ops.iter().map(|&op| Op::Ready(op).at(span)).collect()
        };
        let (test_code, on_match, on_failure) = match test {
            ArmTest::Instructions(instructions) => {
                let mut test_code = ready(&[Bytecode::MarkStack, Bytecode::Dup]);
                test_code.extend(instructions);
                (
                    test_code,
                    ready(&[Bytecode::DropMark, Bytecode::Pop]),
                    ready(&[Bytecode::RewindToMark]),
                )
            }
            ArmTest::MatchPattern {
                pattern,
                span: pattern_span,
            } => (
                vec![
                    Op::MatchPattern(Box::new(pattern)).at(pattern_span),
                    Op::Jump {
                        kind: JumpType::IfFalse,
                        label: fail,
                    }
                    .at(pattern_span),
                ],
                ready(&[Bytecode::Pop]),
                Vec::new(),
            ),
            ArmTest::Untested {
                store,
                span: pattern_span,
            } => {
                let mut test_code = vec![Op::Ready(Bytecode::Dup).at(pattern_span)];
                test_code.extend(store);
                (test_code, ready(&[Bytecode::Pop]), Vec::new())
            }
        };
        let guard = guard.into_iter().flat_map(move |guard| {
            guard.fragment.code.into_iter().chain([Op::Jump {
                kind: JumpType::IfFalse,
                label: fail,
            }
            .at(guard.span)])
        });
        test_code
            .into_iter()
            .chain(guard)
            .chain(on_match)
            .chain(result.code)
            .chain(clear_after_result)
            .chain([
                Op::Jump {
                    kind: JumpType::Unconditional,
                    label: end,
                }
                .at(span),
                Op::Label(fail).at(span),
            ])
            .chain(on_failure)
            .chain(clear_on_failure)
    }
}

/// A compiled catch-all arm.
struct CatchAllFragment {
    pattern: PatternFragment,
    result: ExprFragment,
    /// The clear of its locals, after its result.
    clear_after_result: Option<Ir>,
}

impl CatchAllFragment {
    fn foldable(&self) -> bool {
        self.pattern.foldable && self.result.foldable
    }

    /// `( t -- r )`, as the last code of the match.
    fn code(self) -> Vec<Ir> {
        let mut code = self.pattern.code;
        code.extend(self.result.code);
        code.extend(self.clear_after_result);
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
        let start = self.locals.allocation_point();
        let pattern = self.compile_pattern(
            &arm.node.pattern,
            &mut PatternContext::new(
                Mismatch::Jump(fail),
                BindMode::Arm,
                &mut PatternBindings::new(start),
            ),
        )?;
        let pattern_span = debug_span(arm.node.pattern.span);
        let compile_patterns = self.options.optimization_options.compile_patterns;
        let test = match (compile_patterns, pattern.match_pattern) {
            (true, Some(CompiledPattern::Binding { types: None, .. })) => ArmTest::Untested {
                store: pattern.code,
                span: pattern_span,
            },
            (true, Some(match_pattern)) => ArmTest::MatchPattern {
                pattern: match_pattern,
                span: pattern_span,
            },
            (false, _) | (_, None) => ArmTest::Instructions(pattern.code),
        };
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
        let span = debug_span(arm.span);
        // The failure path goes on to another arm, or the error raised when
        // none matches, so it is never the function's last code.
        let clear_on_failure =
            clear_locals(self.locals.allocated_since(start), Position::Inner, span);
        let result = self.compile_expression(&arm.node.result, position)?;
        // Now also the locals the result bound.
        let clear_after_result = clear_locals(self.locals.allocated_since(start), position, span);
        Ok(ArmFragment {
            fail,
            test,
            pattern_foldable: pattern.foldable,
            guard,
            result,
            clear_on_failure,
            clear_after_result,
            span,
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
        let start = self.locals.allocation_point();
        let pattern = self.compile_pattern(
            &arm.node.pattern,
            &mut PatternContext::new(
                Mismatch::Jump(never),
                BindMode::Arm,
                &mut PatternBindings::new(start),
            ),
        )?;
        let result = self.compile_expression(&arm.node.result, position)?;
        let clear_after_result = clear_locals(
            self.locals.allocated_since(start),
            position,
            debug_span(arm.span),
        );
        Ok(CatchAllFragment {
            pattern,
            result,
            clear_after_result,
        })
    }
}

/// Whether `arm` matches every value: it has no guard, and its pattern cannot
/// fail.
fn matches_everything(arm: &MatchArm) -> bool {
    arm.guard.is_none() && cannot_fail(&arm.pattern.node)
}
