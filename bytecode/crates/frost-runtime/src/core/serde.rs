mod de;
mod ser;

pub use de::from_value;
pub use ser::to_value;

/// The newtype-struct name that `Value`'s own `Serialize`/`Deserialize` use to recognize
/// each other's (de)serializer and pass a `Value` across whole, Functions and Opaques
/// included. A `$` cannot appear in a Rust identifier, so it never collides with a derived
/// struct or field name, and a foreign (de)serializer treats it as an ordinary transparent
/// newtype. The bridge trusts host impls not to hand-forge this exact name; if one did, the
/// (de)serializer's guard still clears the thread-locals, so the worst case is a recoverable
/// error rather than dirty state on the thread.
pub(crate) const VALUE_NEWTYPE_TOKEN: &str = "$frost::core::Value";
