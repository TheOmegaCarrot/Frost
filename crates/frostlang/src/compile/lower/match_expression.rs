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
//!
//! A target whose value is compile-time known is tried against each arm's
//! pattern now, on the fold VM, before the arm's guard and result compile. An
//! arm's outcome, if it is reached, depends only on the target, so it stands
//! even when an arm before it is left undecided. With propagation, a matched
//! pattern's names record the values they took, for its guard and result to
//! propagate. With branch elimination, an arm that cannot match is left out,
//! and the first that must match if reached ends the match, binding its names
//! without a test where it can.

use std::num::NonZeroU64;

use crate::Value;
use crate::bytecode::{Bytecode, CompiledPattern, Span};
use frostlang_parse::ast::{Expr, MatchArm, Spanned};

use crate::compile::{
    Diagnostics,
    lower::{
        ExprFragment, FunctionBuilder, Ir, JumpType, Label, Op, Position, clear_locals, debug_span,
        fold::{KnownMatch, constant_of},
        lowered_pattern::LoweredPattern,
        pattern::{
            BindMode, Bound, Mismatch, PatternBindings, PatternContext, PatternFragment,
            Unreachable, bind_known, cannot_fail,
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
    decision: ArmDecision,
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

/// What a compile-time-known target decides about an arm, if it is tried.
enum ArmDecision {
    /// Only trying it at runtime can tell.
    Undecided,
    /// It does not match.
    Never,
    /// It matches. `binds` binds its names to the values they take, `( -- )`:
    /// `Some` where they can be bound without a test (see
    /// [`bind_known_if_folding`](FunctionBuilder::bind_known_if_folding)).
    Always { binds: Option<Vec<Ir>> },
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
            decision: _,
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
    /// Binds its names to the values they take from a compile-time-known
    /// target, `( -- )`, as [`ArmDecision::Always`] does.
    binds: Option<Vec<Ir>>,
    result: ExprFragment,
    /// The clear of its locals, after its result.
    clear_after_result: Option<Ir>,
    span: Span,
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

/// An arm known to match when the match reaches it, compiled to bind its names
/// without a test.
struct KnownArm {
    binds: Vec<Ir>,
    result: ExprFragment,
    clear_after_result: Option<Ir>,
    span: Span,
}

/// What a match does once no tried arm has matched.
enum Ending {
    /// Raise that no arm matches the target.
    NoMatch,
    CatchAll(CatchAllFragment),
    /// Take an arm known to match. Unlike the others, it needs no target.
    Known(KnownArm),
}

impl Ending {
    fn foldable(&self) -> bool {
        match self {
            Ending::NoMatch => true,
            Ending::CatchAll(fragment) => fragment.foldable(),
            Ending::Known(arm) => arm.result.foldable,
        }
    }

    fn needs_target(&self) -> bool {
        !matches!(self, Ending::Known(_))
    }

    /// The ending with its result folded if eligible.
    fn fold_result(self, builder: &FunctionBuilder) -> Self {
        match self {
            Ending::NoMatch => Ending::NoMatch,
            Ending::CatchAll(fragment) => Ending::CatchAll(CatchAllFragment {
                result: builder.fold_if_eligible(fragment.result),
                ..fragment
            }),
            Ending::Known(arm) => Ending::Known(KnownArm {
                result: builder.fold_if_eligible(arm.result),
                ..arm
            }),
        }
    }

    /// `( t -- r )` if `target_on_stack`, else `( -- r )`, as the last code of
    /// the match at `span`. Only a [`Known`](Ending::Known) arm can do without
    /// the target.
    fn code(self, target_on_stack: bool, span: Span) -> Vec<Ir> {
        match self {
            Ending::NoMatch => no_match_error(span),
            Ending::CatchAll(fragment) => fragment.code(),
            Ending::Known(arm) => target_on_stack
                .then(|| Op::Ready(Bytecode::Pop).at(arm.span))
                .into_iter()
                .chain(arm.binds)
                .chain(arm.result.code)
                .chain(arm.clear_after_result)
                .collect(),
        }
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
        let optimizations = self.options.optimization_options;
        let target = self.compile_expression(target, Position::Inner)?;
        // The target's value is learned only for what it decides: what an arm
        // binds, and which arms can match. A known target is tried as the code
        // loading it, since its value may not be a constant, as a Function is
        // not.
        let (target, known_target) =
            if optimizations.constant_propagate || optimizations.branch_eliminate {
                let (target, value) = self.fold_binding_value(target);
                let known = value.map(|_| target.code.clone());
                (target, known)
            } else {
                (target, None)
            };
        let known_target = known_target.as_deref();

        let at = arms
            .iter()
            .position(|arm| matches_everything(&arm.node))
            .unwrap_or(arms.len());
        let (tried, rest) = arms.split_at(at);
        let (catch_all, dead) = match rest.split_first() {
            Some((catch_all, dead)) => (Some(catch_all), dead),
            None => (None, rest),
        };
        // Branch elimination leaves out every arm after one that must match, so
        // an arm after it is not tried on the target: it compiles only for its
        // diagnostics, as an eliminated `if` branch does.
        let mut arms = Vec::with_capacity(tried.len());
        let mut decided = false;
        for arm in tried {
            let known_target = known_target.filter(|_| !decided);
            let fragment = self.in_scope(|this| this.compile_arm(arm, position, known_target))?;
            decided |= matches!(fragment.decision, ArmDecision::Always { .. });
            arms.push(fragment);
        }
        let known_target = known_target.filter(|_| !decided);
        let catch_all_fragment = catch_all
            .map(|arm| self.in_scope(|this| this.compile_catch_all(arm, position, known_target)))
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

        let (arms, ending) = if optimizations.branch_eliminate {
            eliminate_arms(arms, catch_all_fragment)
        } else {
            let ending = catch_all_fragment.map_or(Ending::NoMatch, Ending::CatchAll);
            (arms, ending)
        };
        // Only a known target yields an ending that needs none, and a known
        // target has no effect: with no arm left to try it on, it is not emitted.
        let target = (!arms.is_empty() || ending.needs_target()).then_some(target);

        let foldable = target.as_ref().is_none_or(|target| target.foldable)
            && arms.iter().all(ArmFragment::foldable)
            && ending.foldable();
        // The folding rule (see the `fold` module doc), over the target and each
        // emitted arm's guard and result. A pattern's own expressions fold as it
        // compiles. A known target, and a guard deciding its arm, may already
        // have folded, to learn their values.
        let (target, arms, ending) = if foldable {
            (target, arms, ending)
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
            let target = target.map(|target| self.fold_if_eligible(target));
            (target, arms, ending.fold_result(self))
        };

        let target_on_stack = target.is_some();
        let end = self.next_label();
        Ok(ExprFragment {
            foldable,
            code: target
                .into_iter()
                .flat_map(|target| target.code)
                .chain(arms.into_iter().flat_map(|arm| arm.code(end)))
                .chain(ending.code(target_on_stack, span))
                .chain([Op::Label(end).at(span)])
                .collect(),
        })
    }

    /// Compile an arm, in its own scope: its bindings are visible to its guard
    /// and result only. `known_target` is the code loading the target, if its
    /// value is compile-time known.
    fn compile_arm(
        &mut self,
        arm: &Spanned<MatchArm>,
        position: Position,
        known_target: Option<&[Ir]>,
    ) -> Result<ArmFragment, Diagnostics> {
        let fail = self.next_label();
        let start = self.locals.allocation_point();
        let mut bindings = PatternBindings::new(start);
        let pattern = self.compile_pattern(
            &arm.node.pattern,
            &mut PatternContext::new(Mismatch::Jump(fail), BindMode::Arm, &mut bindings),
        )?;
        let pattern_span = debug_span(arm.node.pattern.span);
        let known = self.match_known_target(known_target, &pattern, fail, &bindings.bound);
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
        // Only branch elimination acts on a decision.
        let (guard, decision) = if self.options.optimization_options.branch_eliminate {
            self.decide_arm(known, guard, &bindings.bound, pattern_span)
        } else {
            (guard, ArmDecision::Undecided)
        };
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
            decision,
            span,
        })
    }

    /// Compile the catch-all arm, in its own scope: its bindings are visible to
    /// its result only. `known_target` is as for
    /// [`compile_arm`](Self::compile_arm).
    fn compile_catch_all(
        &mut self,
        arm: &Spanned<MatchArm>,
        position: Position,
        known_target: Option<&[Ir]>,
    ) -> Result<CatchAllFragment, Diagnostics> {
        // The pattern cannot fail, so the match's code never places this label:
        // assembly panics on a jump to it. Only trying the pattern at compile
        // time places it, with nothing jumping to it.
        let never = self.next_label();
        let start = self.locals.allocation_point();
        let mut bindings = PatternBindings::new(start);
        let pattern = self.compile_pattern(
            &arm.node.pattern,
            &mut PatternContext::new(Mismatch::Jump(never), BindMode::Arm, &mut bindings),
        )?;
        let known = self.match_known_target(known_target, &pattern, never, &bindings.bound);
        // Only branch elimination ends a match with a known catch-all.
        let binds = match known {
            Some(KnownMatch::Matches(values))
                if self.options.optimization_options.branch_eliminate =>
            {
                self.bind_known_if_folding(
                    &values,
                    &bindings.bound,
                    debug_span(arm.node.pattern.span),
                )
            }
            Some(KnownMatch::Matches(_) | KnownMatch::Fails) | None => None,
        };
        let result = self.compile_expression(&arm.node.result, position)?;
        let span = debug_span(arm.span);
        let clear_after_result = clear_locals(self.locals.allocated_since(start), position, span);
        Ok(CatchAllFragment {
            pattern,
            binds,
            result,
            clear_after_result,
            span,
        })
    }

    /// Try `pattern`, whose mismatches jump to `fail`, on the target loaded by
    /// `known_target`, if any, and with propagation on, record the value each
    /// of `bound` takes on a match. `None` if there is no known target, the
    /// pattern uses runtime input, or trying it errors. Without folding, a
    /// pattern that computes a value it compares against or a key it looks up
    /// is not tried either: as for an `if` condition, only a constant is known
    /// then.
    fn match_known_target(
        &mut self,
        known_target: Option<&[Ir]>,
        pattern: &PatternFragment,
        fail: Label,
        bound: &[Bound],
    ) -> Option<KnownMatch> {
        let decidable = pattern.foldable
            && (!pattern.computed || self.options.optimization_options.constant_fold);
        let target = known_target.filter(|_| decidable)?;
        let ids: Vec<_> = bound.iter().map(|bound| bound.id).collect();
        let known = self.match_known(target, &pattern.code, fail, &ids)?;
        if let KnownMatch::Matches(values) = &known
            && self.options.optimization_options.constant_propagate
        {
            for (id, value) in ids.into_iter().zip(values) {
                self.locals.set_constant(id, value.clone());
            }
        }
        Some(known)
    }

    /// What an arm's pattern, `known` to match or not on the target, and its
    /// guard decide about the arm, with the guard as it is then: folded, if
    /// the pattern matches, to learn whether it passes.
    fn decide_arm(
        &self,
        known: Option<KnownMatch>,
        guard: Option<Guard>,
        bound: &[Bound],
        pattern_span: Span,
    ) -> (Option<Guard>, ArmDecision) {
        let values = match known {
            None => return (guard, ArmDecision::Undecided),
            Some(KnownMatch::Fails) => return (guard, ArmDecision::Never),
            Some(KnownMatch::Matches(values)) => values,
        };
        let guard = guard.map(|guard| Guard {
            fragment: self.fold_if_eligible(guard.fragment),
            ..guard
        });
        let passes = guard.as_ref().map_or(Some(true), |guard| {
            constant_of(&guard.fragment.code).map(|value| value.is_truthy())
        });
        let decision = match passes {
            Some(true) => ArmDecision::Always {
                binds: self.bind_known_if_folding(&values, bound, pattern_span),
            },
            Some(false) => ArmDecision::Never,
            None => ArmDecision::Undecided,
        };
        (guard, decision)
    }

    /// [`bind_known`], only with constant folding on: binding the values a
    /// match at compile time found, in place of the match, is folding it. A
    /// pattern that binds no name has no value to bind, so needs no fold.
    fn bind_known_if_folding(
        &self,
        values: &[Value],
        bound: &[Bound],
        matched: Span,
    ) -> Option<Vec<Ir>> {
        if bound.is_empty() {
            return Some(Vec::new());
        }
        if !self.options.optimization_options.constant_fold {
            return None;
        }
        bind_known(values, bound, matched)
    }
}

/// The arms a match tries and how it ends, once those that cannot be reached
/// or cannot match are left out. The first arm that must match ends the match:
/// as a [`KnownArm`] where its names can be bound without a test, else tried as
/// usual, with the error that no arm matches after it, which it never reaches.
fn eliminate_arms(
    arms: Vec<ArmFragment>,
    catch_all: Option<CatchAllFragment>,
) -> (Vec<ArmFragment>, Ending) {
    let mut tried = Vec::new();
    for mut arm in arms {
        match std::mem::replace(&mut arm.decision, ArmDecision::Undecided) {
            ArmDecision::Never => {}
            ArmDecision::Undecided => tried.push(arm),
            ArmDecision::Always { binds: Some(binds) } => {
                let known = KnownArm {
                    binds,
                    result: arm.result,
                    clear_after_result: arm.clear_after_result,
                    span: arm.span,
                };
                return (tried, Ending::Known(known));
            }
            ArmDecision::Always { binds: None } => {
                tried.push(arm);
                return (tried, Ending::NoMatch);
            }
        }
    }
    let ending = match catch_all {
        None => Ending::NoMatch,
        Some(CatchAllFragment {
            pattern: _,
            binds: Some(binds),
            result,
            clear_after_result,
            span,
        }) => Ending::Known(KnownArm {
            binds,
            result,
            clear_after_result,
            span,
        }),
        Some(catch_all) => Ending::CatchAll(catch_all),
    };
    (tried, ending)
}

/// Whether `arm` matches every value: it has no guard, and its pattern cannot
/// fail.
fn matches_everything(arm: &MatchArm) -> bool {
    arm.guard.is_none() && cannot_fail(&arm.pattern.node)
}
