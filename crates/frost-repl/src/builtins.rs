//! The REPL's own metacommands.

use frost_compile::{CompilerError, CompilerErrors, Optimization, OptimizationOptions};
use frost_parse::parse_program;
use frost_runtime::FrostError;

use crate::metacommand::{Builtin, Registry};
use crate::{INPUT_NAME, Invocation, RESULTS, Repl, ReplError};

/// What comes of a built-in metacommand.
pub(crate) enum Reply {
    /// Text for the frontend to show.
    Text(String),
    /// Nothing to show.
    Nothing,
    /// The session is to end.
    Quit,
}

/// A metacommand's failure, reported as `message`.
pub(crate) fn metacommand_error(message: String) -> ReplError {
    ReplError::Run(FrostError::from(message))
}

impl Repl {
    /// Run `builtin` as `invocation` asks. Text it gives carries ANSI styling
    /// only if `ansi_styling` is true.
    pub(crate) fn run_builtin(
        &mut self,
        builtin: Builtin,
        invocation: &Invocation,
        registry: &Registry,
        ansi_styling: bool,
    ) -> Result<Reply, ReplError> {
        let argument = invocation.argument();
        match builtin {
            Builtin::Help => {
                takes_no_argument(builtin, argument)?;
                Ok(Reply::Text(registry.help()))
            }
            Builtin::Quit => {
                takes_no_argument(builtin, argument)?;
                Ok(Reply::Quit)
            }
            Builtin::Bindings => {
                takes_no_argument(builtin, argument)?;
                Ok(self.list_bindings())
            }
            Builtin::Undef => self.undefine(argument),
            Builtin::Disassemble => {
                let source = needs_argument(builtin, argument, "source to disassemble")?;
                let closure = self.compile(source)?;
                let listing = closure
                    .inner_fn()
                    .disassemble()
                    .with_color(ansi_styling)
                    .to_string();
                Ok(Reply::Text(listing.trim_end().to_string()))
            }
            Builtin::Ast => {
                let source = needs_argument(builtin, argument, "source to parse")?;
                let program = parse_program(INPUT_NAME, source).map_err(|error| {
                    let diagnostic = CompilerError::from_parse_error(&error, INPUT_NAME, source);
                    ReplError::Compile(CompilerErrors::from(diagnostic))
                })?;
                // TODO: A more compact printer; spans make this verbose.
                Ok(Reply::Text(format!("{:#?}", program.statements)))
            }
            Builtin::Optimize if argument.is_empty() => Ok(Reply::Text(self.list_optimizations())),
            Builtin::Optimize => {
                self.optimization = with_settings(self.optimization, argument)?;
                Ok(Reply::Nothing)
            }
        }
    }

    /// Each optimization, and whether it is on.
    fn list_optimizations(&self) -> String {
        let width = Optimization::ALL
            .into_iter()
            .map(|optimization| optimization.name().len())
            .max()
            .unwrap_or(0);
        let lines: Vec<String> = Optimization::ALL
            .into_iter()
            .map(|optimization| {
                let name = optimization.name();
                format!("{name:width$}  {}", self.optimization.get(optimization))
            })
            .collect();
        lines.join("\n")
    }

    /// Each name in scope but the globals, with its value's type, in name
    /// order.
    fn list_bindings(&self) -> Reply {
        let mut names: Vec<(&str, &str)> = self
            .bindings
            .iter()
            .map(|(name, value)| (name.as_str(), value.type_name()))
            .collect();
        if self.keeps_results() {
            names.push((RESULTS, "Array"));
            names.sort_unstable();
        }
        if names.is_empty() {
            return Reply::Nothing;
        }
        let width = names.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
        let lines: Vec<String> = names
            .iter()
            .map(|(name, type_name)| format!("{name:width$}  {type_name}"))
            .collect();
        Reply::Text(lines.join("\n"))
    }

    /// Unbind each name in `argument`: all of them, or, if any is not bound,
    /// none.
    fn undefine(&mut self, argument: &str) -> Result<Reply, ReplError> {
        let names: Vec<&str> = argument.split_whitespace().collect();
        if names.is_empty() {
            return Err(metacommand_error(
                "`:undef` needs the names to remove".to_string(),
            ));
        }
        for name in &names {
            if !self.bindings.contains_key(*name) {
                let message = if *name == RESULTS && self.keeps_results() {
                    "`results` holds recent results, and cannot be removed".to_string()
                } else {
                    format!("`{name}` is not bound")
                };
                return Err(metacommand_error(message));
            }
        }
        for name in names {
            self.bindings.remove(name);
        }
        Ok(Reply::Nothing)
    }
}

/// `options` with each comma-separated setting in `settings` applied, left to
/// right: `<optimization> = true|false`, or `preset = all|none`. If any
/// setting is invalid, none is applied.
fn with_settings(
    options: OptimizationOptions,
    settings: &str,
) -> Result<OptimizationOptions, ReplError> {
    settings
        .split(',')
        .try_fold(options, |mut options, setting| {
            let Some((name, value)) = setting.split_once('=') else {
                return Err(metacommand_error(format!(
                    "`{}` should be `<optimization> = true|false` or `preset = all|none`",
                    setting.trim()
                )));
            };
            let (name, value) = (name.trim(), value.trim());
            if name == "preset" {
                return match value {
                    "all" => Ok(OptimizationOptions::ALL),
                    "none" => Ok(OptimizationOptions::NONE),
                    _ => Err(metacommand_error(format!(
                        "`preset` is `all` or `none`, not `{value}`"
                    ))),
                };
            }
            let Some(optimization) = Optimization::from_name(name) else {
                return Err(metacommand_error(format!(
                    "there is no optimization `{name}`; `:optimize` lists them"
                )));
            };
            let on = match value {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(metacommand_error(format!(
                        "`{name}` is `true` or `false`, not `{value}`"
                    )));
                }
            };
            options.set(optimization, on);
            Ok(options)
        })
}

fn takes_no_argument(builtin: Builtin, argument: &str) -> Result<(), ReplError> {
    if argument.is_empty() {
        Ok(())
    } else {
        Err(metacommand_error(format!(
            "`:{}` takes no argument",
            builtin.name()
        )))
    }
}

fn needs_argument<'a>(
    builtin: Builtin,
    argument: &'a str,
    what: &str,
) -> Result<&'a str, ReplError> {
    if argument.is_empty() {
        Err(metacommand_error(format!(
            "`:{}` needs {what}",
            builtin.name()
        )))
    } else {
        Ok(argument)
    }
}
