//! [`TypeSet`]: a set of [`FrostType`]s, and the named category sets.

use std::fmt;
use std::ops::{BitAnd, BitOr, Sub};

use enumset::{EnumSet, EnumSetType, enum_set, enum_set_union};

use crate::FrostType;

/// A set of [`FrostType`]s, such as the types a parameter accepts.
///
/// Combine types and sets with `|`, intersect with `&`, and remove with `-`.
/// Operators are unavailable in const context; there, combine with
/// [`FrostType::or`], [`TypeSet::or`], and [`TypeSet::union`], or use a named
/// category constant such as [`FrostType::NUMERIC`]:
///
/// ```
/// use frostlang::{FrostType, TypeSet};
///
/// const FLAT_OR_ARRAY: TypeSet = FrostType::String
///     .or(FrostType::Bytes)
///     .or(FrostType::Array);
/// const NUMBER_OR_NULL: TypeSet = FrostType::NUMERIC.union(FrostType::NULL);
///
/// assert_eq!(FLAT_OR_ARRAY, FrostType::FLAT | FrostType::Array);
/// assert!(NUMBER_OR_NULL.contains(FrostType::Null));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TypeSet(EnumSet<Member>);

/// [`FrostType`]'s private mirror, the element type the set is built on. It
/// keeps `enumset`'s derived trait impls off the public `FrostType`, so an
/// `enumset` upgrade is never a breaking change to Frost's API.
// Declared in `FrostType`'s order, which is the order `TypeSet::iter` promises.
// The explicit repr unlocks `EnumSet::as_repr`, needed by the const
// `TypeSet::is_empty`.
#[derive(EnumSetType)]
#[enumset(repr = "u16")]
enum Member {
    Null,
    Bool,
    Int,
    Float,
    String,
    Bytes,
    Array,
    Map,
    Function,
    Opaque,
}

impl Member {
    const fn of(ty: FrostType) -> Member {
        match ty {
            FrostType::Null => Member::Null,
            FrostType::Bool => Member::Bool,
            FrostType::Int => Member::Int,
            FrostType::Float => Member::Float,
            FrostType::String => Member::String,
            FrostType::Bytes => Member::Bytes,
            FrostType::Array => Member::Array,
            FrostType::Map => Member::Map,
            FrostType::Function => Member::Function,
            FrostType::Opaque => Member::Opaque,
        }
    }

    const fn frost_type(self) -> FrostType {
        match self {
            Member::Null => FrostType::Null,
            Member::Bool => FrostType::Bool,
            Member::Int => FrostType::Int,
            Member::Float => FrostType::Float,
            Member::String => FrostType::String,
            Member::Bytes => FrostType::Bytes,
            Member::Array => FrostType::Array,
            Member::Map => FrostType::Map,
            Member::Function => FrostType::Function,
            Member::Opaque => FrostType::Opaque,
        }
    }
}

impl TypeSet {
    /// The set of no types.
    pub const EMPTY: TypeSet = TypeSet(EnumSet::empty());

    const fn only(ty: FrostType) -> TypeSet {
        let member = Member::of(ty);
        TypeSet(enum_set!(member))
    }

    /// This set with `ty` added.
    pub const fn or(self, ty: FrostType) -> TypeSet {
        self.union(TypeSet::only(ty))
    }

    /// The types in either set.
    pub const fn union(self, other: TypeSet) -> TypeSet {
        let (these, those) = (self.0, other.0);
        TypeSet(enum_set_union!(these, those))
    }

    /// Whether the set has no types.
    pub const fn is_empty(self) -> bool {
        // Via the raw repr: `EnumSet::is_empty` is not a const fn.
        self.0.as_repr() == 0
    }

    /// Whether `ty` is in the set.
    pub fn contains(self, ty: FrostType) -> bool {
        self.0.contains(Member::of(ty))
    }

    /// The number of types in the set.
    pub fn len(self) -> usize {
        self.0.len()
    }

    /// The types in the set, in the order [`FrostType`] declares them.
    pub fn iter(self) -> impl Iterator<Item = FrostType> {
        self.0.iter().map(Member::frost_type)
    }
}

/// Named type sets.
impl FrostType {
    /// The set containing only `Null`.
    pub const NULL: TypeSet = TypeSet::only(FrostType::Null);
    /// The set containing only `Bool`.
    pub const BOOL: TypeSet = TypeSet::only(FrostType::Bool);
    /// The set containing only `Int`.
    pub const INT: TypeSet = TypeSet::only(FrostType::Int);
    /// The set containing only `Float`.
    pub const FLOAT: TypeSet = TypeSet::only(FrostType::Float);
    /// The set containing only `String`.
    pub const STRING: TypeSet = TypeSet::only(FrostType::String);
    /// The set containing only `Bytes`.
    pub const BYTES: TypeSet = TypeSet::only(FrostType::Bytes);
    /// The set containing only `Array`.
    pub const ARRAY: TypeSet = TypeSet::only(FrostType::Array);
    /// The set containing only `Map`.
    pub const MAP: TypeSet = TypeSet::only(FrostType::Map);
    /// The set containing only `Function`.
    pub const FUNCTION: TypeSet = TypeSet::only(FrostType::Function);
    /// The set containing only `Opaque`.
    pub const OPAQUE: TypeSet = TypeSet::only(FrostType::Opaque);

    /// Every type: the set that accepts any value.
    pub const ANY: TypeSet = TypeSet(EnumSet::all());

    /// Int or Float.
    pub const NUMERIC: TypeSet = FrostType::Int.or(FrostType::Float);

    /// Null, Bool, Int, Float, String, or Bytes.
    /// All but Null may be Map keys.
    pub const PRIMITIVE: TypeSet = FrostType::Null
        .or(FrostType::Bool)
        .or(FrostType::Int)
        .or(FrostType::Float)
        .or(FrostType::String)
        .or(FrostType::Bytes);

    /// Array or Map.
    pub const STRUCTURED: TypeSet = FrostType::Array.or(FrostType::Map);

    /// String or Bytes: the flat sequence types.
    pub const FLAT: TypeSet = FrostType::String.or(FrostType::Bytes);

    /// Every type except Null.
    pub const NONNULL: TypeSet = FrostType::Bool
        .or(FrostType::Int)
        .or(FrostType::Float)
        .or(FrostType::String)
        .or(FrostType::Bytes)
        .or(FrostType::Array)
        .or(FrostType::Map)
        .or(FrostType::Function)
        .or(FrostType::Opaque);

    /// The set of this type and `other`.
    pub const fn or(self, other: FrostType) -> TypeSet {
        TypeSet::only(self).or(other)
    }
}

impl From<FrostType> for TypeSet {
    fn from(ty: FrostType) -> Self {
        TypeSet::only(ty)
    }
}

impl FromIterator<FrostType> for TypeSet {
    fn from_iter<I: IntoIterator<Item = FrostType>>(types: I) -> Self {
        TypeSet(types.into_iter().map(Member::of).collect())
    }
}

impl fmt::Debug for TypeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Into<TypeSet>> BitOr<T> for TypeSet {
    type Output = TypeSet;

    fn bitor(self, other: T) -> TypeSet {
        TypeSet(self.0 | other.into().0)
    }
}

impl<T: Into<TypeSet>> BitOr<T> for FrostType {
    type Output = TypeSet;

    fn bitor(self, other: T) -> TypeSet {
        TypeSet::only(self) | other
    }
}

impl<T: Into<TypeSet>> BitAnd<T> for TypeSet {
    type Output = TypeSet;

    fn bitand(self, other: T) -> TypeSet {
        TypeSet(self.0 & other.into().0)
    }
}

impl<T: Into<TypeSet>> Sub<T> for TypeSet {
    type Output = TypeSet;

    fn sub(self, other: T) -> TypeSet {
        TypeSet(self.0 - other.into().0)
    }
}
