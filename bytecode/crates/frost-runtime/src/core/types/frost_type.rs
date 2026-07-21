//! [`FrostType`]: the type of a Frost value, and the named category sets.

use enumset::{EnumSet, EnumSetType, enum_set};

/// The possible types of a Frost Value.
///
/// Sets of types are [`EnumSet<FrostType>`]: build one with `|`
/// (`FrostType::Int | FrostType::Float`) or use a named category constant.
// EnumSetType derives Copy, Clone, PartialEq, and Eq itself.
// The explicit repr unlocks `EnumSet::as_repr`: const-compatible inspection,
// needed for const spec validation.
#[derive(EnumSetType, Debug, serde::Serialize, serde::Deserialize)]
#[enumset(repr = "u16")]
pub enum FrostType {
    Null,
    Bool,
    Int,
    Float,
    String,
    Array,
    Map,
    Function,
    Opaque,
}

/// Named type sets, for building specs and tests in const context (where a single
/// [`FrostType`] can't be widened to an [`EnumSet`] via `.into()`).
impl FrostType {
    /// The set containing only `Null`.
    pub const NULL: EnumSet<FrostType> = enum_set!(FrostType::Null);
    /// The set containing only `Bool`.
    pub const BOOL: EnumSet<FrostType> = enum_set!(FrostType::Bool);
    /// The set containing only `Int`.
    pub const INT: EnumSet<FrostType> = enum_set!(FrostType::Int);
    /// The set containing only `Float`.
    pub const FLOAT: EnumSet<FrostType> = enum_set!(FrostType::Float);
    /// The set containing only `String`.
    pub const STRING: EnumSet<FrostType> = enum_set!(FrostType::String);
    /// The set containing only `Array`.
    pub const ARRAY: EnumSet<FrostType> = enum_set!(FrostType::Array);
    /// The set containing only `Map`.
    pub const MAP: EnumSet<FrostType> = enum_set!(FrostType::Map);
    /// The set containing only `Function`.
    pub const FUNCTION: EnumSet<FrostType> = enum_set!(FrostType::Function);
    /// The set containing only `Opaque`.
    pub const OPAQUE: EnumSet<FrostType> = enum_set!(FrostType::Opaque);

    /// Every type: the set that accepts any value.
    pub const ANY: EnumSet<FrostType> = EnumSet::all();

    /// Int or Float.
    pub const NUMERIC: EnumSet<FrostType> = enum_set!(FrostType::Int | FrostType::Float);

    /// Null, Bool, Int, Float, or String.
    pub const PRIMITIVE: EnumSet<FrostType> = enum_set!(
        FrostType::Null | FrostType::Bool | FrostType::Int | FrostType::Float | FrostType::String
    );

    /// Array or Map.
    pub const STRUCTURED: EnumSet<FrostType> = enum_set!(FrostType::Array | FrostType::Map);

    /// Every type except Null.
    pub const NONNULL: EnumSet<FrostType> = enum_set!(
        FrostType::Bool
            | FrostType::Int
            | FrostType::Float
            | FrostType::String
            | FrostType::Array
            | FrostType::Map
            | FrostType::Function
            | FrostType::Opaque
    );
}

impl FrostType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Float => "Float",
            Self::String => "String",
            Self::Array => "Array",
            Self::Map => "Map",
            Self::Function => "Function",
            Self::Opaque => "Opaque",
        }
    }
}
