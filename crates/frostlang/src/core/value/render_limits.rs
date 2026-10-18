/// How much of a value
/// [`Value::to_frost_string_within`](crate::Value::to_frost_string_within) and its kin
/// render.
///
/// Rendering a value uses native stack in proportion to how deeply it nests, and a
/// value holding the same contents in many places renders them in full at each, so its
/// text can be vastly larger than the value. These limits bound both. A container
/// nested past [`max_depth`](Self::max_depth) renders as `[...]` or `{...}`. Once the
/// text reaches [`max_len`](Self::max_len), the rest of each container renders as `...`.
/// A String, Bytes, or other value holding no others is never cut: it renders whole or
/// not at all.
///
/// The defaults, [`new`](Self::new), keep any value's text within reason without
/// touching values a script would reasonably render.
///
/// ```
/// use frostlang::{RenderLimits, Value};
///
/// let value = Value::from(vec![
///     Value::from(1),
///     Value::from(vec![Value::from(2), Value::from(vec![Value::from(3)])]),
/// ]);
/// let shallow = RenderLimits::new().with_max_depth(Some(1));
/// assert_eq!(value.to_frost_string_within(shallow), "[ 1, [ 2, [...] ] ]");
///
/// // The text reaches 3 bytes, `[ 1`, before the second element.
/// let short = RenderLimits::new().with_max_len(Some(3));
/// assert_eq!(value.to_frost_string_within(short), "[ 1, ... ]");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RenderLimits {
    /// How many levels deep containers render, the value rendered being at depth 0. A
    /// container at a greater depth renders as `[...]` or `{...}`, though an empty one
    /// renders as itself. `None` leaves the depth unbounded, and then a deep enough
    /// value overflows the stack, aborting the process.
    pub max_depth: Option<usize>,

    /// How long, in bytes, the text may grow before the rest of each container renders
    /// as `...`. An element begins only while the text is shorter than this, so the text
    /// can pass it by what that element holds that is never cut (one String, say, or a
    /// Map entry's key and value), and by the `...`, separators, closing brackets, and
    /// indentation after it. `None` leaves the length unbounded.
    pub max_len: Option<usize>,
}

impl RenderLimits {
    /// Limits for rendering a value as text in its own right: a depth of 100 and a
    /// length of 16 MiB.
    pub const fn new() -> Self {
        Self {
            max_depth: Some(100),
            max_len: Some(16 << 20),
        }
    }

    /// Limits for rendering a value within a message, such as an error's: a depth of
    /// 100 and a length of 1 KiB, past which only what is never cut adds to it.
    pub const fn brief() -> Self {
        Self {
            max_depth: Some(100),
            max_len: Some(1 << 10),
        }
    }

    /// No limits: depth is bounded only by the stack, and length only by memory.
    pub const UNLIMITED: Self = Self {
        max_depth: None,
        max_len: None,
    };

    /// These limits with [`max_depth`](Self::max_depth) set to `limit`.
    #[must_use]
    pub const fn with_max_depth(mut self, limit: Option<usize>) -> Self {
        self.max_depth = limit;
        self
    }

    /// These limits with [`max_len`](Self::max_len) set to `limit`.
    #[must_use]
    pub const fn with_max_len(mut self, limit: Option<usize>) -> Self {
        self.max_len = limit;
        self
    }
}

/// [`RenderLimits::new`].
impl Default for RenderLimits {
    fn default() -> Self {
        Self::new()
    }
}
