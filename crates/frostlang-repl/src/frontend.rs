//! What a [`Repl`](crate::Repl) reads its input from and shows its outcomes on.

use std::io::{self, BufRead, IsTerminal, Stderr, StdinLock, Stdout, Write};

use frostlang::Value;

use crate::{CancelHandle, Invocation, MetacommandSpec, ReplError, complete_segment};

/// Where a [`Repl`](crate::Repl) session happens: it supplies each input and
/// shows what became of it. A terminal, a notebook, or an in-application
/// console each present a session in their own way.
pub trait Frontend {
    /// The next segment, or `None` once input has ended.
    ///
    /// A segment is one complete piece of source for the REPL to run, or one
    /// [metacommand](crate::Repl#metacommands), however many lines it spans.
    /// Where a segment ends is the frontend's decision: the REPL runs each one
    /// exactly as given.
    fn read_segment(&mut self) -> io::Result<Option<String>>;

    /// Show the outcome of the segment last read: its value, Null included, or
    /// why it failed.
    fn render(&mut self, outcome: Result<&Value, &ReplError>) -> io::Result<()>;

    /// Show text one of the REPL's metacommands produced, which has no
    /// trailing newline. It carries ANSI styling only if
    /// [`ansi_styling`](Self::ansi_styling) says it may.
    fn render_text(&mut self, text: &str) -> io::Result<()>;

    /// Whether text the REPL gives [`render_text`](Self::render_text) may
    /// carry ANSI styling. By default, it may not.
    fn ansi_styling(&self) -> bool {
        false
    }

    /// The metacommands this frontend adds to the REPL's own. The REPL asks
    /// once, as a session starts, and refuses the session if one has an
    /// invalid name or a name already taken (see [`Repl::run`](crate::Repl::run)).
    /// By default, there are none.
    fn metacommands(&self) -> Vec<MetacommandSpec> {
        Vec::new()
    }

    /// Run `invocation`, of one of the metacommands
    /// [`metacommands`](Self::metacommands) lists. The frontend shows whatever
    /// comes of it itself; it can show a failure as the REPL shows its own, as
    /// a [`ReplError::Metacommand`]. An error ends the session.
    fn metacommand(&mut self, invocation: &Invocation) -> io::Result<()> {
        Err(io::Error::other(format!(
            "this frontend has no metacommand `:{}`",
            invocation.name()
        )))
    }

    /// Take the [`CancelHandle`] that cancels the input being evaluated,
    /// for a frontend that lets its user interrupt one, as Ctrl-C at a terminal does.
    /// [`Repl::run`](crate::Repl::run) gives it as a session starts.
    /// By default, it is dropped.
    fn set_cancel_handle(&mut self, handle: CancelHandle) {
        let _ = handle;
    }
}

/// `value` as the REPL's own frontends show an input's value: a String quoted
/// and escaped, so that `"null"` is not mistaken for `null`, and anything else
/// pretty-printed, as by [`Value::to_pretty_string`].
pub fn render_value(value: &Value) -> String {
    match value {
        // Compact and pretty agree on a lone String.
        Value::String(_) => value.to_debug_string(),
        _ => value.to_pretty_string(),
    }
}

/// The simplest [`Frontend`]: it reads lines, each after writing a prompt.
///
/// A segment continues onto more lines until [`complete_segment`] completes
/// it, each further line after a continuation prompt. If input ends first,
/// the unfinished segment is run as it is, for the compiler to report. Once
/// input ends, a newline ends the last prompt's line, unless that prompt is
/// empty.
///
/// Each value other than Null is written as [`render_value`] renders it,
/// after the prompts, as is a metacommand's text, unstyled. A failure is written separately, as [`ReplError`]'s
/// [`Display`](std::fmt::Display) shows it.
pub struct LineFrontend<R, W, E> {
    lines: R,
    output: W,
    errors: E,
    prompt: String,
    continuation_prompt: String,
}

impl LineFrontend<StdinLock<'static>, Stdout, Stderr> {
    /// Lines from standard input; prompts and values on standard output, and
    /// failures on standard error. If standard input is not a terminal, there
    /// is no one to prompt, so both prompts are empty.
    pub fn stdin() -> Self {
        let frontend = Self::new(io::stdin().lock(), io::stdout(), io::stderr());
        if io::stdin().is_terminal() {
            frontend
        } else {
            frontend.with_prompt("").with_continuation_prompt("")
        }
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
            if !prompt.is_empty() {
                writeln!(self.output)?;
            }
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
                writeln!(self.output, "{}", render_value(value))?;
                self.output.flush()
            }
            Err(error) => {
                writeln!(self.errors, "{error}")?;
                self.errors.flush()
            }
        }
    }

    fn render_text(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.output, "{text}")?;
        self.output.flush()
    }
}
