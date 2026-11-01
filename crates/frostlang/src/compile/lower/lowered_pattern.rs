//! Patterns as IR: a [`CompiledPattern`] whose locals are [`LocalId`]s and
//! whose constants and keys are held by value, until assembly places them.

use crate::bytecode::{
    ArrayPatternLength, Bytecode, CompiledPattern, PatternComparand, PatternInput, PatternKey,
    PatternStage,
};
use crate::compile::lower::fold::constant_of;
use crate::compile::lower::locals::LocalRun;
use crate::compile::lower::{Ir, LocalId, Op};
use crate::{MapKey, Value};

/// The [`PatternStage`] of IR patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LoweredStage {}

impl PatternStage for LoweredStage {
    type Local = LocalId;
    type Constant = Value;
    type Key = MapKey;
}

pub(super) type LoweredPattern = CompiledPattern<LoweredStage>;

/// What a pin, compiled to `code`, compares against, if a pattern can hold it.
/// See [`input_of`] for `allocated_in_pattern`.
pub(super) fn pattern_comparand_of(
    code: &[Ir],
    allocated_in_pattern: &LocalRun,
) -> Option<PatternComparand<LoweredStage>> {
    match constant_of(code) {
        Some(value) => Some(comparand_of_value(value)),
        None => input_of(code, allocated_in_pattern).map(PatternComparand::Input),
    }
}

/// The key a Map pattern entry looks up, its key compiled to `code`, if a
/// pattern can hold it. A constant that is not a valid key cannot be held:
/// only the lookup's own instruction raises for it, and only once reached.
/// See [`input_of`] for `allocated_in_pattern`.
pub(super) fn pattern_key_of(
    code: &[Ir],
    allocated_in_pattern: &LocalRun,
) -> Option<PatternKey<LoweredStage>> {
    match constant_of(code) {
        Some(value) => MapKey::try_from(value).ok().map(PatternKey::Constant),
        None => input_of(code, allocated_in_pattern).map(PatternKey::Input),
    }
}

/// The comparand for a known value: a scalar inline, anything else as a
/// constant, so that one value always has one comparand.
fn comparand_of_value(value: Value) -> PatternComparand<LoweredStage> {
    match value {
        Value::Null => PatternComparand::Null,
        Value::Bool(bool) => PatternComparand::Bool(bool),
        Value::Int(int) => PatternComparand::Int(int),
        Value::Float(float) => PatternComparand::Float(float),
        other => PatternComparand::Constant(other),
    }
}

/// The input `code` loads, if it is a single load of a local or a global.
/// `allocated_in_pattern` holds every local allocated since the pattern began.
///
/// A loaded local among them is one the pattern bound earlier (in a later
/// branch of an alternative, by that branch, since the first branch's
/// bindings leave view). It is read as bound in the same test, since its slot
/// is written only once the whole pattern matches. No other local allocated
/// since the pattern began can be loaded alone: one that code within a pin or
/// key defines is in view only to that code, which then is more than a single
/// load.
fn input_of(code: &[Ir], allocated_in_pattern: &LocalRun) -> Option<PatternInput<LoweredStage>> {
    let [ir] = code else {
        return None;
    };
    match ir.op {
        Op::LoadLocal(id) if allocated_in_pattern.contains(id) => {
            Some(PatternInput::EarlierBinding(id))
        }
        Op::LoadLocal(id) => Some(PatternInput::Local(id)),
        Op::Ready(Bytecode::LoadGlobal(global)) => Some(PatternInput::Global(
            u32::try_from(global).expect("a global's index fits a u32"),
        )),
        _ => None,
    }
}

/// How [`LoweredPattern::map_stage`] turns an IR pattern's references into
/// those of stage `T`.
pub(super) trait StageMap<T: PatternStage> {
    fn local(&mut self, local: &LocalId) -> T::Local;
    fn constant(&mut self, constant: &Value) -> T::Constant;
    fn key(&mut self, key: &MapKey) -> T::Key;

    /// The local a part binds to `local` becomes, or `None` to bind nothing.
    /// Defaults to [`local`](Self::local); a map that treats bindings
    /// differently from reads overrides it.
    fn binding(&mut self, local: &LocalId) -> Option<T::Local> {
        Some(self.local(local))
    }
}

/// A reference a pattern makes to a local or global, as
/// [`LoweredPattern::for_each_reference`] meets it.
enum PatternReference<'a> {
    /// A value the pattern reads.
    Input(&'a PatternInput<LoweredStage>),
    /// A local the pattern binds on a match.
    Binding(LocalId),
}

impl LoweredPattern {
    /// The same pattern in stage `T`, its references turned by `map`.
    pub(super) fn map_stage<T: PatternStage>(
        &self,
        map: &mut impl StageMap<T>,
    ) -> CompiledPattern<T> {
        match self {
            Self::Binding { types, local } => CompiledPattern::Binding {
                types: *types,
                local: local.and_then(|local| map.binding(&local)),
            },
            Self::Value(comparand) => CompiledPattern::Value(match comparand {
                PatternComparand::Null => PatternComparand::Null,
                PatternComparand::Bool(bool) => PatternComparand::Bool(*bool),
                PatternComparand::Int(int) => PatternComparand::Int(*int),
                PatternComparand::Float(float) => PatternComparand::Float(*float),
                PatternComparand::Constant(constant) => {
                    PatternComparand::Constant(map.constant(constant))
                }
                PatternComparand::Input(input) => PatternComparand::Input(map_input(input, map)),
            }),
            Self::Array { elements, length } => CompiledPattern::Array {
                elements: elements
                    .iter()
                    .map(|element| element.map_stage(map))
                    .collect(),
                length: match length {
                    ArrayPatternLength::Exact => ArrayPatternLength::Exact,
                    ArrayPatternLength::AtLeast { rest } => ArrayPatternLength::AtLeast {
                        rest: rest.and_then(|rest| map.binding(&rest)),
                    },
                },
            },
            Self::Map { entries, whole } => CompiledPattern::Map {
                entries: entries
                    .iter()
                    .map(|(key, entry)| {
                        let key = match key {
                            PatternKey::Constant(key) => PatternKey::Constant(map.key(key)),
                            PatternKey::Input(input) => PatternKey::Input(map_input(input, map)),
                        };
                        (key, entry.map_stage(map))
                    })
                    .collect(),
                whole: whole.and_then(|whole| map.binding(&whole)),
            },
            Self::Alternative(branches) => CompiledPattern::Alternative(
                branches
                    .iter()
                    .map(|branch| branch.map_stage(map))
                    .collect(),
            ),
        }
    }

    /// Call `visit` with each reference the pattern makes, in the order a test
    /// makes them, every branch of an alternative in turn.
    fn for_each_reference<'a>(&'a self, visit: &mut impl FnMut(PatternReference<'a>)) {
        match self {
            Self::Binding { local, .. } => {
                if let Some(local) = local {
                    visit(PatternReference::Binding(*local));
                }
            }
            Self::Value(comparand) => {
                if let PatternComparand::Input(input) = comparand {
                    visit(PatternReference::Input(input));
                }
            }
            Self::Array { elements, length } => {
                for element in elements {
                    element.for_each_reference(visit);
                }
                if let ArrayPatternLength::AtLeast { rest: Some(rest) } = length {
                    visit(PatternReference::Binding(*rest));
                }
            }
            Self::Map { entries, whole } => {
                for (key, entry) in entries {
                    if let PatternKey::Input(input) = key {
                        visit(PatternReference::Input(input));
                    }
                    entry.for_each_reference(visit);
                }
                if let Some(whole) = whole {
                    visit(PatternReference::Binding(*whole));
                }
            }
            Self::Alternative(branches) => {
                for branch in branches {
                    branch.for_each_reference(visit);
                }
            }
        }
    }

    /// The locals the pattern reads from their slots.
    pub(super) fn slot_reads(&self) -> Vec<LocalId> {
        let mut reads = Vec::new();
        self.for_each_reference(&mut |reference| {
            if let PatternReference::Input(PatternInput::Local(id)) = reference {
                reads.push(*id);
            }
        });
        reads
    }

    /// The locals the pattern binds on a match, in the order it binds them.
    /// Each alternative's branches bind the same locals, so each such local
    /// appears once per branch.
    pub(super) fn bindings(&self) -> Vec<LocalId> {
        let mut bindings = Vec::new();
        self.for_each_reference(&mut |reference| {
            if let PatternReference::Binding(id) = reference {
                bindings.push(id);
            }
        });
        bindings
    }

    /// Bind nothing to each local that `keep` rejects and the pattern itself
    /// does not read as an earlier binding: the part is still tested, but not
    /// kept.
    pub(super) fn discard_bindings(&mut self, keep: impl Fn(LocalId) -> bool) {
        let mut read_within = Vec::new();
        self.for_each_reference(&mut |reference| {
            if let PatternReference::Input(PatternInput::EarlierBinding(id)) = reference {
                read_within.push(*id);
            }
        });
        *self = self.map_stage(&mut BindingFilter {
            keep: |id| keep(id) || read_within.contains(&id),
        });
    }
}

fn map_input<T: PatternStage>(
    input: &PatternInput<LoweredStage>,
    map: &mut impl StageMap<T>,
) -> PatternInput<T> {
    match input {
        PatternInput::Local(local) => PatternInput::Local(map.local(local)),
        PatternInput::Global(global) => PatternInput::Global(*global),
        PatternInput::EarlierBinding(local) => PatternInput::EarlierBinding(map.local(local)),
    }
}

/// The [`StageMap`] of [`LoweredPattern::discard_bindings`]: copies a pattern
/// unchanged, except that it drops each binding whose local `keep` rejects.
struct BindingFilter<K: Fn(LocalId) -> bool> {
    keep: K,
}

impl<K: Fn(LocalId) -> bool> StageMap<LoweredStage> for BindingFilter<K> {
    fn local(&mut self, local: &LocalId) -> LocalId {
        *local
    }

    fn constant(&mut self, constant: &Value) -> Value {
        constant.clone()
    }

    fn key(&mut self, key: &MapKey) -> MapKey {
        key.clone()
    }

    fn binding(&mut self, local: &LocalId) -> Option<LocalId> {
        (self.keep)(*local).then_some(*local)
    }
}
