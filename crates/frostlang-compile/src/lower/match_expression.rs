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
//! An alternative, `p1 | p2`, nests the same scheme inside a pattern: each
//! branch but the last is tried under its own mark, and the last fails to the
//! enclosing failure label. Every branch binds the same names into the same
//! locals, so what follows reads them alike whichever branch matched.

use std::num::NonZeroUsize;

use frostlang_parse::ast::{
    Binding, Expr, MapPatternEntry, MatchArm, MatchPattern, SourceSpan, Spanned, TypeConstraint,
};
use frostlang_runtime::{Bytecode, FrostType, Value};

use crate::{
    CompilerError, CompilerErrors,
    lower::{
        ConstKeyOp, ExprFragment, FunctionBuilder, Ir, JumpType, Label, LocalId, Position,
        destructure::Mismatch,
        fold::constant_key_of,
        locals::{LocalInfo, LocalKind},
    },
};

/// The message raised when no arm matches, followed by the value.
const NO_MATCH: &str = "No match arm matches the value: ";

/// A compiled pattern, `( x -- )`. On a mismatch it jumps to its failure label,
/// leaving the stack for that label's rewind to restore.
struct PatternFragment {
    code: Vec<Ir>,
    foldable: bool,
}

/// A compiled arm, and the label its mismatches jump to.
struct ArmFragment {
    fail: Label,
    pattern: PatternFragment,
    guard: Option<ExprFragment>,
    result: ExprFragment,
}

impl ArmFragment {
    fn foldable(&self) -> bool {
        self.pattern.foldable
            && self.guard.as_ref().is_none_or(|guard| guard.foldable)
            && self.result.foldable
    }

    /// `( t -- r )` on to the end of the match if the arm matches, else
    /// `( t -- t )` on to the next arm.
    fn code(self, end: Label) -> impl Iterator<Item = Ir> {
        let fail = self.fail;
        let guard = self.guard.into_iter().flat_map(move |guard| {
            guard.code.into_iter().chain([Ir::Jump {
                kind: JumpType::IfFalse,
                label: fail,
            }])
        });
        [Bytecode::MarkStack, Bytecode::Dup]
            .map(Ir::Ready)
            .into_iter()
            .chain(self.pattern.code)
            .chain(guard)
            .chain([Bytecode::DropMark, Bytecode::Pop].map(Ir::Ready))
            .chain(self.result.code)
            .chain([
                Ir::Jump {
                    kind: JumpType::Unconditional,
                    label: end,
                },
                Ir::Label(fail),
                Ir::Ready(Bytecode::RewindToMark),
            ])
    }
}

/// A name a pattern bound, the local it went to, and where.
#[derive(Clone)]
struct Bound {
    name: String,
    id: LocalId,
    span: SourceSpan,
}

/// Where a pattern binds its names.
#[derive(Clone, Copy)]
enum BindMode<'a> {
    /// Each name is a new local in the arm's scope.
    Define,
    /// Each name must be one the first branch of an alternative (at `span`)
    /// bound, and goes to the same local.
    Reuse {
        first: &'a [Bound],
        span: SourceSpan,
    },
}

impl FunctionBuilder<'_> {
    pub(super) fn compile_match_expression(
        &mut self,
        target: &Spanned<Expr>,
        arms: &[Spanned<MatchArm>],
        position: Position,
    ) -> Result<ExprFragment, CompilerErrors> {
        let target = self.compile_expression(target, Position::Inner)?;
        let arms = arms
            .iter()
            .map(|arm| self.in_scope(|this| this.compile_arm(&arm.node, position)))
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
                    guard: arm.guard.map(|guard| self.fold_if_eligible(guard)),
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
                    Ir::Const(Value::from(NO_MATCH)),
                    Ir::Ready(Bytecode::PeekDown(1)),
                    Ir::Ready(Bytecode::Concat(
                        NonZeroUsize::new(2).expect("two is nonzero"),
                    )),
                    Ir::Ready(Bytecode::ProduceError),
                    Ir::Label(end),
                ])
                .collect(),
        })
    }

    /// Compile an arm, in its own scope: its bindings are visible to its guard
    /// and result only.
    fn compile_arm(
        &mut self,
        arm: &MatchArm,
        position: Position,
    ) -> Result<ArmFragment, CompilerErrors> {
        let fail = self.next_label();
        let pattern =
            self.compile_pattern(&arm.pattern, fail, BindMode::Define, &mut Vec::new())?;
        let guard = arm
            .guard
            .as_ref()
            .map(|guard| self.compile_expression(guard, Position::Inner))
            .transpose()?;
        let result = self.compile_expression(&arm.result, position)?;
        Ok(ArmFragment {
            fail,
            pattern,
            guard,
            result,
        })
    }

    /// Compile `pattern`, `( x -- )`, jumping to `fail` on a mismatch. Each name
    /// it binds is bound per `mode` and recorded in `bound`.
    fn compile_pattern(
        &mut self,
        pattern: &Spanned<MatchPattern>,
        fail: Label,
        mode: BindMode,
        bound: &mut Vec<Bound>,
    ) -> Result<PatternFragment, CompilerErrors> {
        match &pattern.node {
            MatchPattern::Binding {
                name,
                type_constraint,
            } => {
                let test = type_constraint.as_ref().map(|constraint| {
                    self.check_shape(
                        [
                            Ir::Ready(Bytecode::Dup),
                            Ir::Ready(type_test(constraint.node)),
                        ],
                        Mismatch::Jump(fail),
                    )
                });
                let store = self.bind(name, mode, bound)?;
                Ok(PatternFragment {
                    code: test.into_iter().flatten().chain([store]).collect(),
                    foldable: true,
                })
            }
            MatchPattern::Value(expr) => {
                // Folded now, so a constant value is a single op.
                let value = self.compile_expression(expr, Position::Inner)?;
                let value = self.fold_if_eligible(value);
                Ok(PatternFragment {
                    foldable: value.foldable,
                    code: value
                        .code
                        .into_iter()
                        .chain([
                            Ir::Ready(Bytecode::CompareEqual),
                            Ir::Jump {
                                kind: JumpType::IfFalse,
                                label: fail,
                            },
                        ])
                        .collect(),
                })
            }
            MatchPattern::Array { elements, rest } => {
                let layout =
                    self.array_layout(elements.len(), rest.is_some(), Mismatch::Jump(fail));
                let parts = elements
                    .iter()
                    .map(|element| self.compile_pattern(element, fail, mode, bound))
                    .collect::<Result<Vec<_>, _>>()?;
                let rest = rest
                    .as_ref()
                    .map(|rest| self.bind(rest, mode, bound))
                    .transpose()?;
                Ok(PatternFragment {
                    foldable: parts.iter().all(|part| part.foldable),
                    code: layout
                        .into_iter()
                        .chain(parts.into_iter().flat_map(|part| part.code))
                        .chain(rest)
                        .collect(),
                })
            }
            MatchPattern::Map {
                entries,
                bind_whole,
            } => self.compile_map_pattern(entries, bind_whole.as_ref(), fail, mode, bound),
            MatchPattern::Alternative(branches) => {
                self.compile_alternative(branches, fail, mode, bound)
            }
        }
    }

    /// `{key: part, ...} as whole`: as a Map destructure, but a missing key is a
    /// mismatch, not an error.
    fn compile_map_pattern(
        &mut self,
        entries: &[Spanned<MapPatternEntry>],
        bind_whole: Option<&Spanned<Binding>>,
        fail: Label,
        mode: BindMode,
        bound: &mut Vec<Bound>,
    ) -> Result<PatternFragment, CompilerErrors> {
        let mut code = self.check_shape(
            [
                Ir::Ready(Bytecode::Dup),
                Ir::Ready(Bytecode::TypeTest(FrostType::MAP)),
            ],
            Mismatch::Jump(fail),
        );
        let mut foldable = true;

        for entry in entries {
            // Folded now, as a destructure's key is, so a known key is looked up
            // directly.
            let key = self.compile_expression(&entry.node.key, Position::Inner)?;
            let key = self.fold_if_eligible(key);
            let part = self.compile_pattern(&entry.node.pattern, fail, mode, bound)?;
            foldable &= key.foldable && part.foldable;
            let missing = Ir::Jump {
                kind: JumpType::IfFalse,
                label: fail,
            };
            match constant_key_of(&key.code) {
                Some(known_key) => code.extend([
                    Ir::ConstKey {
                        op: ConstKeyOp::Test,
                        key: known_key.clone(),
                    },
                    missing,
                    Ir::ConstKey {
                        op: ConstKeyOp::Extract,
                        key: known_key,
                    },
                ]),
                None => code.extend(key.code.into_iter().chain([
                    Ir::Ready(Bytecode::TestKey),
                    missing,
                    Ir::Ready(Bytecode::ExtractKey),
                ])),
            }
            code.extend(part.code);
        }

        let whole = match bind_whole {
            Some(binding) => self.bind(binding, mode, bound)?,
            None => Ir::Ready(Bytecode::Pop),
        };
        code.push(whole);

        Ok(PatternFragment { code, foldable })
    }

    /// `p1 | p2 | ...`: try each branch in turn, each but the last under its own
    /// mark. The first branch binds per `mode`; every later one must bind the
    /// same names, and binds them into the first's locals. Each branch sees only
    /// the names in view before the alternative, and those it has bound itself.
    fn compile_alternative(
        &mut self,
        branches: &[Spanned<MatchPattern>],
        fail: Label,
        mode: BindMode,
        bound: &mut Vec<Bound>,
    ) -> Result<PatternFragment, CompilerErrors> {
        let (first, _) = branches
            .split_first()
            .expect("an alternative has at least one branch");
        let last = branches.len() - 1;
        let matched = self.next_label();
        let start = bound.len();
        let before = self.locals.checkpoint();
        let mut first_bound = Vec::new();
        let mut code = Vec::new();
        let mut foldable = true;

        for (index, branch) in branches.iter().enumerate() {
            let branch_fail = if index == last {
                fail
            } else {
                self.next_label()
            };
            let fragment = if index == 0 {
                let fragment = self.compile_pattern(branch, branch_fail, mode, bound)?;
                first_bound = bound[start..].to_vec();
                fragment
            } else {
                self.locals.rewind(before);
                let mode = BindMode::Reuse {
                    first: &first_bound,
                    span: first.span,
                };
                let mut branch_bound = Vec::new();
                let fragment =
                    self.compile_pattern(branch, branch_fail, mode, &mut branch_bound)?;
                self.check_binds_all(&first_bound, &branch_bound, branch.span)?;
                fragment
            };
            foldable &= fragment.foldable;

            if index == last {
                code.extend(fragment.code);
            } else {
                code.extend([Bytecode::MarkStack, Bytecode::Dup].map(Ir::Ready));
                code.extend(fragment.code);
                code.extend([
                    Ir::Ready(Bytecode::DropMark),
                    Ir::Ready(Bytecode::Pop),
                    Ir::Jump {
                        kind: JumpType::Unconditional,
                        label: matched,
                    },
                    Ir::Label(branch_fail),
                    Ir::Ready(Bytecode::RewindToMark),
                ]);
            }
        }
        code.push(Ir::Label(matched));

        Ok(PatternFragment { code, foldable })
    }

    /// Bind the value atop the stack to `binding` per `mode`, recording it in
    /// `bound`. `( x -- )`
    fn bind(
        &mut self,
        binding: &Spanned<Binding>,
        mode: BindMode,
        bound: &mut Vec<Bound>,
    ) -> Result<Ir, CompilerErrors> {
        let Binding::Named(name) = &binding.node else {
            return Ok(Ir::Ready(Bytecode::Pop));
        };
        let id = match mode {
            BindMode::Define => self
                .locals
                .define(LocalInfo {
                    name: name.clone(),
                    span: binding.span,
                    exported: false,
                    constant: None,
                    kind: LocalKind::Binding,
                })
                .map_err(|original| self.duplicate_binding(name, binding.span, original))?,
            BindMode::Reuse { first, span } => {
                if let Some(earlier) = bound.iter().find(|earlier| earlier.name == *name) {
                    return Err(self
                        .duplicate_binding(name, binding.span, earlier.span)
                        .into());
                }
                match first.iter().find(|original| original.name == *name) {
                    Some(original) => {
                        self.locals.revive(original.id);
                        original.id
                    }
                    None => {
                        return Err(self
                            .alternative_mismatch(name, binding.span, "bound only here".into())
                            .related(
                                CompilerError::advice(format!(
                                    "the first alternative does not bind `{name}`"
                                ))
                                .label(span, "first alternative".into()),
                            )
                            .into());
                    }
                }
            }
        };
        bound.push(Bound {
            name: name.clone(),
            id,
            span: binding.span,
        });
        Ok(Ir::DefLocal(id))
    }

    /// Check a later alternative branch, at `span`, bound every name in `first`.
    fn check_binds_all(
        &self,
        first: &[Bound],
        branch: &[Bound],
        span: SourceSpan,
    ) -> Result<(), CompilerErrors> {
        match first
            .iter()
            .find(|original| !branch.iter().any(|bound| bound.name == original.name))
        {
            Some(missing) => Err(self
                .alternative_mismatch(
                    &missing.name,
                    span,
                    format!("does not bind `{}`", missing.name),
                )
                .related(
                    CompilerError::advice(format!(
                        "the first alternative binds `{}`",
                        missing.name
                    ))
                    .label(missing.span, "bound here".into()),
                )
                .into()),
            None => Ok(()),
        }
    }

    /// The error for alternatives that bind different names, `name` among them.
    fn alternative_mismatch(&self, name: &str, span: SourceSpan, label: String) -> CompilerError {
        self.error(format!(
            "every alternative must bind the same names, but only some bind `{name}`"
        ))
        .code("alternative bindings".into())
        .label_primary(span, label)
    }
}

/// The test for a type constraint: `( x -- b )`.
fn type_test(constraint: TypeConstraint) -> Bytecode {
    Bytecode::TypeTest(match constraint {
        TypeConstraint::Null => FrostType::NULL,
        TypeConstraint::Int => FrostType::INT,
        TypeConstraint::Float => FrostType::FLOAT,
        TypeConstraint::Bool => FrostType::BOOL,
        TypeConstraint::String => FrostType::STRING,
        TypeConstraint::Bytes => FrostType::BYTES,
        TypeConstraint::Array => FrostType::ARRAY,
        TypeConstraint::Map => FrostType::MAP,
        TypeConstraint::Function => FrostType::FUNCTION,
        TypeConstraint::Opaque => FrostType::OPAQUE,
        TypeConstraint::Primitive => FrostType::PRIMITIVE,
        TypeConstraint::Numeric => FrostType::NUMERIC,
        TypeConstraint::Structured => FrostType::STRUCTURED,
        TypeConstraint::Flat => FrostType::FLAT,
        TypeConstraint::Nonnull => FrostType::NONNULL,
    })
}
