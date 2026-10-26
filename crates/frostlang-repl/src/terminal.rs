//! [`TerminalFrontend`]: line editing, history, and highlighting on a terminal.

use std::borrow::Cow;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command};
use std::sync::{LazyLock, Mutex, MutexGuard, Once, PoisonError};

use frostlang::Value;
use nu_ansi_term::{Color, Style};
use reedline::{
    Emacs, FileBackedHistory, HISTORY_SIZE, Highlighter, Prompt, PromptEditMode,
    PromptHistorySearch, Reedline, SearchDirection, SearchQuery, Signal, StyledText,
    ValidationResult, Validator, Vi,
};

use crate::edit_dir::EditDir;
use crate::highlight::{self, Class};
use crate::{
    CancelHandle, Frontend, Invocation, MetacommandError, MetacommandSpec, MetacommandTable,
    ReplError, complete_segment,
};

mod settings;
#[cfg(test)]
mod tests;

use settings::{Context, EditMode, Problem, Prompts, Settings, StringDisplay};

/// A [`Frontend`] for a person at a terminal, with line editing, history, and
/// syntax highlighting.
///
/// A segment continues onto more lines until [`complete_segment`] completes
/// it.
///
/// Ctrl-C discards the segment being typed; Ctrl-D on an empty line ends input.
/// While an input runs, Ctrl-C [cancels](CancelHandle) it; at any other moment,
/// it ends the process with exit status 130.
/// Ctrl-O opens the segment being typed in the editor `$VISUAL` or `$EDITOR`
/// names, if either is set. The editor edits a file in a new directory only the
/// user can enter, in the system's temporary directory, which is removed when
/// the frontend is dropped; if that directory cannot be made, Ctrl-O does
/// nothing.
///
/// Each value other than Null is written to standard output: a String as the
/// `strings` setting below says, and anything else as
/// [`render_value`](crate::render_value) renders it.
/// A failure is written to standard error.
///
/// # Configuration
///
/// The frontend takes its settings from the `terminal` section of the
/// [config file](crate::Repl#config-file), a Map of them:
///
/// - `prompt`, shown before the first line of each segment;
///   `continuation_prompt`, before each further line; and `right_prompt`, at
///   the right of the first line, and by default empty. Each is a String, or a
///   function given a Map and returning one. The Map holds `number`, the
///   segment's number in the session, from 1; `failed`, whether the input
///   before it failed; and `cwd`, the working directory, or Null if it cannot
///   be read. A prompt replaces the default entirely, colors included.
/// - `edit_mode`: `'emacs'`, by default, or `'vi'`, for the keys lines are
///   edited with. Ctrl-C, Ctrl-D, and Ctrl-O work as described above in
///   either.
/// - `strings`: `'quoted'`, by default, for a String result written quoted
///   and escaped, or `'plain'`, for its text as it is. Only a String result
///   itself is written plain, not one inside another value. `:strings` shows
///   the setting, or sets it until `:config reload` takes the section again.
///
/// Its functions run as the config file does, under the same limits and with
/// the same imports:
///
/// ```frost
/// def repl = import('repl')
///
/// defn prompt(context) -> {
///     def number = repl.style.bold(to_string(context.number))
///     if context.failed: $'${number} ${repl.colors.red('!')}> '
///     else: $'${number}> '
/// }
///
/// export def terminal = {prompt, right_prompt: fn context -> context.cwd or ''}
/// ```
///
/// A problem with the section is shown as the frontend takes it: a section
/// holding an unknown setting, or one of the wrong type, sets nothing. A
/// setting left out, or Null, keeps its default. A prompt function that fails
/// or returns anything but a String is reported, and the default is used in its
/// place from then on.
///
/// # Ctrl-C is process-wide
///
/// A process has one Ctrl-C handler. The first session on a `TerminalFrontend`
/// installs one, which stays for the life of the process and cancels the
/// inputs of the latest session to start. If the host has installed a handler
/// already, through the `ctrlc` crate, Ctrl-C stays the host's and cancels no
/// input.
pub struct TerminalFrontend {
    color: bool,
    // `None` for the default history file.
    history: Option<FileBackedHistory>,
    // Built at the first read, so that the default history file is opened
    // only if no other history was chosen.
    editor: Option<Reedline>,
    // Made with the editor, if an external editor is named; holds the file it
    // edits.
    edit_dir: Option<EditDir>,
    // `None` until configured, or for a configuration with no section here.
    settings: Option<Settings>,
    // As configured, or as `:strings` set it since.
    strings: StringDisplay,
    segments_read: u64,
    // Whether the outcome rendered since the last segment was read is a failure.
    failed: bool,
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
            edit_dir: None,
            settings: None,
            strings: StringDisplay::default(),
            segments_read: 0,
            failed: false,
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
        if self.editor.is_none() {
            let history = self.history.take().unwrap_or_else(default_history);
            let mut editor = Reedline::create()
                .with_edit_mode(self.edit_mode())
                .with_validator(Box::new(FrostValidator))
                .with_highlighter(Box::new(FrostHighlighter))
                .with_history(Box::new(history))
                .with_ansi_colors(self.color)
                .use_bracketed_paste(true);
            if let Some(command) = external_editor()
                && let Ok(edit_dir) = EditDir::new()
            {
                editor = editor.with_buffer_editor(command, edit_dir.file());
                self.edit_dir = Some(edit_dir);
            }
            self.editor = Some(editor);
        }
        self.editor.as_mut().expect("the editor was just built")
    }

    /// How the settings say to edit lines.
    fn edit_mode(&self) -> Box<dyn reedline::EditMode> {
        let mode = self
            .settings
            .as_ref()
            .map(Settings::edit_mode)
            .unwrap_or_default();
        match mode {
            EditMode::Emacs => Box::<Emacs>::default(),
            EditMode::Vi => Box::<Vi>::default(),
        }
    }

    /// The prompts for the next segment, after showing any problem computing
    /// them.
    fn prompt(&mut self) -> io::Result<FrostPrompt> {
        let Some(settings) = &mut self.settings else {
            return Ok(FrostPrompt(Prompts::default()));
        };
        let context = Context {
            number: self.segments_read + 1,
            failed: self.failed,
            cwd: env::current_dir()
                .ok()
                .and_then(|dir| dir.into_os_string().into_string().ok()),
        };
        let (prompts, problems) = settings.prompts(&context);
        self.report(problems)?;
        Ok(FrostPrompt(prompts))
    }

    /// Show each of `problems`, on standard error.
    fn report(&self, problems: Vec<Problem>) -> io::Result<()> {
        if problems.is_empty() {
            return Ok(());
        }
        let mut stderr = io::stderr().lock();
        for problem in problems {
            writeln!(stderr, "{problem}")?;
        }
        stderr.flush()
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
            let prompt = self.prompt()?;
            match self.editor().read_line(&prompt)? {
                Signal::Success(segment) if segment.trim().is_empty() => {}
                Signal::Success(segment) => {
                    self.segments_read += 1;
                    self.failed = false;
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
        // A problem with the configuration is no input's failure.
        self.failed = matches!(outcome, Err(error) if !matches!(error, ReplError::Config(_)));
        match outcome {
            Ok(Value::Null) => Ok(()),
            Ok(value) => {
                let mut stdout = io::stdout().lock();
                writeln!(stdout, "{}", self.strings.render(value))?;
                stdout.flush()
            }
            #[cfg(feature = "graphical-diagnostics")]
            Err(
                ReplError::Compile(diagnostics)
                | ReplError::Config(crate::ConfigError::Compile(diagnostics)),
            ) if self.color => {
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

    fn set_cancel_handle(&mut self, handle: CancelHandle) {
        cancel_on_ctrl_c(handle);
    }

    fn config_key(&self) -> Option<&str> {
        Some(settings::KEY)
    }

    fn configure(&mut self, section: Option<Value>) -> io::Result<()> {
        let problems = match section {
            Some(section) => {
                let (settings, problems) = Settings::from_section(section, self.color);
                self.settings = Some(settings);
                problems
            }
            None => {
                self.settings = None;
                Vec::new()
            }
        };
        self.strings = self
            .settings
            .as_ref()
            .map(Settings::strings)
            .unwrap_or_default();
        // An editor not yet built takes the edit mode as it is built.
        if let Some(editor) = self.editor.take() {
            self.editor = Some(editor.with_edit_mode(self.edit_mode()));
        }
        self.report(problems)
    }
}

/// What Ctrl-C cancels through: the latest session's handle.
static CTRL_C_TARGET: Mutex<Option<CancelHandle>> = Mutex::new(None);

/// Have Ctrl-C cancel the inputs `handle` cancels, installing the process's
/// handler if this is the first session.
fn cancel_on_ctrl_c(handle: CancelHandle) {
    *ctrl_c_target() = Some(handle);
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        // Fails only if the host installed a handler first; Ctrl-C stays the host's.
        let _ = ctrlc::set_handler(on_ctrl_c);
    });
}

fn on_ctrl_c() {
    let cancelled = ctrl_c_target().as_ref().is_some_and(CancelHandle::cancel);
    if !cancelled {
        // No input is running: end the process, as Ctrl-C does by default.
        // While input is read, the terminal sends Ctrl-C as a key instead.
        process::exit(130);
    }
}

fn ctrl_c_target() -> MutexGuard<'static, Option<CancelHandle>> {
    // Every critical section is a single store or read, so a panic cannot
    // leave the target half-written.
    CTRL_C_TARGET.lock().unwrap_or_else(PoisonError::into_inner)
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
        .with(
            MetacommandSpec::new(
                "strings",
                "Show how String results are written, or set it: `quoted` or `plain`",
            )
            .with_arguments("[quoted|plain]"),
            show_or_set_strings,
        )
});

fn show_or_set_strings(frontend: &mut TerminalFrontend, invocation: &Invocation) -> io::Result<()> {
    match invocation.argument() {
        "" => frontend.render_text(frontend.strings.name()),
        name => match StringDisplay::try_from(name.to_string()) {
            Ok(strings) => {
                frontend.strings = strings;
                Ok(())
            }
            Err(_) => frontend.fail(MetacommandError::InvalidArgument {
                metacommand: invocation.name().to_string(),
                reason: "takes `quoted` or `plain`".to_string(),
            }),
        },
    }
}

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

/// The prompts the configuration computed, or by default `~> `, and a wider
/// `..>  ` before each further line of a segment, both light blue.
struct FrostPrompt(Prompts);

/// The color of a default prompt. A configured prompt is shown as written,
/// colors included.
const DEFAULT_PROMPT_COLOR: Color = Color::LightBlue;

impl Prompt for FrostPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.0.right_prompt.as_deref().unwrap_or(""))
    }

    fn get_prompt_right_color(&self) -> Color {
        Color::Default
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed(self.0.prompt.as_deref().unwrap_or("~> "))
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.0.continuation_prompt.as_deref().unwrap_or("..>  "))
    }

    fn render_prompt_history_search_indicator(&self, search: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Owned(format!("(search: {}) ", search.term))
    }

    fn get_indicator_color(&self) -> Color {
        match self.0.prompt {
            Some(_) => Color::Default,
            None => DEFAULT_PROMPT_COLOR,
        }
    }

    fn get_prompt_multiline_color(&self) -> Color {
        match self.0.continuation_prompt {
            Some(_) => Color::Default,
            None => DEFAULT_PROMPT_COLOR,
        }
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
