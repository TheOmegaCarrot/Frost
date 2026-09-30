//! Where a [`Repl`](crate::Repl) reads its input.

use std::io::{self, BufRead, StdinLock, Stdout, Write};

/// A source of input for a [`Repl`](crate::Repl), read one segment at a time.
///
/// A segment is one complete piece of source for the REPL to run, however many
/// lines it spans. Where a segment ends is the input's decision: the REPL runs
/// each one exactly as given.
pub trait ReplInput {
    /// The next segment, or `None` once input has ended.
    fn read_segment(&mut self) -> io::Result<Option<String>>;
}

/// The simplest [`ReplInput`]: each line is a segment, read after writing a
/// prompt.
pub struct LineInput<R, W> {
    lines: R,
    prompts: W,
    prompt: String,
}

impl LineInput<StdinLock<'static>, Stdout> {
    /// Lines from standard input, prompted on standard output.
    pub fn stdin() -> Self {
        Self::new(io::stdin().lock(), io::stdout())
    }
}

impl<R: BufRead, W: Write> LineInput<R, W> {
    /// Lines from `lines`, each prompted for on `prompts` with `> `.
    pub fn new(lines: R, prompts: W) -> Self {
        Self {
            lines,
            prompts,
            prompt: "> ".to_string(),
        }
    }

    /// Set the prompt written before each line is read.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }
}

impl<R: BufRead, W: Write> ReplInput for LineInput<R, W> {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        write!(self.prompts, "{}", self.prompt)?;
        self.prompts.flush()?;
        let mut line = String::new();
        if self.lines.read_line(&mut line)? == 0 {
            // End the prompt's line, so whatever follows starts on its own.
            writeln!(self.prompts)?;
            return Ok(None);
        }
        let line = line.strip_suffix('\n').unwrap_or(&line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        Ok(Some(line.to_string()))
    }
}
