use std::fmt::{self, Display, Formatter};

/// The start of each line of one source text, for translating byte offsets,
/// such as a [`SourceSpan`](crate::ast::SourceSpan)'s, to [`SourceLocation`]s.
///
/// Offsets are `u32`, so in a source over 4 GiB, a location past the first
/// 4 GiB is inexact.
// TODO: Deduplicate with the VM's `SourceLines`, which this mirrors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    // The offset of each `\n` within the first 4 GiB, ascending.
    newlines: Vec<u32>,
}

impl LineIndex {
    /// The line table of `source`.
    pub fn new(source: &str) -> Self {
        let newlines = source
            .bytes()
            .enumerate()
            .filter(|&(_, byte)| byte == b'\n')
            .map_while(|(offset, _)| u32::try_from(offset).ok())
            .collect();
        Self { newlines }
    }

    /// The line and column of the byte at `offset`.
    ///
    /// A line's terminating `\n` is on that line, so a CRLF ends a line once
    /// and a lone CR ends none, as the parser reads line breaks.
    /// An offset past the end of the source is placed as if the last line went on.
    pub fn location(&self, offset: u32) -> SourceLocation {
        let preceding = self.newlines.partition_point(|&newline| newline < offset);
        let line_start = match preceding {
            0 => 0,
            n => self.newlines[n - 1] + 1,
        };
        // Saturating: only a source over 4 GiB reaches these limits.
        SourceLocation {
            line: u32::try_from(preceding + 1).unwrap_or(u32::MAX),
            column: (offset - line_start).saturating_add(1),
        }
    }
}

/// A position in source text, from [`LineIndex::location`].
///
/// It displays as `line:column`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceLocation {
    line: u32,
    column: u32,
}

impl SourceLocation {
    /// The line, counting from 1.
    pub fn line(self) -> u32 {
        self.line
    }

    /// The byte offset within the line, counting from 1.
    /// A byte, not a character: a multibyte character earlier in the line
    /// counts once per byte.
    pub fn column(self) -> u32 {
        self.column
    }
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}
