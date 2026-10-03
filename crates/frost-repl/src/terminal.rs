//! [`TerminalFrontend`]: line editing, history, and highlighting on a terminal.

use std::borrow::Cow;
use std::io::{self, Write};
use std::path::PathBuf;

use frost_runtime::Value;
use nu_ansi_term::{Color, Style};
use reedline::{
    FileBackedHistory, HISTORY_SIZE, Highlighter, Prompt, PromptEditMode, PromptHistorySearch,
    Reedline, Signal, StyledText, ValidationResult, Validator,
};

use crate::highlight::{self, Class};
use crate::{Frontend, ReplError, complete_segment};

/// A [`Frontend`] for a person at a terminal, with line editing, history, and
/// syntax highlighting.
///
/// A segment continues onto more lines until [`complete_segment`] completes
/// it.
///
/// Ctrl-C discards the segment being typed; Ctrl-D on an empty line ends input.
///
/// Each value other than Null is written pretty-printed, as by
/// [`Value::to_pretty_string`], to standard output; a failure is written to
/// standard error.
pub struct TerminalFrontend {
    editor: Reedline,
    color: bool,
}

impl Default for TerminalFrontend {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalFrontend {
    /// A colored frontend on the terminal, with history kept for this session
    /// only.
    pub fn new() -> Self {
        let editor = Reedline::create()
            .with_validator(Box::new(FrostValidator))
            .with_highlighter(Box::new(FrostHighlighter))
            .use_bracketed_paste(true);
        Self {
            editor,
            color: true,
        }
    }

    /// Set whether to color the prompt, the source being typed, and
    /// diagnostics.
    pub fn with_color(mut self, color: bool) -> Self {
        self.editor = self.editor.with_ansi_colors(color);
        self.color = color;
        self
    }

    /// Keep history in the file at `path`, reading what is there already, so
    /// that it lasts from one session to the next.
    pub fn with_history_file(mut self, path: PathBuf) -> io::Result<Self> {
        let history = FileBackedHistory::with_file(HISTORY_SIZE, path).map_err(io::Error::other)?;
        self.editor = self.editor.with_history(Box::new(history));
        Ok(self)
    }
}

impl Frontend for TerminalFrontend {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        loop {
            match self.editor.read_line(&FrostPrompt)? {
                Signal::Success(segment) if segment.trim().is_empty() => {}
                Signal::Success(segment) => {
                    // The validator submits only what completes.
                    return Ok(Some(complete_segment(&segment).unwrap_or(segment)));
                }
                Signal::CtrlD => return Ok(None),
                // Ctrl-C discards the segment, and anything else asks for no
                // action from us: read another.
                _ => {}
            }
        }
    }

    fn render(&mut self, outcome: Result<&Value, &ReplError>) -> io::Result<()> {
        match outcome {
            Ok(Value::Null) => Ok(()),
            Ok(value) => {
                let mut stdout = io::stdout().lock();
                writeln!(stdout, "{}", value.to_pretty_string())?;
                stdout.flush()
            }
            Err(ReplError::Compile(diagnostics)) if self.color => {
                let mut stderr = io::stderr().lock();
                write!(stderr, "{}", diagnostics.render_pretty())?;
                stderr.flush()
            }
            Err(error) => {
                let mut stderr = io::stderr().lock();
                writeln!(stderr, "{error}")?;
                stderr.flush()
            }
        }
    }
}

/// `~> `, and a wider `..>  ` before each further line of a segment.
struct FrostPrompt;

impl Prompt for FrostPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("~> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("..>  ")
    }

    fn render_prompt_history_search_indicator(&self, search: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Owned(format!("(search: {}) ", search.term))
    }

    fn get_indicator_color(&self) -> Color {
        Color::LightBlue
    }

    fn get_prompt_multiline_color(&self) -> Color {
        Color::LightBlue
    }
}

struct FrostValidator;

impl Validator for FrostValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        match complete_segment(line) {
            Some(_) => ValidationResult::Complete,
            None => ValidationResult::Incomplete,
        }
    }
}

struct FrostHighlighter;

/// The colors brackets cycle through as they nest.
const BRACKET_COLORS: [Color; 5] = [
    Color::LightRed,
    Color::LightYellow,
    Color::LightGreen,
    Color::LightBlue,
    Color::LightMagenta,
];

impl Highlighter for FrostHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();
        for (span, class) in highlight::classify(line) {
            let style = match class {
                Class::Keyword => Style::new().fg(Color::LightCyan),
                Class::Number => Style::new().fg(Color::Yellow),
                Class::String => Style::new().fg(Color::Green),
                Class::Comment => Style::new().fg(Color::DarkGray),
                Class::Bracket(depth) => {
                    Style::new().fg(BRACKET_COLORS[depth % BRACKET_COLORS.len()])
                }
                Class::UnmatchedBracket => Style::new().fg(Color::White).on(Color::Red),
                Class::Plain => Style::new(),
            };
            styled.push((style, line[span].to_string()));
        }
        styled
    }
}
