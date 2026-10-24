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

use crate::bytecode::{Bytecode, Span};
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
        locals::{LocalInfo, LocalKind},
    },
};

/// The start of every message a raising pattern fails with.
const PATTERN_DOES_NOT_MATCH: &str = "Pattern does not match: ";

/// A compiled pattern, `( x -- )`.
pub(super) struct PatternFragment {
    pub(super) code: Vec<Ir>,
    /// Whether the pattern uses no runtime input, so code made of it and a
    /// foldable value may fold as a whole.
    pub(super) foldable: bool,
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

impl FunctionBuilder<'_> {
    /// Compile `pattern`, `( x -- )`, handling a failed check per `mismatch`.
    /// Each name it binds is bound per `mode` and recorded in `bound`.
    pub(super) fn compile_pattern(
        &mut self,
        pattern: &Spanned<Pattern>,
        mismatch: Mismatch,
        mode: BindMode,
        bound: &mut Vec<Bound>,
    ) -> Result<PatternFragment, Diagnostics> {
        let span = debug_span(pattern.span);
        match &pattern.node {
            Pattern::Binding {
                name,
                type_constraint,
            } => {
                let test = type_constraint.as_ref().map(|constraint| {
                    let (ty, described) = type_test(constraint.node);
                    let constraint_span = debug_span(constraint.span);
                    self.check(
                        [
                            Op::Ready(Bytecode::Dup).at(constraint_span),
                            Op::Ready(Bytecode::TypeTest(ty)).at(constraint_span),
                        ],
                        mismatch,
                        constraint_span,
                        || format!("expected {described}"),
                    )
                });
                let store = self.bind(name, mode, bound)?;
                Ok(PatternFragment {
                    code: test.into_iter().flatten().chain([store]).collect(),
                    foldable: true,
                })
            }
            Pattern::Value(expr) => {
                // Folded now, so a constant value is a single op.
                let value = self.compile_expression(expr, Position::Inner)?;
                let value = self.fold_if_eligible(value);
                let source = self.source;
                let foldable = value.foldable;
                let code = self.check(
                    value
                        .code
                        .into_iter()
                        .chain([Op::Ready(Bytecode::CompareEqual).at(span)]),
                    mismatch,
                    span,
                    || expected_value(source, pattern.span, expr.span),
                );
                Ok(PatternFragment { code, foldable })
            }
            Pattern::Array { elements, rest } => {
                let layout = self.array_layout(elements.len(), rest.is_some(), mismatch, span);
                let parts = elements
                    .iter()
                    .map(|element| self.compile_pattern(element, mismatch, mode, bound))
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
            Pattern::Map {
                entries,
                bind_whole,
            } => {
                self.compile_map_pattern(entries, bind_whole.as_ref(), mismatch, mode, bound, span)
            }
            Pattern::Alternative(branches) => {
                self.compile_alternative(branches, mismatch, mode, bound, span)
            }
        }
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

    /// `{key: part, ...} as whole`: check the value is a Map, then take each key
    /// in source order, the Map staying beneath. A known key is looked up
    /// directly, `( m -- m v )`; any other is evaluated in place first,
    /// `( m -- m k )`, then looked up, `( m k -- m v )`. Its part consumes the
    /// value. The Map left at the end is bound whole, or dropped.
    ///
    /// A missing key is a mismatch. Under [`Mismatch::Raise`], the lookup itself
    /// raises, its error naming the key.
    fn compile_map_pattern(
        &mut self,
        entries: &[Spanned<MapPatternEntry>],
        bind_whole: Option<&Spanned<Binding>>,
        mismatch: Mismatch,
        mode: BindMode,
        bound: &mut Vec<Bound>,
        span: Span,
    ) -> Result<PatternFragment, Diagnostics> {
        let mut code = self.check(
            [
                Op::Ready(Bytecode::Dup).at(span),
                Op::Ready(Bytecode::TypeTest(FrostType::MAP)).at(span),
            ],
            mismatch,
            span,
            || "expected a Map".to_string(),
        );
        let mut foldable = true;

        for entry in entries {
            // Folded now, even if the whole pattern folds later, so a known key
            // is looked up directly.
            let key = self.compile_expression(&entry.node.key, Position::Inner)?;
            let key = self.fold_if_eligible(key);
            let part = self.compile_pattern(&entry.node.pattern, mismatch, mode, bound)?;
            foldable &= key.foldable && part.foldable;
            // The entry's own lookup.
            let entry_span = debug_span(entry.span);
            let (test, extract) = match constant_key_of(&key.code) {
                Some(known_key) => (
                    Op::ConstKey {
                        op: ConstKeyOp::Test,
                        key: known_key.clone(),
                    },
                    Op::ConstKey {
                        op: ConstKeyOp::Extract,
                        key: known_key,
                    },
                ),
                None => {
                    code.extend(key.code);
                    (
                        Op::Ready(Bytecode::TestKey),
                        Op::Ready(Bytecode::ExtractKey),
                    )
                }
            };
            if !matches!(mismatch, Mismatch::Raise) {
                code.extend(self.check([test.at(entry_span)], mismatch, entry_span, || {
                    unreachable!("only `Mismatch::Raise` says what a check expected")
                }));
            }
            code.push(extract.at(entry_span));
            code.extend(part.code);
        }

        let whole = match bind_whole {
            Some(binding) => self.bind(binding, mode, bound)?,
            None => Op::Ready(Bytecode::Pop).at(span),
        };
        code.push(whole);

        Ok(PatternFragment { code, foldable })
    }

    /// `p1 | p2 | ...`: try each branch in turn under its own mark, but the
    /// last, which needs none: it fails as the alternative does, straight to
    /// the enclosing label, or raising in place. The first branch binds per
    /// `mode`; every later one must bind the same names, and binds them into
    /// the first's locals. Each branch sees only the names in view before the
    /// alternative, and those it has bound itself.
    fn compile_alternative(
        &mut self,
        branches: &[Spanned<Pattern>],
        mismatch: Mismatch,
        mode: BindMode,
        bound: &mut Vec<Bound>,
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
        let start = bound.len();
        let before = self.locals.checkpoint();
        let mut first_bound = Vec::new();
        let mut code = Vec::new();
        let mut foldable = true;
        // An alternative nested in the last branch of another raises as that
        // outer one.
        let last_mismatch = match mismatch {
            Mismatch::Raise => Mismatch::RaiseNoAlternative(span),
            Mismatch::RaiseNoAlternative(_) | Mismatch::Jump(_) => mismatch,
        };

        for (index, branch) in reachable.iter().enumerate() {
            let branch_fail = (index != last).then(|| self.next_label());
            let branch_mismatch = branch_fail.map_or(last_mismatch, Mismatch::Jump);
            let fragment = if index == 0 {
                let fragment = self.compile_pattern(branch, branch_mismatch, mode, bound)?;
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
                    self.compile_pattern(branch, branch_mismatch, mode, &mut branch_bound)?;
                self.check_binds_all(&first_bound, &branch_bound, branch.span)?;
                fragment
            };
            foldable &= fragment.foldable;

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

        Ok(PatternFragment { code, foldable })
    }

    /// Bind the value atop the stack to `binding` per `mode`, recording it in
    /// `bound`. `( x -- )`
    fn bind(
        &mut self,
        binding: &Spanned<Binding>,
        mode: BindMode,
        bound: &mut Vec<Bound>,
    ) -> Result<Ir, Diagnostics> {
        let span = debug_span(binding.span);
        let Binding::Named(name) = &binding.node else {
            return Ok(Op::Ready(Bytecode::Pop).at(span));
        };
        let id = match mode {
            BindMode::Arm => self.define_binding(binding, name, false)?,
            BindMode::Def { exported, pending } => {
                let id = self.define_binding(binding, name, exported)?;
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
        Ok(Op::DefLocal(id).at(span))
    }

    /// Define `name`, which `binding` binds, as a new local in the current scope.
    fn define_binding(
        &mut self,
        binding: &Spanned<Binding>,
        name: &str,
        exported: bool,
    ) -> Result<LocalId, Diagnostics> {
        self.lint_type_name_binding(name, binding.span);
        self.locals
            .define(LocalInfo {
                name: name.to_owned(),
                span: binding.span,
                exported,
                constant: None,
                kind: LocalKind::Binding,
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
