mod de;
mod ser;

pub use de::from_value;
pub(crate) use de::{accepted_types, deserialize_of_types};
pub use ser::to_value;

use std::fmt::{self, Display};

use crate::core::path::{PathSegment, write_path};
use crate::core::{SpecialFloat, Value};

/// Where in a value an error happened, which an error gains as it passes back
/// out through each enclosing level.
#[derive(Debug, Default)]
struct ErrorPath {
    /// Whether the error was in a Map key, rather than a value.
    in_key: bool,
    /// The Map keys and Array indices leading to the error, innermost first.
    segments: Vec<PathSegment>,
}

impl ErrorPath {
    /// Records that the error happened within `segment` of the enclosing value.
    fn push(&mut self, segment: PathSegment) {
        self.segments.push(segment);
    }

    /// Records that the error happened in a key of the enclosing Map. Any place
    /// within the key is dropped: no Frost index reaches inside a key.
    fn mark_in_key(&mut self) {
        self.in_key = true;
        self.segments.clear();
    }
}

/// Renders as a suffix for the error's message, such as `` (at `a.b`)``, the
/// path written as [`write_path`] writes it. At the top level it renders as
/// nothing.
impl Display for ErrorPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.in_key, self.segments.is_empty()) {
            (false, true) => return Ok(()),
            (true, true) => return f.write_str(" (at a key)"),
            (true, false) => f.write_str(" (at a key of `")?,
            (false, false) => f.write_str(" (at `")?,
        }
        write_path(f, self.segments.iter().rev())?;
        f.write_str("`)")
    }
}

/// The float a [`SpecialFloat`] Opaque leaves serde as, or `None` for any other value.
fn special_float(value: &Value) -> Option<f64> {
    value.downcast_opaque::<SpecialFloat>().map(|s| s.get())
}

/// `int` as an Int, or the message saying no Int can hold it.
fn int_value<I: TryInto<i64> + Display + Copy>(int: I) -> Result<Value, String> {
    int.try_into()
        .map(Value::Int)
        .map_err(|_| format!("integer {int} is beyond the Int range"))
}

/// The newtype-struct name that `Value`'s own `Serialize`/`Deserialize` use to recognize
/// each other's (de)serializer and pass a `Value` across whole, Functions and Opaques
/// included. A `$` cannot appear in a Rust identifier, so it never collides with a derived
/// struct or field name, and a foreign (de)serializer treats it as an ordinary transparent
/// newtype. The bridge trusts host impls not to hand-forge this exact name; if one did, the
/// (de)serializer's guard still clears the thread-locals.
pub(crate) const VALUE_NEWTYPE_TOKEN: &str = "$frost::core::Value";

/// The newtype-struct name a [`Nullable`](crate::Nullable) (de)serializes through, so that
/// our own (de)serializer can tell its `None` (Null) from an `Option`'s (absent). As with
/// [`VALUE_NEWTYPE_TOKEN`], the `$` keeps it from colliding, and a foreign (de)serializer
/// sees an ordinary newtype.
pub(crate) const NULLABLE_NEWTYPE_TOKEN: &str = "$frost::core::Nullable";

/// The newtype-struct name [`deserialize_of_types`] deserializes through. Our own
/// deserializer passes the value across whole, as for [`VALUE_NEWTYPE_TOKEN`]; the
/// probe reports the types that call asks for.
pub(crate) const TYPED_NEWTYPE_TOKEN: &str = "$frost::core::Typed";
