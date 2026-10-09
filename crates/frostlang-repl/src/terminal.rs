//! [`TerminalFrontend`]: line editing, history, and highlighting on a terminal.

use std::borrow::Cow;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command};
use std::sync::LazyLock;

use frostlang::Value;
use nu_ansi_term::{Color, Style};
use reedline::{
    FileBackedHistory, HISTORY_SIZE, Highlighter, Prompt, PromptEditMode, PromptHistorySearch,
    Reedline, SearchDirection, SearchQuery, Signal, StyledText, ValidationResult, Validator,
};

use crate::highlight::{self, Class};
use crate::{
    Frontend, Invocation, MetacommandError, MetacommandSpec, MetacommandTable, ReplError,
    complete_segment, render_value,
};

/// A [`Frontend`] for a person at a terminal, with line editing, history, and
/// syntax highlighting.
///
/// A segment continues onto more lines until [`complete_segment`] completes
/// it.
///
/// Ctrl-C discards the segment being typed; Ctrl-D on an empty line ends input.
/// Ctrl-O opens the segment being typed in the editor `$VISUAL` or `$EDITOR`
/// names, if either is set.
///
/// Each value other than Null is written as [`render_value`] renders it, to
/// standard output; a failure is written to standard error.
pub struct TerminalFrontend {
    color: bool,
    // `None` for the default history file.
    history: Option<FileBackedHistory>,
    // Built at the first read, so that the default history file is opened
    // only if no other history was chosen.
    editor: Option<Reedline>,
}

impl Default for TerminalFrontend {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalFrontend {
    /// A colored frontend on the terminal, keeping history from one session
    /// to the next in the platform's usual place for it:
    ///
    /// | Platform | History file |
    /// | -------- | ------------ |
    /// | Linux | `$XDG_STATE_HOME/frost/history`, or `~/.local/state/frost/history` |
    /// | macOS | `~/Library/Application Support/frost/history` |
    /// | Windows | `%LOCALAPPDATA%\frost\history` |
    ///
    /// If there is no such place, or its history cannot be read, history is
    /// kept for the session only.
    pub fn new() -> Self {
        Self {
            color: true,
            history: None,
            editor: None,
        }
    }

    /// Set whether to color the prompt, the source being typed, and
    /// diagnostics.
    pub fn with_color(mut self, color: bool) -> Self {
        self.color = color;
        self
    }

    /// Keep history in the file at `path` instead, reading what is there
    /// already.
    pub fn with_history_file(mut self, path: PathBuf) -> io::Result<Self> {
        let history = FileBackedHistory::with_file(HISTORY_SIZE, path).map_err(io::Error::other)?;
        self.history = Some(history);
        Ok(self)
    }

    /// Keep history for this session only, writing it to no file.
    pub fn with_session_history(mut self) -> Self {
        self.history = Some(FileBackedHistory::default());
        self
    }

    fn editor(&mut self) -> &mut Reedline {
        self.editor.get_or_insert_with(|| {
            let history = self.history.take().unwrap_or_else(default_history);
            let editor = Reedline::create()
                .with_validator(Box::new(FrostValidator))
                .with_highlighter(Box::new(FrostHighlighter))
                .with_history(Box::new(history))
                .with_ansi_colors(self.color)
                .use_bracketed_paste(true);
            match external_editor() {
                Some(command) => editor.with_buffer_editor(command, edit_file()),
                None => editor,
            }
        })
    }
}

/// The editor `$VISUAL`, or else `$EDITOR`, names, if either is set.
fn external_editor() -> Option<Command> {
    let line = ["VISUAL", "EDITOR"]
        .into_iter()
        .filter_map(env::var_os)
        .find(|line| !line.is_empty())?
        .into_string()
        .ok()?;
    // An editor may be named with arguments, as `code --wait` is.
    let mut words = line.split_whitespace();
    let mut command = Command::new(words.next()?);
    command.args(words);
    Some(command)
}

/// The file the external editor edits a segment in. Its extension lets the
/// editor recognize Frost source.
fn edit_file() -> PathBuf {
    env::temp_dir().join(format!("frost-edit-{}.frst", process::id()))
}

/// History in the platform's default file, or for the session only if there
/// is none or it cannot be read.
fn default_history() -> FileBackedHistory {
    // Only Linux has a state directory; elsewhere, history is local data.
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|dir| dir.join("frost").join("history"))
        .and_then(|path| FileBackedHistory::with_file(HISTORY_SIZE, path).ok())
        .unwrap_or_default()
}

impl Frontend for TerminalFrontend {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        loop {
            match self.editor().read_line(&FrostPrompt)? {
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
                writeln!(stdout, "{}", render_value(value))?;
                stdout.flush()
            }
            #[cfg(feature = "graphical-diagnostics")]
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

    fn render_text(&mut self, text: &str) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        writeln!(stdout, "{text}")?;
        stdout.flush()
    }

    fn ansi_styling(&self) -> bool {
        self.color
    }

    fn metacommands(&self) -> Vec<MetacommandSpec> {
        METACOMMANDS.specs()
    }

    fn metacommand(&mut self, invocation: &Invocation) -> io::Result<()> {
        METACOMMANDS.dispatch(self, invocation)
    }
}

static METACOMMANDS: LazyLock<MetacommandTable<TerminalFrontend>> = LazyLock::new(|| {
    MetacommandTable::new()
        .with(
            MetacommandSpec::new("history", "List the history, oldest first"),
            show_history,
        )
        .with(
            MetacommandSpec::new(
                "clear-history",
                "Forget the history, and its file's contents",
            ),
            clear_history,
        )
});

fn show_history(frontend: &mut TerminalFrontend, invocation: &Invocation) -> io::Result<()> {
    if !takes_no_argument(frontend, invocation)? {
        return Ok(());
    }
    let query = SearchQuery::everything(SearchDirection::Forward, None);
    let entries = match frontend.editor().history().search(query) {
        Ok(entries) => entries,
        Err(error) => {
            return frontend.fail(MetacommandError::Failed {
                metacommand: invocation.name().to_string(),
                reason: format!("cannot read the history: {error}"),
            });
        }
    };
    // Each entry is numbered, its further lines aligned under its first.
    let width = entries.len().to_string().len();
    let mut stdout = io::stdout().lock();
    for (number, entry) in (1..).zip(&entries) {
        let mut lines = entry.command_line.lines();
        writeln!(stdout, "{number:>width$}  {}", lines.next().unwrap_or(""))?;
        for line in lines {
            writeln!(stdout, "{:width$}  {line}", "")?;
        }
    }
    stdout.flush()
}

fn clear_history(frontend: &mut TerminalFrontend, invocation: &Invocation) -> io::Result<()> {
    if !takes_no_argument(frontend, invocation)? {
        return Ok(());
    }
    let history = frontend.editor().history_mut();
    // Clearing removes the history's file, which syncing first ensures exists.
    let cleared = history
        .sync()
        .map_err(|error| error.to_string())
        .and_then(|()| history.clear().map_err(|error| error.to_string()));
    match cleared {
        Ok(()) => Ok(()),
        Err(error) => frontend.fail(MetacommandError::Failed {
            metacommand: invocation.name().to_string(),
            reason: format!("cannot clear the history: {error}"),
        }),
    }
}

/// Whether `invocation` came without an argument, as it must. If not, the
/// frontend says so.
fn takes_no_argument(frontend: &mut TerminalFrontend, invocation: &Invocation) -> io::Result<bool> {
    if invocation.argument().is_empty() {
        return Ok(true);
    }
    frontend.fail(MetacommandError::InvalidArgument {
        metacommand: invocation.name().to_string(),
        reason: "takes no argument".to_string(),
    })?;
    Ok(false)
}

impl TerminalFrontend {
    /// Show `error` the way a failed input is shown.
    fn fail(&mut self, error: MetacommandError) -> io::Result<()> {
        self.render(Err(&ReplError::Metacommand(error)))
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
