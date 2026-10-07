//! The REPL's own metacommands.

use frostlang::compile::{Diagnostic, Diagnostics, Optimization};
use frostlang_parse::parse_program;

use crate::metacommand::{Builtin, Registry};
use crate::{INPUT_NAME, Invocation, MetacommandError, RESULTS, Repl, ReplError};

/// What comes of a built-in metacommand.
pub(crate) enum Reply {
    /// Text for the frontend to show.
    Text(String),
    /// Nothing to show.
    Nothing,
    /// The session is to end.
    Quit,
}

/// `builtin` refusing its argument, for `reason`.
fn invalid_argument(builtin: Builtin, reason: impl Into<String>) -> ReplError {
    ReplError::Metacommand(MetacommandError::InvalidArgument {
        metacommand: builtin.name().to_string(),
        reason: reason.into(),
    })
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
            Builtin::Undef => self.undefine(builtin, argument),
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
                    let diagnostic = Diagnostic::from_parse_error(&error, INPUT_NAME, source);
                    ReplError::Compile(Diagnostics::from(diagnostic))
                })?;
                Ok(Reply::Text(program.dump()))
            }
            Builtin::Optimize if argument.is_empty() => Ok(Reply::Text(self.list_optimizations())),
            Builtin::Optimize => {
                self.optimization = self
                    .optimization
                    .with_settings(argument)
                    .map_err(|invalid| invalid_argument(builtin, invalid.to_string()))?;
                Ok(Reply::Nothing)
            }
        }
    }

    /// Each optimization, and whether it is on.
    fn list_optimizations(&self) -> String {
        let width = Optimization::ALL
            .iter()
            .map(|optimization| optimization.name().len())
            .max()
            .unwrap_or(0);
        let lines: Vec<String> = Optimization::ALL
            .iter()
            .map(|&optimization| {
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

    /// Unbind each name in `argument`, as `builtin` asks: all of them, or, if
    /// any is not bound, none.
    fn undefine(&mut self, builtin: Builtin, argument: &str) -> Result<Reply, ReplError> {
        let names: Vec<&str> = needs_argument(builtin, argument, "the names to remove")?
            .split_whitespace()
            .collect();
        for name in &names {
            if !self.bindings.contains_key(*name) {
                let reason = if *name == RESULTS && self.keeps_results() {
                    "`results` holds recent results, and cannot be removed".to_string()
                } else {
                    format!("`{name}` is not bound")
                };
                return Err(invalid_argument(builtin, reason));
            }
        }
        for name in names {
            self.bindings.remove(name);
        }
        Ok(Reply::Nothing)
    }
}

fn takes_no_argument(builtin: Builtin, argument: &str) -> Result<(), ReplError> {
    if argument.is_empty() {
        Ok(())
    } else {
        Err(invalid_argument(builtin, "takes no argument"))
    }
}

fn needs_argument<'a>(
    builtin: Builtin,
    argument: &'a str,
    what: &str,
) -> Result<&'a str, ReplError> {
    if argument.is_empty() {
        Err(invalid_argument(builtin, format!("needs {what}")))
    } else {
        Ok(argument)
    }
}
