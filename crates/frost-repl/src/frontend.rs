//! What a [`Repl`](crate::Repl) reads its input from and shows its outcomes on.

use std::io::{self, BufRead, Stderr, StdinLock, Stdout, Write};

use frost_runtime::Value;

use crate::{ReplError, complete_segment};

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

/// The simplest [`Frontend`]: it reads lines, each after writing a prompt.
///
/// A segment continues onto more lines until [`complete_segment`] completes
/// it, each further line after a continuation prompt. If input ends first,
/// the unfinished segment is run as it is, for the compiler to report.
///
/// Each value other than Null is written pretty-printed, as by
/// [`Value::to_pretty_string`], after the prompts; a failure is written
/// separately, as [`ReplError`]'s [`Display`](std::fmt::Display) shows it.
pub struct LineFrontend<R, W, E> {
    lines: R,
    output: W,
    errors: E,
    prompt: String,
    continuation_prompt: String,
}

impl LineFrontend<StdinLock<'static>, Stdout, Stderr> {
    /// Lines from standard input; prompts and values on standard output, and
    /// failures on standard error.
    pub fn stdin() -> Self {
        Self::new(io::stdin().lock(), io::stdout(), io::stderr())
    }
}

impl<R: BufRead, W: Write, E: Write> LineFrontend<R, W, E> {
    /// Lines from `lines`, prompted for with `> `, and with `. ` as a segment
    /// continues; prompts and values on `output`, and failures on `errors`.
    pub fn new(lines: R, output: W, errors: E) -> Self {
        Self {
            lines,
            output,
            errors,
            prompt: "> ".to_string(),
            continuation_prompt: ". ".to_string(),
        }
    }

    /// Set the prompt written before the first line of each segment.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Set the prompt written before each further line of a segment.
    pub fn with_continuation_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.continuation_prompt = prompt.into();
        self
    }

    /// The next line, without its line ending, after writing the prompt for
    /// the first line of a segment or a further one. `None` once input ends.
    fn read_line(&mut self, continuing: bool) -> io::Result<Option<String>> {
        let prompt = if continuing {
            &self.continuation_prompt
        } else {
            &self.prompt
        };
        write!(self.output, "{prompt}")?;
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
}

impl<R: BufRead, W: Write, E: Write> Frontend for LineFrontend<R, W, E> {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        let Some(mut source) = self.read_line(false)? else {
            return Ok(None);
        };
        loop {
            if let Some(segment) = complete_segment(&source) {
                return Ok(Some(segment));
            }
            match self.read_line(true)? {
                Some(line) => {
                    source.push('\n');
                    source.push_str(&line);
                }
                None => return Ok(Some(source)),
            }
        }
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
