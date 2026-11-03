//! [`CompiledPattern`]: a pattern saved as data, for
//! [`Bytecode::MatchPattern`](crate::bytecode::Bytecode::MatchPattern) to test a
//! value against.

use std::borrow::Cow;
use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::bytecode::CompiledFunction;
use crate::vm::from_operand;
use crate::vm::globals::GlobalSet;
use crate::{FrostError, FrostFloat, MapKey, TypeSet, Value};

/// A Frost pattern, such as a `match` arm's or a `def`'s, saved as data: what
/// [`Bytecode::MatchPattern`](crate::bytecode::Bytecode::MatchPattern) tests a
/// value against, and the locals it binds to the value's parts.
///
/// Parts are tested in order, depth first, and a mismatch ends the test. A
/// `whole` and an Array's `rest` are bound after the parts within them, and
/// an Array's `whole` after its `rest`.
///
/// A pattern binds only locals, never captures, and binds no local twice in
/// one match; the branches of an [`Alternative`](Self::Alternative) bind the
/// same locals.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CompiledPattern<S: PatternStage = AssembledStage> {
    /// Matches a value whose type is in `types`, or any value if `None`, and
    /// binds it to `local`, if any.
    Binding {
        /// The types the value may have.
        types: Option<TypeSet>,
        /// The local bound to the value.
        local: Option<S::Local>,
    },
    /// Matches a value equal to the comparand.
    Value(PatternComparand<S>),
    /// Matches an Array whose elements, in order, match `elements`, and whose
    /// length is as `length` says, and binds the Array to `whole`, if any.
    Array {
        /// The patterns of the leading elements.
        elements: Box<[CompiledPattern<S>]>,
        /// The Array's length, relative to `elements`.
        length: ArrayPatternLength<S>,
        /// The local bound to the Array.
        whole: Option<S::Local>,
    },
    /// Matches a Map holding every key of `entries` that is not optional, the
    /// value of each present key matching its pattern, and binds the Map to
    /// `whole`, if any.
    Map {
        /// The entries, tested in order.
        entries: Box<[PatternEntry<S>]>,
        /// The local bound to the Map.
        whole: Option<S::Local>,
    },
    /// Matches a value any branch matches. Branches are tried in order, and the
    /// first that matches is committed to: it binds, and if a later part of the
    /// pattern then fails, no other branch is tried. An error in a branch
    /// raises rather than trying the next. Every branch must bind the same
    /// locals.
    Alternative(Box<[CompiledPattern<S>]>),
}

/// How the length of an Array matched by a [`CompiledPattern::Array`] relates
/// to the number of element patterns.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ArrayPatternLength<S: PatternStage = AssembledStage> {
    /// The Array has exactly as many elements.
    Exact,
    /// The Array has at least as many elements. Those past them are bound, as
    /// a new Array, to `rest`, if any.
    AtLeast {
        /// The local bound to the remaining elements.
        rest: Option<S::Local>,
    },
}

/// What a [`CompiledPattern::Value`] compares the value against, with
/// [`Value`]'s equality.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PatternComparand<S: PatternStage = AssembledStage> {
    /// `null`.
    Null,
    /// A Bool.
    Bool(bool),
    /// An Int.
    Int(i64),
    /// A Float.
    Float(FrostFloat),
    /// A constant.
    Constant(S::Constant),
    /// A value taken from outside the pattern.
    Input(PatternInput<S>),
}

/// One entry of a [`CompiledPattern::Map`]: a key, and the pattern its value
/// must match.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PatternEntry<S: PatternStage = AssembledStage> {
    /// The key looked up.
    pub key: PatternKey<S>,
    /// Whether the key may be absent. An absent optional key matches without
    /// testing `pattern`, and binds null to each local `pattern` binds.
    pub optional: bool,
    /// The pattern the key's value must match.
    pub pattern: CompiledPattern<S>,
}

/// The key a [`CompiledPattern::Map`] entry looks up.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PatternKey<S: PatternStage = AssembledStage> {
    /// A constant key.
    Constant(S::Key),
    /// A value taken from outside the pattern.
    Input(PatternInput<S>),
}

/// A value a [`CompiledPattern`] takes from outside the value it tests.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PatternInput<S: PatternStage = AssembledStage> {
    /// A local, or a capture, of the running function.
    Local(S::Local),
    /// A global, by its index in [`GLOBAL_NAMES`](crate::bytecode::GLOBAL_NAMES).
    Global(u32),
    /// A local the pattern binds, as bound earlier in the same test. The local
    /// itself is not yet defined; see
    /// [`Bytecode::MatchPattern`](crate::bytecode::Bytecode::MatchPattern).
    EarlierBinding(S::Local),
}

/// How a [`CompiledPattern`] refers to locals, constants, and Map keys.
/// [`CompiledFunction::patterns`] uses [`AssembledStage`].
pub trait PatternStage: Clone + Debug + PartialEq {
    /// How a local is referred to.
    type Local: Clone + Debug + PartialEq + Serialize + DeserializeOwned;
    /// How a constant is referred to.
    type Constant: Clone + Debug + PartialEq + Serialize + DeserializeOwned;
    /// How a constant Map key is referred to.
    type Key: Clone + Debug + PartialEq + Serialize + DeserializeOwned;
}

/// The [`PatternStage`] of bytecode's patterns: a local by its slot, a
/// constant by its index in [`CompiledFunction::constants`], and a Map key by
/// its index in [`CompiledFunction::key_constants`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssembledStage {}

impl PatternStage for AssembledStage {
    type Local = u32;
    type Constant = u32;
    type Key = u32;
}

/// Test `value` against `pattern`, a pattern of `function`, whose running
/// frame's slots are `slots`. On a match, define each local the pattern binds;
/// on a mismatch, define none. Until the test is done, the values it binds are
/// only borrowed.
pub(super) fn match_pattern(
    pattern: &CompiledPattern,
    value: &Value,
    function: &CompiledFunction,
    slots: &mut [Option<Value>],
    globals: &GlobalSet,
) -> Result<bool, FrostError> {
    let mut test = PatternTest {
        function,
        slots,
        globals,
        pending: Vec::new(),
    };
    if !test.matches(pattern, value)? {
        return Ok(false);
    }
    for PendingBinding { local, value } in test.pending {
        let slot = index(local);
        debug_assert!(
            slot >= function.num_captures,
            "a pattern of `{}` binds capture {local}",
            function.name
        );
        slots[slot] = Some(value.into_value());
    }
    Ok(true)
}

/// One test of a value against a pattern, collecting what it binds as it goes.
struct PatternTest<'a, 'v> {
    function: &'a CompiledFunction,
    slots: &'a [Option<Value>],
    globals: &'a GlobalSet,
    /// What the test has bound so far, to be written to the locals on a match.
    pending: Vec<PendingBinding<'v>>,
}

/// A local a pattern binds, and the value it binds, not yet written to the
/// local.
struct PendingBinding<'v> {
    local: u32,
    value: BoundValue<'v>,
}

/// The value a local is bound to, borrowed from the tested value.
enum BoundValue<'v> {
    Value(&'v Value),
    /// An Array's elements past those a pattern tested, which a rest binds as
    /// a new Array.
    Rest(&'v [Value]),
}

impl<'v> BoundValue<'v> {
    fn value(&self) -> Cow<'v, Value> {
        match *self {
            BoundValue::Value(value) => Cow::Borrowed(value),
            BoundValue::Rest(elements) => Cow::Owned(Value::from(elements.to_vec())),
        }
    }

    fn into_value(self) -> Value {
        self.value().into_owned()
    }
}

impl<'a, 'v> PatternTest<'a, 'v> {
    fn matches(&mut self, pattern: &CompiledPattern, value: &'v Value) -> Result<bool, FrostError> {
        match pattern {
            CompiledPattern::Binding { types, local } => {
                if types.is_some_and(|types| !value.fits(types)) {
                    return Ok(false);
                }
                self.bind(*local, BoundValue::Value(value));
                Ok(true)
            }
            CompiledPattern::Value(comparand) => Ok(*value == *self.comparand(comparand)),
            CompiledPattern::Array {
                elements,
                length,
                whole,
            } => {
                let Some(array) = value.as_array() else {
                    return Ok(false);
                };
                let array = array.as_slice();
                let fits = match length {
                    ArrayPatternLength::Exact => array.len() == elements.len(),
                    ArrayPatternLength::AtLeast { .. } => array.len() >= elements.len(),
                };
                if !fits {
                    return Ok(false);
                }
                for (element_pattern, element) in elements.iter().zip(array) {
                    if !self.matches(element_pattern, element)? {
                        return Ok(false);
                    }
                }
                if let ArrayPatternLength::AtLeast { rest } = length {
                    self.bind(*rest, BoundValue::Rest(&array[elements.len()..]));
                }
                self.bind(*whole, BoundValue::Value(value));
                Ok(true)
            }
            CompiledPattern::Map { entries, whole } => {
                let Some(map) = value.as_map() else {
                    return Ok(false);
                };
                for entry in entries {
                    let Some(found) = map.get(&*self.key(&entry.key)?) else {
                        if !entry.optional {
                            return Ok(false);
                        }
                        self.bind_null(&entry.pattern);
                        continue;
                    };
                    if !self.matches(&entry.pattern, found)? {
                        return Ok(false);
                    }
                }
                self.bind(*whole, BoundValue::Value(value));
                Ok(true)
            }
            CompiledPattern::Alternative(branches) => {
                let before = self.pending.len();
                for branch in branches {
                    if self.matches(branch, value)? {
                        return Ok(true);
                    }
                    self.pending.truncate(before);
                }
                Ok(false)
            }
        }
    }

    /// Bind `local`, if any, to `value`.
    fn bind(&mut self, local: Option<u32>, value: BoundValue<'v>) {
        if let Some(local) = local {
            self.pending.push(PendingBinding { local, value });
        }
    }

    /// Bind null to each local `pattern` binds, in the order a match would
    /// bind them, without testing it.
    fn bind_null(&mut self, pattern: &CompiledPattern) {
        static NULL: Value = Value::Null;
        let null = || BoundValue::Value(&NULL);
        match pattern {
            CompiledPattern::Binding { local, .. } => self.bind(*local, null()),
            CompiledPattern::Value(_) => {}
            CompiledPattern::Array {
                elements,
                length,
                whole,
            } => {
                for element in elements {
                    self.bind_null(element);
                }
                if let ArrayPatternLength::AtLeast { rest } = length {
                    self.bind(*rest, null());
                }
                self.bind(*whole, null());
            }
            CompiledPattern::Map { entries, whole } => {
                for entry in entries {
                    self.bind_null(&entry.pattern);
                }
                self.bind(*whole, null());
            }
            // Every branch binds the same locals.
            CompiledPattern::Alternative(branches) => {
                if let Some(first) = branches.first() {
                    self.bind_null(first);
                }
            }
        }
    }

    fn comparand(&self, comparand: &PatternComparand) -> Cow<'_, Value> {
        match comparand {
            PatternComparand::Null => Cow::Owned(Value::Null),
            PatternComparand::Bool(bool) => Cow::Owned(Value::Bool(*bool)),
            PatternComparand::Int(int) => Cow::Owned(Value::Int(*int)),
            PatternComparand::Float(float) => Cow::Owned(Value::Float(*float)),
            PatternComparand::Constant(constant) => {
                Cow::Borrowed(&self.function.constants[index(*constant)])
            }
            PatternComparand::Input(input) => self.input(input),
        }
    }

    /// Errors if an input is not a valid Map key.
    fn key(&self, key: &PatternKey) -> Result<Cow<'_, MapKey>, FrostError> {
        match key {
            PatternKey::Constant(key) => {
                Ok(Cow::Borrowed(&self.function.key_constants[index(*key)]))
            }
            PatternKey::Input(input) => {
                MapKey::try_from(self.input(input).into_owned()).map(Cow::Owned)
            }
        }
    }

    fn input(&self, input: &PatternInput) -> Cow<'_, Value> {
        match *input {
            PatternInput::Local(local) => Cow::Borrowed(
                self.slots[index(local)]
                    .as_ref()
                    .expect("IMPOSSIBLE: local value is undefined"),
            ),
            PatternInput::Global(global) => Cow::Borrowed(self.globals.get(index(global))),
            PatternInput::EarlierBinding(local) => self
                .pending
                .iter()
                .rfind(|binding| binding.local == local)
                .expect("IMPOSSIBLE: a pattern reads a binding it has not made")
                .value
                .value(),
        }
    }
}

/// A pattern's index into its function's slots or one of its pools.
fn index(operand: u32) -> usize {
    from_operand(u64::from(operand))
}
