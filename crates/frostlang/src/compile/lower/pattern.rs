//! Patterns: testing a value's shape and binding its parts to names, for a
//! `def` and for a `match` arm.
//!
//! A pattern fragment consumes the value it tests, `( x -- )`. Its parts are
//! tested in source order, each by its own fragment, and each binding is in
//! scope for the parts after it. A failed check does what the fragment's
//! [`Mismatch`] says: a `def` raises, and a `match` arm moves on to the next.
//!
//! An alternative, `p1 | p2`, tries each branch but the last under its own
//! mark, rewinding the stack to it when the branch fails. Every check inside
//! such a branch jumps to that branch's failure label. Once the last branch
//! fails, no branch is left, so its checks fail as the whole alternative does:
//! in a `def`, each raises in place that no alternative matches.
//! Every branch binds the same names into the same locals, so what follows
//! reads them alike whichever branch matched.
//!
//! Alongside its code, a pattern is built as a `MatchPattern` operand, a
//! [`LoweredPattern`], when each pin and computed key in it is a constant or a
//! single load (see [`pattern_comparand_of`] and [`pattern_key_of`]). A `match`
//! arm may test that instead; see
//! [`match_expression`](crate::compile::lower::match_expression).

use crate::bytecode::{ArrayPatternLength, Bytecode, CompiledPattern, PatternEntry, Span};
use crate::core::util::count::counted;
use crate::vm::to_operand;
use crate::{FrostType, TypeSet, Value};
use frostlang_parse::ast::{
    Binding, MapPatternEntry, Pattern, SourceSpan, Spanned, TypeConstraint,
};

use crate::compile::{
    Diagnostic, Diagnostics,
    lower::{
        ConstKeyOp, FunctionBuilder, Ir, JumpType, Label, LocalId, Op, Position, debug_span,
        fold::constant_key_of,
        globals::global_slot,
        locals::{AllocationPoint, LocalInfo, LocalKind, LocalRun},
        lowered_pattern::{LoweredPattern, pattern_comparand_of, pattern_key_of},
    },
};

/// The start of every message a raising pattern fails with.
const PATTERN_DOES_NOT_MATCH: &str = "Pattern does not match: ";

/// A compiled pattern: its code, `( x -- )`, and possibly the same pattern as
/// a `MatchPattern` operand.
pub(super) struct PatternFragment {
    pub(super) code: Vec<Ir>,
    /// Whether the pattern uses no runtime input, so code made of it and a
    /// foldable value may fold as a whole.
    pub(super) foldable: bool,
    /// The pattern as a `MatchPattern` operand, if it can be one.
    pub(super) match_pattern: Option<LoweredPattern>,
}

/// What a failed check does. A raise leaves the stack as it is: the error
/// discards it with the frame.
#[derive(Clone, Copy)]
pub(super) enum Mismatch {
    /// Raise an error saying what the check expected.
    Raise,
    /// Raise that the value matches no alternative, as the raising alternative
    /// at this span, whose last branch holds the check.
    RaiseNoAlternative(Span),
    /// Jump to this label, leaving the stack for the label's rewind to restore.
    Jump(Label),
}

/// A name a pattern bound, the local it went to, and where.
#[derive(Clone)]
pub(super) struct Bound {
    pub(super) name: String,
    pub(super) id: LocalId,
    pub(super) span: SourceSpan,
}

/// The names a pattern, or one branch of an alternative in it, has bound so
/// far, and where the whole pattern's locals start.
pub(super) struct PatternBindings {
    /// Where the pattern began allocating locals.
    start: AllocationPoint,
    pub(super) bound: Vec<Bound>,
}

impl PatternBindings {
    /// For a pattern whose locals are allocated from `start`.
    pub(super) fn new(start: AllocationPoint) -> Self {
        Self {
            start,
            bound: Vec::new(),
        }
    }

    /// For a later branch of an alternative in the same pattern, which has
    /// bound nothing yet.
    fn for_branch(&self) -> Self {
        Self::new(self.start)
    }
}

/// Where a pattern binds its names.
#[derive(Clone, Copy)]
pub(super) enum BindMode<'a> {
    /// Each name is a new local in a `match` arm's scope.
    Arm,
    /// Each name is a new local in the current scope, as a `def` binds it:
    /// exported or not, and pending if the def's value is compile-time known
    /// (see `Locals::mark_pending`).
    Def { exported: bool, pending: bool },
    /// Each name must be one the first branch of an alternative (at `span`)
    /// bound, and goes to the same local.
    Reuse {
        first: &'a [Bound],
        span: SourceSpan,
    },
}

/// How a pattern, or a part of one, is compiled: what its failed checks do,
/// and where the names it binds go and are recorded.
pub(super) struct PatternContext<'a> {
    mismatch: Mismatch,
    mode: BindMode<'a>,
    bindings: &'a mut PatternBindings,
    /// Whether the part being compiled is under an optional Map key, so that the
    /// names it binds are [`OptionalBinding`](LocalKind::OptionalBinding)s.
    in_optional_entry: bool,
}

impl<'a> PatternContext<'a> {
    /// For a whole pattern.
    pub(super) fn new(
        mismatch: Mismatch,
        mode: BindMode<'a>,
        bindings: &'a mut PatternBindings,
    ) -> Self {
        Self {
            mismatch,
            mode,
            bindings,
            in_optional_entry: false,
        }
    }

    /// This context, its failed checks handled per `mismatch` instead.
    fn with_mismatch(&mut self, mismatch: Mismatch) -> PatternContext<'_> {
        PatternContext {
            mismatch,
            mode: self.mode,
            bindings: self.bindings,
            in_optional_entry: self.in_optional_entry,
        }
    }

    /// This context, for the part of an optional Map key.
    fn for_optional_entry(&mut self) -> PatternContext<'_> {
        PatternContext {
            mismatch: self.mismatch,
            mode: self.mode,
            bindings: self.bindings,
            in_optional_entry: true,
        }
    }
}

impl FunctionBuilder<'_> {
    /// Compile `pattern`, `( x -- )`, per `ctx`.
    pub(super) fn compile_pattern(
        &mut self,
        pattern: &Spanned<Pattern>,
        ctx: &mut PatternContext,
    ) -> Result<PatternFragment, Diagnostics> {
        let span = debug_span(pattern.span);
        match &pattern.node {
            Pattern::Binding {
                name,
                type_constraint,
            } => {
                let (mut code, types) = match type_constraint {
                    Some(constraint) => {
                        let (ty, described) = type_test(constraint.node);
                        let constraint_span = debug_span(constraint.span);
                        let test = self.check(
                            [
                                Op::Ready(Bytecode::Dup).at(constraint_span),
                                Op::Ready(Bytecode::TypeTest(ty)).at(constraint_span),
                            ],
                            ctx.mismatch,
                            constraint_span,
                            || format!("expected {described}"),
                        );
                        (test, Some(ty))
                    }
                    None => (Vec::new(), None),
                };
                let local = self.bind_and_store(name, ctx, &mut code)?;
                Ok(PatternFragment {
                    code,
                    foldable: true,
                    match_pattern: Some(CompiledPattern::Binding { types, local }),
                })
            }
            Pattern::Value(expr) => {
                // Folded now, so a constant value is a single op.
                let value = self.compile_expression(expr, Position::Inner)?;
                let value = self.fold_if_eligible(value);
                let match_pattern =
                    pattern_comparand_of(&value.code, &self.allocated_in_pattern(ctx.bindings))
                        .map(CompiledPattern::Value);
                let source = self.source;
                let foldable = value.foldable;
                let code = self.check(
                    value
                        .code
                        .into_iter()
                        .chain([Op::Ready(Bytecode::CompareEqual).at(span)]),
                    ctx.mismatch,
                    span,
                    || expected_value(source, pattern.span, expr.span),
                );
                Ok(PatternFragment {
                    code,
                    foldable,
                    match_pattern,
                })
            }
            Pattern::Array {
                elements,
                rest,
                bind_whole,
            } => {
                self.compile_array_pattern(elements, rest.as_ref(), bind_whole.as_ref(), ctx, span)
            }
            Pattern::Map {
                entries,
                bind_whole,
            } => self.compile_map_pattern(entries, bind_whole.as_ref(), ctx, span),
            Pattern::Alternative(branches) => self.compile_alternative(branches, ctx, span),
        }
    }

    /// Every local allocated since the pattern `bindings` belongs to began.
    fn allocated_in_pattern(&self, bindings: &PatternBindings) -> LocalRun {
        self.locals.allocated_since(bindings.start)
    }

    /// Run `test`, which leaves a Bool atop the stack, and handle a false one
    /// per `mismatch`, as the check at `span`. Under [`Mismatch::Raise`], the
    /// error says the check `expected` something.
    fn check(
        &mut self,
        test: impl IntoIterator<Item = Ir>,
        mismatch: Mismatch,
        span: Span,
        expected: impl FnOnce() -> String,
    ) -> Vec<Ir> {
        let test = test.into_iter();
        let (message, raise_span) = match mismatch {
            Mismatch::Jump(label) => {
                return test
                    .chain([Op::Jump {
                        kind: JumpType::IfFalse,
                        label,
                    }
                    .at(span)])
                    .collect();
            }
            Mismatch::Raise => (expected(), span),
            Mismatch::RaiseNoAlternative(alternative) => {
                ("the value matches no alternative".to_owned(), alternative)
            }
        };
        let passed = self.next_label();
        test.chain([
            Op::Jump {
                kind: JumpType::IfTrue,
                label: passed,
            }
            .at(span),
            Op::Const(Value::from(format!("{PATTERN_DOES_NOT_MATCH}{message}"))).at(raise_span),
            Op::Ready(Bytecode::ProduceError).at(raise_span),
            Op::Label(passed).at(span),
        ])
        .collect()
    }

    /// `[element, ..., ...rest] as whole`: check the value's shape (see
    /// [`array_layout`](Self::array_layout)), then test each element against
    /// its pattern in order, and bind the rest, if any. To be bound whole, the
    /// Array is copied beneath its layout first, and bound last.
    /// `( a -- )`, through `( a? rest? aN ... a1 a0 )`.
    fn compile_array_pattern(
        &mut self,
        elements: &[Spanned<Pattern>],
        rest: Option<&Spanned<Binding>>,
        bind_whole: Option<&Spanned<Binding>>,
        ctx: &mut PatternContext,
        span: Span,
    ) -> Result<PatternFragment, Diagnostics> {
        let mut code = Vec::new();
        if bind_whole.is_some() {
            // `( a -- a a )`: the layout takes the copy on top.
            code.push(Op::Ready(Bytecode::Dup).at(span));
        }
        code.extend(self.array_layout(elements.len(), rest.is_some(), ctx.mismatch, span));
        let mut foldable = true;
        let mut match_elements = Vec::new();
        for element in elements {
            let part = self.compile_pattern(element, ctx)?;
            code.extend(part.code);
            foldable &= part.foldable;
            match_elements.push(part.match_pattern);
        }
        let length = match rest {
            Some(rest) => ArrayPatternLength::AtLeast {
                rest: self.bind_and_store(rest, ctx, &mut code)?,
            },
            None => ArrayPatternLength::Exact,
        };
        // Only the copy is left: the rest, beneath the elements, is stored first.
        let whole = match bind_whole {
            Some(binding) => self.bind_and_store(binding, ctx, &mut code)?,
            None => None,
        };
        Ok(PatternFragment {
            code,
            foldable,
            match_pattern: match_elements
                .into_iter()
                .collect::<Option<_>>()
                .map(|elements| CompiledPattern::Array {
                    elements,
                    length,
                    whole,
                }),
        })
    }

    /// Check the value is an Array of `count` elements (at least, if `rest`), then
    /// lay it out for the parts to consume in source order, as the Array pattern at
    /// `span`. A rest splits the Array, its tail beneath the elements; the
    /// elements are exploded, the first on top. `( a -- rest? aN ... a1 a0 )`
    fn array_layout(
        &mut self,
        count: usize,
        rest: bool,
        mismatch: Mismatch,
        span: Span,
    ) -> Vec<Ir> {
        let operand = to_operand(count);
        let (test, bound) = if rest {
            (Bytecode::TestArrayLenAtLeast(operand), "at least")
        } else {
            (Bytecode::TestArrayLenExact(operand), "exactly")
        };
        self.check([Op::Ready(test).at(span)], mismatch, span, || {
            format!("expected an Array of {bound} {}", counted(count, "element"))
        })
        .into_iter()
        .chain(rest.then_some(Op::Ready(Bytecode::SplitArray(operand)).at(span)))
        .chain([Op::Ready(Bytecode::ExplodeArray(operand)).at(span)])
        .collect()
    }

    /// `{key: part, key?: part, ...} as whole`: check the value is a Map, then
    /// take each entry in source order, the Map staying beneath: its key is
    /// compiled and looked up (see [`KeyLookup`]), and its part consumes the
    /// value. The Map left at the end is bound whole, or dropped.
    ///
    /// A missing key is a mismatch, unless optional (see
    /// [`optional_entry`](Self::optional_entry)). Under [`Mismatch::Raise`], the
    /// lookup itself raises, its error naming the key.
    fn compile_map_pattern(
        &mut self,
        entries: &[Spanned<MapPatternEntry>],
        bind_whole: Option<&Spanned<Binding>>,
        ctx: &mut PatternContext,
        span: Span,
    ) -> Result<PatternFragment, Diagnostics> {
        let mut code = self.check(
            [
                Op::Ready(Bytecode::Dup).at(span),
                Op::Ready(Bytecode::TypeTest(FrostType::MAP)).at(span),
            ],
            ctx.mismatch,
            span,
            || "expected a Map".to_string(),
        );
        let mut foldable = true;
        let mut match_entries = Vec::new();

        for entry in entries {
            // Folded now, even if the whole pattern folds later, so a known key
            // is looked up directly.
            let key = self.compile_expression(&entry.node.key, Position::Inner)?;
            let key = self.fold_if_eligible(key);
            let match_key = pattern_key_of(&key.code, &self.allocated_in_pattern(ctx.bindings));
            let lookup = key_lookup(key.code);
            let part = &entry.node.pattern;
            let entry_span = debug_span(entry.span);
            let fragment = if entry.node.optional {
                self.optional_entry(lookup, part, ctx, entry_span)?
            } else {
                self.required_entry(lookup, part, ctx, entry_span)?
            };
            foldable &= key.foldable && fragment.foldable;
            code.extend(fragment.code);
            match_entries.push(match_key.zip(fragment.match_pattern).map(|(key, pattern)| {
                PatternEntry {
                    key,
                    optional: entry.node.optional,
                    pattern,
                }
            }));
        }

        let whole = match bind_whole {
            Some(binding) => self.bind_and_store(binding, ctx, &mut code)?,
            None => {
                code.push(Op::Ready(Bytecode::Pop).at(span));
                None
            }
        };

        Ok(PatternFragment {
            code,
            foldable,
            match_pattern: match_entries
                .into_iter()
                .collect::<Option<_>>()
                .map(|entries| CompiledPattern::Map { entries, whole }),
        })
    }

    /// A required Map entry at `span`, whose key `lookup` finds: check the key
    /// is present, unless a missing key raises in the lookup itself, then look it
    /// up and match its `part` against the value. The fragment's code is the
    /// whole entry's, `( m -- m )`; the rest is its part's.
    fn required_entry(
        &mut self,
        lookup: KeyLookup,
        part: &Spanned<Pattern>,
        ctx: &mut PatternContext,
        span: Span,
    ) -> Result<PatternFragment, Diagnostics> {
        let mut part = self.compile_pattern(part, ctx)?;
        let mut code = lookup.evaluate.unwrap_or_default();
        if !matches!(ctx.mismatch, Mismatch::Raise) {
            code.extend(self.check([lookup.test.at(span)], ctx.mismatch, span, || {
                unreachable!("only `Mismatch::Raise` says what a check expected")
            }));
        }
        code.push(lookup.extract.at(span));
        code.append(&mut part.code);
        part.code = code;
        Ok(part)
    }

    /// An optional Map entry at `span`, whose key `lookup` finds: if the key is
    /// present, look it up and match its `part` against the value; if absent,
    /// define each local the part binds as null instead. The fragment's code is
    /// the whole entry's, `( m -- m )`; the rest is its part's.
    fn optional_entry(
        &mut self,
        lookup: KeyLookup,
        part: &Spanned<Pattern>,
        ctx: &mut PatternContext,
        span: Span,
    ) -> Result<PatternFragment, Diagnostics> {
        let bound_before = ctx.bindings.bound.len();
        let mut part = self.compile_pattern(part, &mut ctx.for_optional_entry())?;
        let absent = self.next_label();
        let done = self.next_label();
        let jump = |kind, label| Op::Jump { kind, label }.at(span);

        let key_on_stack = lookup.evaluate.is_some();
        let mut code = lookup.evaluate.unwrap_or_default();
        code.extend([
            lookup.test.at(span),
            jump(JumpType::IfFalse, absent),
            lookup.extract.at(span),
        ]);
        code.append(&mut part.code);
        code.extend([
            jump(JumpType::Unconditional, done),
            Op::Label(absent).at(span),
        ]);
        if key_on_stack {
            code.push(Op::Ready(Bytecode::Pop).at(span));
        }
        for bound in &ctx.bindings.bound[bound_before..] {
            code.extend([
                Op::Ready(Bytecode::PushNull).at(span),
                Op::DefLocal(bound.id).at(span),
            ]);
        }
        code.push(Op::Label(done).at(span));
        part.code = code;
        Ok(part)
    }

    /// `p1 | p2 | ...`: try each branch in turn under its own mark, but the
    /// last, which needs none: it fails as the alternative does, straight to
    /// the enclosing label, or raising in place. The first branch binds per
    /// `ctx`; every later one must bind the same names, and binds them into
    /// the first's locals. Each branch sees only the names in view before the
    /// alternative, and those it has bound itself.
    fn compile_alternative(
        &mut self,
        branches: &[Spanned<Pattern>],
        ctx: &mut PatternContext,
        span: Span,
    ) -> Result<PatternFragment, Diagnostics> {
        let catch_all = branches.iter().position(|branch| cannot_fail(&branch.node));
        let (reachable, dead) = branches.split_at(catch_all.map_or(branches.len(), |at| at + 1));
        // Before any branch binds, so that the catch-all's own name is not yet
        // in view.
        let help = catch_all.and_then(|at| self.catch_all_help(&branches[at].node));
        let first = branches
            .first()
            .expect("an alternative has at least one branch");
        let last = branches.len() - 1;
        let matched = self.next_label();
        let bound_before = ctx.bindings.bound.len();
        let before = self.locals.checkpoint();
        let mut first_bound = Vec::new();
        let mut code = Vec::new();
        let mut foldable = true;
        let mut match_branches = Vec::new();
        // An alternative nested in the last branch of another raises as that
        // outer one.
        let last_mismatch = match ctx.mismatch {
            Mismatch::Raise => Mismatch::RaiseNoAlternative(span),
            Mismatch::RaiseNoAlternative(_) | Mismatch::Jump(_) => ctx.mismatch,
        };

        for (index, branch) in reachable.iter().enumerate() {
            let branch_fail = (index != last).then(|| self.next_label());
            let branch_mismatch = branch_fail.map_or(last_mismatch, Mismatch::Jump);
            let fragment = if index == 0 {
                let fragment =
                    self.compile_pattern(branch, &mut ctx.with_mismatch(branch_mismatch))?;
                first_bound = ctx.bindings.bound[bound_before..].to_vec();
                fragment
            } else {
                self.locals.rewind(before);
                let mut branch_bindings = ctx.bindings.for_branch();
                let mut branch_ctx = PatternContext {
                    mismatch: branch_mismatch,
                    mode: BindMode::Reuse {
                        first: &first_bound,
                        span: first.span,
                    },
                    bindings: &mut branch_bindings,
                    in_optional_entry: ctx.in_optional_entry,
                };
                let fragment = self.compile_pattern(branch, &mut branch_ctx)?;
                self.check_binds_all(&first_bound, &branch_bindings.bound, branch.span)?;
                fragment
            };
            foldable &= fragment.foldable;
            match_branches.push(fragment.match_pattern);

            // Trying the branch is the branch's own.
            let branch_span = debug_span(branch.span);
            match branch_fail {
                None => code.extend(fragment.code),
                Some(branch_fail) => {
                    code.extend(
                        [Bytecode::MarkStack, Bytecode::Dup]
                            .map(|op| Op::Ready(op).at(branch_span)),
                    );
                    code.extend(fragment.code);
                    code.extend([
                        Op::Ready(Bytecode::DropMark).at(branch_span),
                        Op::Ready(Bytecode::Pop).at(branch_span),
                        Op::Jump {
                            kind: JumpType::Unconditional,
                            label: matched,
                        }
                        .at(branch_span),
                        Op::Label(branch_fail).at(branch_span),
                        Op::Ready(Bytecode::RewindToMark).at(branch_span),
                    ]);
                }
            }
        }
        if let (Some(dead), Some(catch_all)) = (dead.first(), reachable.last()) {
            return Err(self
                .unreachable(Unreachable::Branch, dead.span, catch_all.span, help)
                .into());
        }
        // Only matching gets past the last branch.
        code.push(Op::Label(matched).at(span));

        Ok(PatternFragment {
            code,
            foldable,
            match_pattern: match_branches
                .into_iter()
                .collect::<Option<_>>()
                .map(CompiledPattern::Alternative),
        })
    }

    /// Bind `binding` per `ctx`, and append to `code` the store of the value
    /// atop the stack to its local, `( x -- )`. Returns the local; `None` for
    /// `_`, which binds nothing and drops the value.
    fn bind_and_store(
        &mut self,
        binding: &Spanned<Binding>,
        ctx: &mut PatternContext,
        code: &mut Vec<Ir>,
    ) -> Result<Option<LocalId>, Diagnostics> {
        let local = self.bind(binding, ctx)?;
        code.push(
            local
                .map_or(Op::Ready(Bytecode::Pop), Op::DefLocal)
                .at(debug_span(binding.span)),
        );
        Ok(local)
    }

    /// Bind `binding` per `ctx`'s mode, recording it in its bindings, and
    /// return its local, as [`bind_and_store`](Self::bind_and_store) says.
    fn bind(
        &mut self,
        binding: &Spanned<Binding>,
        ctx: &mut PatternContext,
    ) -> Result<Option<LocalId>, Diagnostics> {
        let Binding::Named(name) = &binding.node else {
            return Ok(None);
        };
        let kind = if ctx.in_optional_entry {
            LocalKind::OptionalBinding
        } else {
            LocalKind::Binding
        };
        let bound = &mut ctx.bindings.bound;
        let id = match ctx.mode {
            BindMode::Arm => self.define_binding(binding, name, false, kind)?,
            BindMode::Def { exported, pending } => {
                let id = self.define_binding(binding, name, exported, kind)?;
                if pending {
                    self.locals.mark_pending(id);
                }
                id
            }
            BindMode::Reuse { first, span } => {
                if let Some(earlier) = bound.iter().find(|earlier| earlier.name == *name) {
                    return Err(self
                        .duplicate_binding(name, binding.span, earlier.span)
                        .into());
                }
                match first.iter().find(|original| original.name == *name) {
                    Some(original) => {
                        self.locals.revive(original.id);
                        if kind == LocalKind::OptionalBinding {
                            self.locals.mark_optional(original.id);
                        }
                        original.id
                    }
                    None => {
                        return Err(self
                            .alternative_mismatch(name, binding.span, "bound only here".into())
                            .related(
                                Diagnostic::advice(format!(
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
        Ok(Some(id))
    }

    /// Define `name`, which `binding` binds, as a new local of `kind` in the
    /// current scope.
    fn define_binding(
        &mut self,
        binding: &Spanned<Binding>,
        name: &str,
        exported: bool,
        kind: LocalKind,
    ) -> Result<LocalId, Diagnostics> {
        self.lint_type_name_binding(name, binding.span);
        self.locals
            .define(LocalInfo {
                name: name.to_owned(),
                span: binding.span,
                exported,
                constant: None,
                kind,
            })
            .map_err(|original| self.duplicate_binding(name, binding.span, original).into())
    }

    /// Check a later alternative branch, at `span`, bound every name in `first`.
    fn check_binds_all(
        &self,
        first: &[Bound],
        branch: &[Bound],
        span: SourceSpan,
    ) -> Result<(), Diagnostics> {
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
                    Diagnostic::advice(format!("the first alternative binds `{}`", missing.name))
                        .label(missing.span, "bound here".into()),
                )
                .into()),
            None => Ok(()),
        }
    }

    /// The error for alternatives that bind different names, `name` among them.
    fn alternative_mismatch(&self, name: &str, span: SourceSpan, label: String) -> Diagnostic {
        self.error(format!(
            "every alternative must bind the same names, but only some bind `{name}`"
        ))
        .code("alternative bindings".into())
        .label_primary(span, label)
    }

    /// The error for the `kind` of part at `dead`, which no value reaches,
    /// since the earlier part at `catch_all` matches every value. `help`, from
    /// [`catch_all_help`](Self::catch_all_help), goes with the catch-all.
    pub(super) fn unreachable(
        &self,
        kind: Unreachable,
        dead: SourceSpan,
        catch_all: SourceSpan,
        help: Option<String>,
    ) -> Diagnostic {
        let noun = kind.noun();
        let mut advice = Diagnostic::advice(format!("an earlier {noun} matches every value"))
            .label(catch_all, "matches every value".into());
        if let Some(help) = help {
            advice = advice.help(help);
        }
        self.error(format!("this {noun} is never reached"))
            .code(kind.code().into())
            .label_primary(dead, "never tried".into())
            .related(advice)
            .help(format!(
                "remove the {noun}, or move it before the one that matches every value"
            ))
    }

    /// Help for a catch-all `pattern` that is a bare name, which binds every
    /// value but may have been meant as a test: of the type it names, or
    /// against the name, if one is in view. Ask before the pattern binds
    /// anything, or its own binding is in view.
    pub(super) fn catch_all_help(&self, pattern: &Pattern) -> Option<String> {
        let Pattern::Binding {
            name:
                Spanned {
                    node: Binding::Named(name),
                    ..
                },
            ..
        } = pattern
        else {
            return None;
        };
        if TypeConstraint::from_name(name).is_some() {
            return Some(format!("to test for the type, write `_ is {name}`"));
        }
        // TODO: An enclosing scope's name is in view only if this function
        // reads it, since only names read are captured, and a pattern binding
        // the name does not read it. So a name bound only in an enclosing
        // function, or in a `compile_in_scope` caller's scope (such as an
        // earlier REPL input), gets no hint.
        (self.locals.resolve(name).is_some() || global_slot(name).is_some())
            .then(|| format!("to compare against an existing `{name}`, write `({name})`"))
    }
}

/// What part of a pattern or `match` is unreachable: one after a part that
/// matches every value, which no value reaches. It is a compile error,
/// reported after any error in the parts before it, and is never compiled.
#[derive(Clone, Copy)]
pub(super) enum Unreachable {
    /// A `match` arm after an arm with no guard whose pattern cannot fail.
    Arm,
    /// A branch of an alternative after a branch that cannot fail.
    Branch,
}

impl Unreachable {
    fn noun(self) -> &'static str {
        match self {
            Self::Arm => "match arm",
            Self::Branch => "alternative branch",
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::Arm => "unreachable match arm",
            Self::Branch => "unreachable alternative branch",
        }
    }
}

/// How a Map pattern entry looks up its key, the Map beneath.
struct KeyLookup {
    /// Evaluates the key, `( m -- m k )`; `None` for a known key, which the
    /// lookups hold, leaving no `k` on the stack.
    evaluate: Option<Vec<Ir>>,
    /// Tests whether the key is present, `( m k? -- m k? b )`.
    test: Op,
    /// Takes the key's value, `( m k? -- m v )`.
    extract: Op,
}

/// The lookup of the key `key_code` evaluates: directly, if the code is a known
/// key.
fn key_lookup(key_code: Vec<Ir>) -> KeyLookup {
    match constant_key_of(&key_code) {
        Some(known_key) => KeyLookup {
            evaluate: None,
            test: Op::ConstKey {
                op: ConstKeyOp::Test,
                key: known_key.clone(),
            },
            extract: Op::ConstKey {
                op: ConstKeyOp::Extract,
                key: known_key,
            },
        },
        None => KeyLookup {
            evaluate: Some(key_code),
            test: Op::Ready(Bytecode::TestKey),
            extract: Op::Ready(Bytecode::ExtractKey),
        },
    }
}

/// Whether `pattern` matches every value: a name or `_` with no type
/// constraint, or an alternative with such a branch.
pub(super) fn cannot_fail(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Binding {
            type_constraint, ..
        } => type_constraint.is_none(),
        Pattern::Alternative(branches) => branches.iter().any(|branch| cannot_fail(&branch.node)),
        Pattern::Value(_) | Pattern::Array { .. } | Pattern::Map { .. } => false,
    }
}

/// What a value pattern at `pattern` expected, quoting its source: the value of
/// a pin `(expr)`, whose span holds its parentheses as `expr`'s does not, or a
/// literal.
fn expected_value(source: &str, pattern: SourceSpan, expr: SourceSpan) -> String {
    let text = &source[pattern.start..pattern.end];
    if pattern == expr {
        format!("expected `{text}`")
    } else {
        format!("expected the value of `{text}`")
    }
}

/// The test for a type constraint, `( x -- b )`, and how a mismatch error names
/// what it expected.
fn type_test(constraint: TypeConstraint) -> (TypeSet, &'static str) {
    match constraint {
        TypeConstraint::Null => (FrostType::NULL, "Null"),
        TypeConstraint::Int => (FrostType::INT, "an Int"),
        TypeConstraint::Float => (FrostType::FLOAT, "a Float"),
        TypeConstraint::Bool => (FrostType::BOOL, "a Bool"),
        TypeConstraint::String => (FrostType::STRING, "a String"),
        TypeConstraint::Bytes => (FrostType::BYTES, "Bytes"),
        TypeConstraint::Array => (FrostType::ARRAY, "an Array"),
        TypeConstraint::Map => (FrostType::MAP, "a Map"),
        TypeConstraint::Function => (FrostType::FUNCTION, "a Function"),
        TypeConstraint::Opaque => (FrostType::OPAQUE, "an Opaque"),
        TypeConstraint::Primitive => (FrostType::PRIMITIVE, "a Primitive"),
        TypeConstraint::Numeric => (FrostType::NUMERIC, "a Numeric"),
        TypeConstraint::Structured => (FrostType::STRUCTURED, "a Structured"),
        TypeConstraint::Flat => (FrostType::FLAT, "a Flat"),
        TypeConstraint::Nonnull => (FrostType::NONNULL, "a Nonnull"),
    }
}
