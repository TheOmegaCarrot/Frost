//! Debug info: where a compiled function's instructions came from in source,
//! and the stack height each expects.

use std::sync::Arc;

/// Debug info for a [`CompiledFunction`](crate::bytecode::CompiledFunction)'s code;
/// [`CompiledProgram::strip`](crate::CompiledProgram::strip) says what it is for.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DebugInfo {
    /// The entry for each instruction, by its index in the function's code.
    /// The compiler gives every instruction a corresponding per-opcode info.
    pub ops: Vec<OpDebugInfo>,
    /// The line table of the function's source, to translate a byte offset to a [`SourceLocation`].
    // An `Arc` because every function from one compilation shares it.
    pub lines: Arc<SourceLines>,
}

/// Debug info for one instruction: the source it came from, and the stack height on entry to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpDebugInfo {
    /// The source the instruction is derived from.
    pub span: Span,
    /// The operand stack height on entry to the instruction, counting the callee and its arguments
    /// as the [calling convention](crate::bytecode::Bytecode#calling-convention) lays them out.
    /// `None` when it is not statically known,
    /// which is only on entry to [`RewindToMark`](crate::bytecode::Bytecode::RewindToMark).
    pub stack_height: Option<u32>,
}

/// A range of source text, as byte offsets: from `start` up to but not including `end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Span {
    /// The offset where the span starts.
    pub start: u32,
    /// The offset just past the span's last byte.
    pub end: u32,
}

impl Span {
    /// The smallest span covering both `self` and `other`, including any gap between them.
    #[must_use]
    pub fn filled_union(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// The start of each line of one source text, for translating byte offsets to [`SourceLocation`]s.
///
/// Offsets are `u32`, so in a source over 4 GiB, a location past the first 4 GiB is inexact.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceLines {
    // The offset of each `\n` within the first 4 GiB, ascending.
    #[serde(deserialize_with = "ascending_offsets")]
    newlines: Vec<u32>,
}

impl SourceLines {
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
    /// A line's terminating `\n` is on that line.
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

/// Deserialize newline offsets, which an image from this same version always has ascending.
fn ascending_offsets<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u32>, D::Error> {
    let newlines: Vec<u32> = serde::Deserialize::deserialize(deserializer)?;
    debug_assert!(
        newlines.is_sorted(),
        "a line table's newline offsets must ascend"
    );
    Ok(newlines)
}

/// A position in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceLocation {
    line: u32,
    column: u32,
}

impl SourceLocation {
    /// The line, counting from 1.
    pub const fn line(self) -> u32 {
        self.line
    }

    /// The byte offset within the line, counting from 1.
    /// A byte, not a character: a multi-byte character earlier in the line counts once per byte.
    pub const fn column(self) -> u32 {
        self.column
    }
}
