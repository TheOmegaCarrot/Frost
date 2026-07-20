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

/// The type categories exposed within the language, as named sets.
impl FrostType {
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
