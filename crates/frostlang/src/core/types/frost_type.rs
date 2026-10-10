//! [`FrostType`]: the type of a Frost value.

/// The possible types of a Frost Value.
///
/// Sets of types are [`TypeSet`](crate::TypeSet)s: build one with `|`
/// (`FrostType::Int | FrostType::Float`) or use a named category constant.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum FrostType {
    /// The absence of a value.
    Null,
    /// `true` or `false`.
    Bool,
    /// A 64-bit signed integer.
    Int,
    /// A finite 64-bit float; see [`FrostFloat`](crate::FrostFloat).
    Float,
    /// UTF-8 text.
    String,
    /// A byte sequence.
    Bytes,
    /// An ordered sequence of values.
    Array,
    /// A mapping from [`MapKey`](crate::MapKey) keys to values.
    Map,
    /// A callable: native or defined in Frost.
    Function,
    /// Host data; see [`FrostOpaque`](crate::FrostOpaque).
    Opaque,
}

impl FrostType {
    /// The type's name as Frost spells it, e.g. `"Int"`.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Float => "Float",
            Self::String => "String",
            Self::Bytes => "Bytes",
            Self::Array => "Array",
            Self::Map => "Map",
            Self::Function => "Function",
            Self::Opaque => "Opaque",
        }
    }
}
