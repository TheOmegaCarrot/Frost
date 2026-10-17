use std::num::NonZeroU64;

/// How deeply source may nest before [`parse_program_with_limits`](crate::parse_program_with_limits)
/// rejects it with a [`ParseError`](crate::ParseError).
///
/// Parsing, and anything that walks the resulting [`ast`](crate::ast), uses native stack in
/// proportion to how deeply the source nests. Without limits, deeply nested source overflows
/// the stack, which aborts the process. The defaults, [`new`](Self::new), suit a thread with
/// Rust's default stack size (2 MiB) and are lower in a debug build, whose stack frames are
/// larger. A host whose parse runs on a smaller stack should lower them; one with a larger
/// stack may raise them.
///
/// ```
/// use std::num::NonZeroU64;
/// use frostlang_parse::ParseLimits;
///
/// const LIMITS: ParseLimits = ParseLimits::new().with_max_ast_depth(NonZeroU64::new(100));
/// assert_eq!(LIMITS.max_ast_depth, NonZeroU64::new(100));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseLimits {
    /// How deeply the parser may recurse: each expression or pattern inside another is one
    /// level deeper, as is each pair of parentheses. This bounds the parser's own use of the
    /// stack. `None` leaves it unbounded.
    pub max_recursion_depth: Option<NonZeroU64>,

    /// How deep the syntax tree may grow: each expression or pattern inside another is one
    /// level deeper, and so is each link of a chain such as `a + b + c` or `a.b.c`, which
    /// nests every link in the next. Parentheses add no level. This bounds the stack used by
    /// whatever walks the tree, such as a compiler, and by freeing it.
    /// `None` leaves it unbounded.
    pub max_ast_depth: Option<NonZeroU64>,
}

impl ParseLimits {
    /// Limits suited to a thread with Rust's default stack size, leaving room for the frames
    /// of whatever calls the parser: a recursion depth of 128 and an AST depth of 512, or 96
    /// and 256 in a debug build.
    pub const fn new() -> Self {
        if cfg!(debug_assertions) {
            Self {
                max_recursion_depth: NonZeroU64::new(96),
                max_ast_depth: NonZeroU64::new(256),
            }
        } else {
            Self {
                max_recursion_depth: NonZeroU64::new(128),
                max_ast_depth: NonZeroU64::new(512),
            }
        }
    }

    /// No limits: nesting is bounded only by the stack.
    pub const UNLIMITED: Self = Self {
        max_recursion_depth: None,
        max_ast_depth: None,
    };

    /// These limits with [`max_recursion_depth`](Self::max_recursion_depth) set to `limit`.
    #[must_use]
    pub const fn with_max_recursion_depth(mut self, limit: Option<NonZeroU64>) -> Self {
        self.max_recursion_depth = limit;
        self
    }

    /// These limits with [`max_ast_depth`](Self::max_ast_depth) set to `limit`.
    #[must_use]
    pub const fn with_max_ast_depth(mut self, limit: Option<NonZeroU64>) -> Self {
        self.max_ast_depth = limit;
        self
    }
}

/// [`ParseLimits::new`].
impl Default for ParseLimits {
    fn default() -> Self {
        Self::new()
    }
}
