//! What a [`Repl`](crate::Repl) reads its input from and shows its outcomes on.

use std::io::{self, BufRead, Stderr, StdinLock, Stdout, Write};

use frost_runtime::Value;

use crate::ReplError;

/// Where a [`Repl`](crate::Repl) session happens: it supplies each input and
/// shows what became of it. A terminal, a notebook, or an in-application
/// console each present a session in their own way.
pub trait Frontend {
    /// The next segment, or `None` once input has ended.
    ///
    /// A segment is one complete piece of source for the REPL to run, however
    /// many lines it spans. Where a segment ends is the frontend's decision:
    /// the REPL runs each one exactly as given.
    fn read_segment(&mut self) -> io::Result<Option<String>>;

    /// Show the outcome of the segment last read: its value, Null included, or
    /// why it failed.
    fn render(&mut self, outcome: Result<&Value, &ReplError>) -> io::Result<()>;
}

/// The simplest [`Frontend`]: each line is a segment, read after writing a
/// prompt.
///
/// Each value other than Null is written pretty-printed, as by
/// [`Value::to_pretty_string`], after the prompts; a failure is written
/// separately, as [`ReplError`]'s [`Display`](std::fmt::Display) shows it.
pub struct LineFrontend<R, W, E> {
    lines: R,
    output: W,
    errors: E,
    prompt: String,
}

impl LineFrontend<StdinLock<'static>, Stdout, Stderr> {
    /// Lines from standard input; prompts and values on standard output, and
    /// failures on standard error.
    pub fn stdin() -> Self {
        Self::new(io::stdin().lock(), io::stdout(), io::stderr())
    }
}

impl<R: BufRead, W: Write, E: Write> LineFrontend<R, W, E> {
    /// Lines from `lines`, each prompted for with `> `; prompts and values on
    /// `output`, and failures on `errors`.
    pub fn new(lines: R, output: W, errors: E) -> Self {
        Self {
            lines,
            output,
            errors,
            prompt: "> ".to_string(),
        }
    }

    /// Set the prompt written before each line is read.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }
}

impl<R: BufRead, W: Write, E: Write> Frontend for LineFrontend<R, W, E> {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        write!(self.output, "{}", self.prompt)?;
        self.output.flush()?;
        let mut line = String::new();
        if self.lines.read_line(&mut line)? == 0 {
            // End the prompt's line, so whatever follows starts on its own.
            writeln!(self.output)?;
            return Ok(None);
        }
        let line = line.strip_suffix('\n').unwrap_or(&line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        Ok(Some(line.to_string()))
    }

    fn render(&mut self, outcome: Result<&Value, &ReplError>) -> io::Result<()> {
        match outcome {
            Ok(Value::Null) => Ok(()),
            Ok(value) => {
                writeln!(self.output, "{}", value.to_pretty_string())?;
                self.output.flush()
            }
            Err(error) => {
                writeln!(self.errors, "{error}")?;
                self.errors.flush()
            }
        }
    }
}
