//! Builders of [`CompiledPattern`]s, for the tests that write patterns by hand.

use frostlang::TypeSet;
use frostlang::bytecode::{
    ArrayPatternLength, CompiledPattern, PatternComparand, PatternEntry, PatternInput, PatternKey,
};

/// `local`: matches any value, binding it to `local`.
pub(crate) fn bind(local: u32) -> CompiledPattern {
    CompiledPattern::Binding {
        types: None,
        local: Some(local),
    }
}

/// `_`: matches any value, binding nothing.
pub(crate) fn discard() -> CompiledPattern {
    CompiledPattern::Binding {
        types: None,
        local: None,
    }
}

/// `local is types`, or `_ is types` without a local.
pub(crate) fn typed(types: TypeSet, local: Option<u32>) -> CompiledPattern {
    CompiledPattern::Binding {
        types: Some(types),
        local,
    }
}

/// A value pattern comparing against `comparand`.
pub(crate) fn value(comparand: PatternComparand) -> CompiledPattern {
    CompiledPattern::Value(comparand)
}

/// A value pattern comparing against the Int `int`.
pub(crate) fn int(int: i64) -> CompiledPattern {
    value(PatternComparand::Int(int))
}

/// A value pattern comparing against `input`: a pin.
pub(crate) fn pin(input: PatternInput) -> CompiledPattern {
    value(PatternComparand::Input(input))
}

/// `[elements...]`: an Array of exactly as many elements.
pub(crate) fn exact(elements: impl Into<Box<[CompiledPattern]>>) -> CompiledPattern {
    CompiledPattern::Array {
        elements: elements.into(),
        length: ArrayPatternLength::Exact,
        whole: None,
    }
}

/// `[elements..., ...rest]`: an Array of at least as many elements, the
/// others bound to `rest`, if any.
pub(crate) fn at_least(
    elements: impl Into<Box<[CompiledPattern]>>,
    rest: Option<u32>,
) -> CompiledPattern {
    CompiledPattern::Array {
        elements: elements.into(),
        length: ArrayPatternLength::AtLeast { rest },
        whole: None,
    }
}

/// `array as whole`: the Array pattern `array`, from [`exact`] or
/// [`at_least`], binding the whole Array to local `whole`.
pub(crate) fn array_as(array: CompiledPattern, whole: u32) -> CompiledPattern {
    let CompiledPattern::Array {
        elements, length, ..
    } = array
    else {
        panic!("`array_as` takes an Array pattern, not {array:?}");
    };
    CompiledPattern::Array {
        elements,
        length,
        whole: Some(whole),
    }
}

/// `{key: pattern, ...} as whole`, every key required, with `whole` if any.
pub(crate) fn map(
    entries: impl IntoIterator<Item = (PatternKey, CompiledPattern)>,
    whole: Option<u32>,
) -> CompiledPattern {
    map_of(
        entries
            .into_iter()
            .map(|(key, pattern)| required(key, pattern)),
        whole,
    )
}

/// [`map`], of entries from [`required`] and [`optional`].
pub(crate) fn map_of(
    entries: impl IntoIterator<Item = PatternEntry>,
    whole: Option<u32>,
) -> CompiledPattern {
    CompiledPattern::Map {
        entries: entries.into_iter().collect(),
        whole,
    }
}

/// `key: pattern`, an entry of a [`map_of`].
pub(crate) fn required(key: PatternKey, pattern: CompiledPattern) -> PatternEntry {
    PatternEntry {
        key,
        optional: false,
        pattern,
    }
}

/// `key?: pattern`, an entry of a [`map_of`].
pub(crate) fn optional(key: PatternKey, pattern: CompiledPattern) -> PatternEntry {
    PatternEntry {
        key,
        optional: true,
        pattern,
    }
}

/// Constant key `index` of the function's key pool.
pub(crate) fn key(index: u32) -> PatternKey {
    PatternKey::Constant(index)
}

/// `[input]`: a key taken from `input`.
pub(crate) fn input_key(input: PatternInput) -> PatternKey {
    PatternKey::Input(input)
}

/// `p1 | p2 | ...`
pub(crate) fn either(branches: impl Into<Box<[CompiledPattern]>>) -> CompiledPattern {
    CompiledPattern::Alternative(branches.into())
}
