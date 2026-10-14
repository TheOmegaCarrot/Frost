//! The [`TerminalFrontend`](crate::TerminalFrontend)'s configuration: a Frost
//! script whose exports set its prompts.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use frostlang::compile::{CompilerOptions, Diagnostics, compile_in_scope, compile_program};
use frostlang::{
    Closure, FrostError, HostComponent, IdleVm, Importer, ImporterBuilder, MapKey, ProgramParts,
    Stdlib, Value, Vm, VmRuntimeConfiguration, frost_closure, stdlib,
};

#[cfg(test)]
mod tests;

/// The configuration file in the platform's configuration directory, if it
/// has one.
pub(crate) fn default_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("frost").join("repl.frst"))
}

/// A setting the configuration may export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Setting {
    Prompt,
    ContinuationPrompt,
}

impl Setting {
    const ALL: [Self; 2] = [Self::Prompt, Self::ContinuationPrompt];

    /// The name the configuration exports it as.
    fn name(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::ContinuationPrompt => "continuation_prompt",
        }
    }

    /// What the frontend shows in its place, by default.
    fn default_description(self) -> &'static str {
        match self {
            Self::Prompt => "the default prompt",
            Self::ContinuationPrompt => "the default continuation prompt",
        }
    }
}

/// A prompt the configuration sets: as written, or computed by a function.
enum Prompt {
    Text(String),
    Computed(Arc<Closure>),
}

/// What the configuration sets, and the Vm its functions run on.
#[derive(Default)]
pub(crate) struct Config {
    prompt: Option<Prompt>,
    continuation_prompt: Option<Prompt>,
    runner: Runner,
}

/// What a segment's prompts are computed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Context {
    /// The segment's number in the session, from 1.
    pub(crate) number: u64,
    /// Whether the input before it failed.
    pub(crate) failed: bool,
    /// The working directory, if it can be read and is UTF-8.
    pub(crate) cwd: Option<String>,
}

impl Context {
    fn to_value(&self) -> Value {
        let number = i64::try_from(self.number).unwrap_or(i64::MAX);
        let cwd = self.cwd.clone().map_or(Value::Null, Value::from);
        Value::Map(
            [
                (MapKey::from("number"), Value::from(number)),
                (MapKey::from("failed"), Value::from(self.failed)),
                (MapKey::from("cwd"), cwd),
            ]
            .into_iter()
            .collect(),
        )
    }
}

/// A segment's prompts: `None` for the frontend's default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Prompts {
    pub(crate) prompt: Option<String>,
    pub(crate) continuation_prompt: Option<String>,
}

impl Config {
    /// The configuration in the file at `path`, and the problems found in it.
    /// With no file there, nothing is set, and there is no problem.
    pub(crate) fn read(path: &Path, color: bool) -> (Self, Vec<Problem>) {
        match fs::read_to_string(path) {
            Ok(source) => Self::load(&path.display().to_string(), &source, color),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (Self::default(), Vec::new()),
            Err(error) => (
                Self::default(),
                vec![Problem::Unreadable {
                    path: path.to_owned(),
                    error,
                }],
            ),
        }
    }

    /// The configuration `source`, from the file `filename`, sets, and the
    /// problems found in it. `color` is whether `repl.colors` and `repl.style`
    /// may color their text.
    pub(crate) fn load(filename: &str, source: &str, color: bool) -> (Self, Vec<Problem>) {
        let mut runner = Runner::new(color);
        let exports = match runner.run_script(filename, source) {
            Ok(exports) => exports,
            Err(problem) => return (Self::default(), vec![problem]),
        };
        let mut config = Self {
            prompt: None,
            continuation_prompt: None,
            runner,
        };
        let mut problems = Vec::new();
        for (name, value) in exports {
            let Some(setting) = Setting::ALL.into_iter().find(|s| s.name() == name) else {
                problems.push(Problem::UnknownExport(name));
                continue;
            };
            match config.prompt_from(setting, value) {
                Ok(prompt) => *config.slot(setting) = Some(prompt),
                Err(problem) => problems.push(problem),
            }
        }
        (config, problems)
    }

    /// The prompts for the segment `context` describes, and the problems
    /// computing them. A prompt function that fails is not called again.
    pub(crate) fn prompts(&mut self, context: &Context) -> (Prompts, Vec<Problem>) {
        let context = context.to_value();
        let mut problems = Vec::new();
        let mut compute = |setting| match self.compute(setting, &context) {
            Ok(prompt) => prompt,
            Err(problem) => {
                problems.push(problem);
                None
            }
        };
        let prompts = Prompts {
            prompt: compute(Setting::Prompt),
            continuation_prompt: compute(Setting::ContinuationPrompt),
        };
        (prompts, problems)
    }

    fn slot(&mut self, setting: Setting) -> &mut Option<Prompt> {
        match setting {
            Setting::Prompt => &mut self.prompt,
            Setting::ContinuationPrompt => &mut self.continuation_prompt,
        }
    }

    /// The prompt `value`, the export of `setting`, sets.
    fn prompt_from(&mut self, setting: Setting, value: Value) -> Result<Prompt, Problem> {
        match value {
            Value::String(text) => Ok(Prompt::Text(text.as_str().to_owned())),
            Value::Closure(closure) => Ok(Prompt::Computed(closure)),
            Value::NativeFunction(_) => self
                .runner
                .wrap(value)
                .map(Prompt::Computed)
                .map_err(Problem::Run),
            other => Err(Problem::WrongType {
                setting,
                type_name: other.type_name(),
            }),
        }
    }

    /// The prompt `setting` sets for `context`, or `None` for the default. A
    /// function that fails is dropped, leaving the default from then on.
    fn compute(&mut self, setting: Setting, context: &Value) -> Result<Option<String>, Problem> {
        let closure = match self.slot(setting) {
            None => return Ok(None),
            Some(Prompt::Text(text)) => return Ok(Some(text.clone())),
            Some(Prompt::Computed(closure)) => Arc::clone(closure),
        };
        let outcome = match self.runner.call(closure, context.clone()) {
            Ok(Value::String(text)) => return Ok(Some(text.as_str().to_owned())),
            Ok(other) => Problem::PromptNotString {
                setting,
                type_name: other.type_name(),
            },
            Err(error) => Problem::PromptFailed { setting, error },
        };
        *self.slot(setting) = None;
        Err(outcome)
    }
}

/// What the configuration and its functions may spend. A prompt is computed
/// before every segment, so one that runs away must fail fast.
fn limits() -> VmRuntimeConfiguration {
    const FUEL: NonZeroU64 = NonZeroU64::new(100_000).unwrap();
    const CALL_DEPTH: NonZeroU64 = NonZeroU64::new(256).unwrap();
    const IMPORT_DEPTH: NonZeroU64 = NonZeroU64::new(4).unwrap();
    VmRuntimeConfiguration::default()
        .with_fuel(Some(FUEL))
        .with_max_call_depth(Some(CALL_DEPTH))
        .with_max_import_depth(Some(IMPORT_DEPTH))
}

/// Runs the configuration and its functions, on a Vm kept warm between runs.
#[derive(Default)]
struct Runner {
    importer: Arc<Importer>,
    idle_vm: Option<IdleVm>,
}

impl Runner {
    /// A runner whose scripts may import `std.string`, `std.math`, and the
    /// `repl` component.
    fn new(color: bool) -> Self {
        let stdlib = Stdlib::new()
            .with_module(stdlib::string())
            .and_then(|stdlib| stdlib.with_module(stdlib::math()))
            .expect("the modules are distinct");
        let importer = ImporterBuilder::new()
            .with_stdlib(stdlib)
            .with_component(component(color))
            .expect("`repl` is free to claim")
            .build();
        Self {
            importer,
            idle_vm: None,
        }
    }

    /// The exports of the script `source`, from the file `filename`.
    fn run_script(
        &mut self,
        filename: &str,
        source: &str,
    ) -> Result<Vec<(String, Value)>, Problem> {
        let program = compile_program(filename, source, CompilerOptions::new())
            .map_err(Problem::Compile)?
            .code;
        let closure = program
            .close(BTreeMap::new())
            .expect("a whole script captures nothing");
        self.run(closure, None)
            .map(|parts| parts.exports)
            .map_err(Problem::Run)
    }

    /// `function`, a native, as a closure, so that it runs as a script does.
    fn wrap(&mut self, function: Value) -> Result<Arc<Closure>, FrostError> {
        const SOURCE: &str = "fn context -> function(context)";
        let program = compile_in_scope("<prompt>", SOURCE, CompilerOptions::new(), &["function"])
            .expect("the wrapper compiles")
            .code;
        let closure = program
            .close(BTreeMap::from([("function".to_owned(), function)]))
            .expect("the wrapper captures only `function`");
        match self.run(closure, None)?.tail {
            Value::Closure(wrapper) => Ok(wrapper),
            other => unreachable!("the wrapper is a lambda, not {}", other.type_name()),
        }
    }

    /// What `closure` returns, given `argument`.
    fn call(&mut self, closure: Arc<Closure>, argument: Value) -> Result<Value, FrostError> {
        self.run(closure, Some(argument)).map(|parts| parts.tail)
    }

    /// The tail and exports of `closure`, given `argument`, if any.
    fn run(&mut self, closure: Arc<Closure>, argument: Option<Value>) -> Result<Ran, FrostError> {
        let vm = match self.idle_vm.take() {
            Some(idle_vm) => idle_vm.build(closure),
            None => Vm::factory()
                .configuration(limits())
                .with_importer(Arc::clone(&self.importer))
                .build(closure),
        };
        match vm.run_with_args(argument) {
            Ok(result) => {
                let ProgramParts {
                    tail,
                    exports,
                    idle_vm,
                } = result.into_parts();
                self.idle_vm = Some(idle_vm);
                Ok(Ran { tail, exports })
            }
            Err(failure) => {
                let (error, idle_vm) = failure.into_parts();
                self.idle_vm = Some(idle_vm);
                Err(error)
            }
        }
    }
}

/// What a run gave back.
struct Ran {
    tail: Value,
    exports: Vec<(String, Value)>,
}

/// The `repl` component: `repl.colors` and `repl.style` hold functions that
/// wrap a String in ANSI codes, or, if `color` is false, return it unchanged.
fn component(color: bool) -> HostComponent {
    HostComponent::builder("repl")
        .module("colors", |colors| {
            COLORS
                .into_iter()
                .fold(colors, |colors, (key, name, code)| {
                    colors.value(key, painter(name, color, code, FOREGROUND_RESET))
                })
        })
        .module("style", |style| {
            STYLES
                .into_iter()
                .fold(style, |style, (key, name, on, off)| {
                    style.value(key, painter(name, color, on, off))
                })
        })
        .build()
}

/// The SGR code that returns the foreground to the terminal's default.
const FOREGROUND_RESET: u8 = 39;

/// Each of `repl.colors`: its key, its function's name, and its SGR code.
const COLORS: [(&str, &str, u8); 16] = [
    ("black", "repl.colors.black", 30),
    ("red", "repl.colors.red", 31),
    ("green", "repl.colors.green", 32),
    ("yellow", "repl.colors.yellow", 33),
    ("blue", "repl.colors.blue", 34),
    ("magenta", "repl.colors.magenta", 35),
    ("cyan", "repl.colors.cyan", 36),
    ("white", "repl.colors.white", 37),
    ("bright_black", "repl.colors.bright_black", 90),
    ("bright_red", "repl.colors.bright_red", 91),
    ("bright_green", "repl.colors.bright_green", 92),
    ("bright_yellow", "repl.colors.bright_yellow", 93),
    ("bright_blue", "repl.colors.bright_blue", 94),
    ("bright_magenta", "repl.colors.bright_magenta", 95),
    ("bright_cyan", "repl.colors.bright_cyan", 96),
    ("bright_white", "repl.colors.bright_white", 97),
];

/// Each of `repl.style`: its key, its function's name, and the SGR codes that
/// turn it on and off.
const STYLES: [(&str, &str, u8, u8); 4] = [
    ("bold", "repl.style.bold", 1, 22),
    ("dim", "repl.style.dim", 2, 22),
    ("italic", "repl.style.italic", 3, 23),
    ("underline", "repl.style.underline", 4, 24),
];

/// A function named `name` that wraps a String between the SGR codes `on` and
/// `off`, or, if `color` is false, returns it unchanged.
fn painter(name: &'static str, color: bool, on: u8, off: u8) -> Value {
    frost_closure!(name, move |text: String| {
        if color {
            format!("\x1b[{on}m{text}\x1b[{off}m")
        } else {
            text
        }
    })
}

/// Something wrong with the configuration, or with a prompt it computes.
#[derive(Debug)]
pub(crate) enum Problem {
    /// The file is there but cannot be read.
    Unreadable { path: PathBuf, error: io::Error },
    /// The configuration does not compile.
    Compile(Diagnostics),
    /// The configuration raised an error while running.
    Run(FrostError),
    /// The configuration exports a name that is no setting.
    UnknownExport(String),
    /// A setting is neither a String nor a Function.
    WrongType {
        setting: Setting,
        type_name: &'static str,
    },
    /// A prompt function raised an error, perhaps by exceeding its limits.
    PromptFailed { setting: Setting, error: FrostError },
    /// A prompt function returned something other than a String.
    PromptNotString {
        setting: Setting,
        type_name: &'static str,
    },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, error } => write!(
                f,
                "Error: cannot read the REPL configuration {}: {error}",
                path.display()
            ),
            #[cfg(feature = "graphical-diagnostics")]
            Self::Compile(diagnostics) => f.write_str(diagnostics.render_plain().trim_end()),
            #[cfg(not(feature = "graphical-diagnostics"))]
            Self::Compile(diagnostics) => f.write_str(diagnostics.render_narrated().trim_end()),
            Self::Run(error) => write!(
                f,
                "Error: the REPL configuration failed: {}",
                error.with_backtrace()
            ),
            Self::UnknownExport(name) => write!(
                f,
                "Warning: the REPL configuration exports `{name}`, which is not a setting; \
                 the settings are `prompt` and `continuation_prompt`"
            ),
            Self::WrongType { setting, type_name } => write!(
                f,
                "Warning: the REPL configuration's `{}` must be a String or a Function, got \
                 {type_name}; {} is used instead",
                setting.name(),
                setting.default_description()
            ),
            Self::PromptFailed { setting, error } => write!(
                f,
                "Error: `{}` failed, so {} is used from now on: {}",
                setting.name(),
                setting.default_description(),
                error.with_backtrace()
            ),
            Self::PromptNotString { setting, type_name } => write!(
                f,
                "Error: `{}` must return a String, got {type_name}, so {} is used from now on",
                setting.name(),
                setting.default_description()
            ),
        }
    }
}
