//! The [`TerminalFrontend`](crate::TerminalFrontend)'s settings: what its
//! section of the configuration sets.

use std::fmt;
use std::sync::Arc;

use frostlang::native::Function;
use frostlang::{Closure, FrostError, MapKey, Nullable, Value, from_value};
use serde::{Deserialize, Deserializer, de};

use crate::config::chosen;
use crate::config_runner::ConfigRunner;

#[cfg(test)]
mod tests;

/// The export the frontend takes its settings from.
pub(crate) const KEY: &str = "terminal";

/// The section, as written. Read each setting through [`chosen`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Section {
    prompt: Option<Nullable<PromptSetting>>,
    continuation_prompt: Option<Nullable<PromptSetting>>,
    right_prompt: Option<Nullable<PromptSetting>>,
    edit_mode: Option<Nullable<EditMode>>,
}

/// A prompt, as written: a String, or a Function computing one.
enum PromptSetting {
    Text(String),
    Function(Function),
}

impl<'de> Deserialize<'de> for PromptSetting {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::String(text) => Ok(Self::Text(text.as_str().to_owned())),
            value => {
                let type_name = value.type_name();
                Function::try_from(value).map(Self::Function).map_err(|_| {
                    // Worded as the serde bridge words its own type mismatches.
                    de::Error::custom(format_args!("expected String or Function, got {type_name}"))
                })
            }
        }
    }
}

/// A prompt the section may set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Setting {
    Prompt,
    ContinuationPrompt,
    RightPrompt,
}

impl Setting {
    const ALL: [Self; 3] = [Self::Prompt, Self::ContinuationPrompt, Self::RightPrompt];

    /// Its key in the section.
    fn name(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::ContinuationPrompt => "continuation_prompt",
            Self::RightPrompt => "right_prompt",
        }
    }

    /// What the frontend shows in its place, by default.
    fn default_description(self) -> &'static str {
        match self {
            Self::Prompt => "the default prompt",
            Self::ContinuationPrompt => "the default continuation prompt",
            Self::RightPrompt => "no right prompt",
        }
    }
}

/// How lines are edited: with emacs's keys, or vi's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum EditMode {
    #[default]
    Emacs,
    Vi,
}

/// Fails for anything but the name of an edit mode.
impl TryFrom<String> for EditMode {
    type Error = String;

    fn try_from(name: String) -> Result<Self, String> {
        match name.as_str() {
            "emacs" => Ok(Self::Emacs),
            "vi" => Ok(Self::Vi),
            _ => Err(format!(r#"expected "emacs" or "vi", got {name:?}"#)),
        }
    }
}

/// A prompt the section sets: as written, or computed by a function.
enum Prompt {
    Text(String),
    Computed(Arc<Closure>),
}

/// What the section sets, and how its functions run.
pub(crate) struct Settings {
    prompt: Option<Prompt>,
    continuation_prompt: Option<Prompt>,
    right_prompt: Option<Prompt>,
    edit_mode: EditMode,
    runner: ConfigRunner,
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
    pub(crate) right_prompt: Option<String>,
}

impl Settings {
    /// What `section` sets, and the problems found in it. A section that does
    /// not hold the settings sets nothing. If its functions import `repl` as
    /// they run, its painters color only if `color` is true.
    pub(crate) fn from_section(section: Value, color: bool) -> (Self, Vec<Problem>) {
        let mut settings = Self {
            prompt: None,
            continuation_prompt: None,
            right_prompt: None,
            edit_mode: EditMode::default(),
            runner: ConfigRunner::new(color),
        };
        let section: Section = match from_value(section) {
            Ok(section) => section,
            Err(error) => return (settings, vec![Problem::Invalid(error)]),
        };
        settings.edit_mode = chosen(section.edit_mode).unwrap_or_default();
        let prompts = [
            chosen(section.prompt),
            chosen(section.continuation_prompt),
            chosen(section.right_prompt),
        ];
        let mut problems = Vec::new();
        for (setting, written) in Setting::ALL.into_iter().zip(prompts) {
            let Some(written) = written else { continue };
            match settings.prompt_from(setting, written) {
                Ok(prompt) => *settings.slot(setting) = Some(prompt),
                Err(problem) => problems.push(problem),
            }
        }
        (settings, problems)
    }

    /// How lines are edited.
    pub(crate) fn edit_mode(&self) -> EditMode {
        self.edit_mode
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
            right_prompt: compute(Setting::RightPrompt),
        };
        (prompts, problems)
    }

    fn slot(&mut self, setting: Setting) -> &mut Option<Prompt> {
        match setting {
            Setting::Prompt => &mut self.prompt,
            Setting::ContinuationPrompt => &mut self.continuation_prompt,
            Setting::RightPrompt => &mut self.right_prompt,
        }
    }

    /// The prompt `written`, the value of `setting`, sets.
    fn prompt_from(&mut self, setting: Setting, written: PromptSetting) -> Result<Prompt, Problem> {
        match written {
            PromptSetting::Text(text) => Ok(Prompt::Text(text)),
            PromptSetting::Function(function) => match function.into_value() {
                Value::Closure(closure) => Ok(Prompt::Computed(closure)),
                native => self
                    .runner
                    .wrap(native)
                    .map(Prompt::Computed)
                    .map_err(|error| Problem::PromptFailed { setting, error }),
            },
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

/// Something wrong with the section, or with a prompt it computes.
#[derive(Debug)]
pub(crate) enum Problem {
    /// The section does not hold the settings, as their types say.
    Invalid(FrostError),
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
            Self::Invalid(error) => write!(
                f,
                "Warning: the REPL configuration's `{KEY}` sets nothing: {error}"
            ),
            Self::PromptFailed { setting, error } => write!(
                f,
                "Error: `{KEY}.{}` failed, so {} is used from now on: {}",
                setting.name(),
                setting.default_description(),
                error.with_backtrace()
            ),
            Self::PromptNotString { setting, type_name } => write!(
                f,
                "Error: `{KEY}.{}` must return a String, got {type_name}, so {} is used from now \
                 on",
                setting.name(),
                setting.default_description()
            ),
        }
    }
}
