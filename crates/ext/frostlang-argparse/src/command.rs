//! A checked spec as a clap `Command`, and a parse's matches as Frost values.

use clap::builder::PossibleValuesParser;
use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::parser::ValueSource;
use clap::{Arg, ArgAction, ArgMatches, Command};
use frostlang::{FrostError, NativeCtx, Value};

use crate::spec::{
    COMMAND, CommandSpec, FlagOrOption, FlagOrOptionKind, Occurrence, Operands, Positional, Values,
    long_form,
};
use crate::string_keyed_map;

/// The clap command for `spec`, named `name`.
pub(crate) fn build(name: &str, spec: &CommandSpec) -> Command {
    let command = Command::new(name.to_string())
        // A subcommand's help is its `--help`.
        .disable_help_subcommand(true)
        .hide(spec.hidden)
        .args(spec.flags_and_options.iter().map(flag_or_option_arg));
    let command = with_some(command, spec.description.as_ref(), Command::about);
    let command = with_some(command, spec.version.as_ref(), Command::version);
    let command = with_some(command, spec.epilog.as_ref(), Command::after_help);
    match &spec.operands {
        Operands::Positionals(positionals) => command.args(positionals.iter().map(positional_arg)),
        Operands::Commands(commands) => command
            .subcommand_required(true)
            .subcommands(commands.iter().map(|(name, command)| build(name, command))),
    }
}

fn flag_or_option_arg(spec: &FlagOrOption) -> Arg {
    let arg = Arg::new(spec.name.clone())
        .long(long_form(&spec.name))
        .short(spec.short)
        .hide(spec.hidden);
    let arg = with_some(arg, spec.description.as_ref(), Arg::help);
    match &spec.kind {
        FlagOrOptionKind::Flag => arg.action(ArgAction::SetTrue),
        FlagOrOptionKind::Option { values, value_name } => with_some(
            accept_values(arg, values),
            value_name.as_ref(),
            Arg::value_name,
        ),
    }
}

fn positional_arg(positional: &Positional) -> Arg {
    let arg = Arg::new(positional.name.clone()).hide(positional.hidden);
    let arg = with_some(arg, positional.description.as_ref(), Arg::help);
    let arg = accept_values(arg, &positional.values);
    match positional.values.occurrence {
        // One positional takes every value given for it, where an option takes
        // one value each time it is given.
        Occurrence::Repeated { required: true } => arg.num_args(1..),
        Occurrence::Repeated { required: false } => arg.num_args(0..),
        _ => arg,
    }
}

/// `target`, with `set` giving it `value` if there is one. clap takes a missing
/// setting as `None` only for static text.
fn with_some<T, V>(target: T, value: Option<V>, set: impl FnOnce(T, V) -> T) -> T {
    match value {
        Some(value) => set(target, value),
        None => target,
    }
}

/// `arg`, taking `values`.
fn accept_values(arg: Arg, values: &Values) -> Arg {
    // A value may be a negative number, as `--offset -5` would be.
    let arg = arg.allow_negative_numbers(true);
    let arg = match &values.occurrence {
        Occurrence::Required => arg.action(ArgAction::Set).required(true),
        Occurrence::Optional => arg.action(ArgAction::Set),
        Occurrence::Defaulted(value) => arg.action(ArgAction::Set).default_value(value.clone()),
        Occurrence::Repeated { required } => arg.action(ArgAction::Append).required(*required),
    };
    match &values.choices {
        Some(choices) => arg.value_parser(PossibleValuesParser::new(choices.iter().cloned())),
        None => arg,
    }
}

/// Why the values of a parse's arguments could not be read.
pub(crate) enum ReadError {
    /// `parse` gave Null for a value the command line gave: the command line's
    /// mistake, worded by clap.
    Invalid(clap::Error),
    /// A `parse` function raised this error, or gave Null for a default.
    Raised(FrostError),
}

/// The Map of each argument of `spec`'s value, by name, from `matches`, each
/// passed through its `parse`. The `parse` functions run in the order `spec`
/// gives the arguments, so the first that rejects its text is the one
/// reported; `command` words that usage error.
pub(crate) fn read(
    spec: &CommandSpec,
    matches: &ArgMatches,
    command: &Command,
    ctx: &mut NativeCtx<'_>,
) -> Result<Value, ReadError> {
    let mut entries = Vec::new();
    for arg in &spec.flags_and_options {
        let value = match &arg.kind {
            FlagOrOptionKind::Flag => Value::Bool(matches.get_flag(&arg.name)),
            FlagOrOptionKind::Option { values, .. } => {
                read_values(matches, &arg.name, values, command, ctx)?
            }
        };
        entries.push((arg.name.as_str(), value));
    }
    match &spec.operands {
        Operands::Positionals(positionals) => {
            for positional in positionals {
                let value =
                    read_values(matches, &positional.name, &positional.values, command, ctx)?;
                entries.push((positional.name.as_str(), value));
            }
        }
        Operands::Commands(commands) => {
            let (name, sub_matches) = matches
                .subcommand()
                .expect("the command requires a subcommand");
            let sub_command = command
                .find_subcommand(name)
                .expect("the chosen subcommand is the command's");
            let chosen = read(&commands[name], sub_matches, sub_command, ctx)?;
            entries.push((COMMAND, string_keyed_map([(name, chosen)])));
        }
    }
    Ok(string_keyed_map(entries))
}

/// The value of the argument `id`, which takes `values`, from `matches`, as
/// [`read`] makes each.
fn read_values(
    matches: &ArgMatches,
    id: &str,
    values: &Values,
    command: &Command,
    ctx: &mut NativeCtx<'_>,
) -> Result<Value, ReadError> {
    let value_of = |text: &String| -> Result<Value, ReadError> {
        let Some(parse) = &values.parse else {
            return Ok(Value::from(text.as_str()));
        };
        let parsed = ctx
            .invoke(&parse.function, [Value::from(text.as_str())])
            .map_err(ReadError::Raised)?;
        if !parsed.is_null() {
            return Ok(parsed);
        }
        Err(match matches.value_source(id) {
            Some(ValueSource::DefaultValue) => ReadError::Raised(parse.null_for_default()),
            _ => ReadError::Invalid(invalid_value(command, id, text)),
        })
    };
    match values.occurrence {
        Occurrence::Repeated { .. } => matches
            .get_many::<String>(id)
            .into_iter()
            .flatten()
            .map(value_of)
            .collect(),
        _ => matches
            .get_one::<String>(id)
            .map_or(Ok(Value::Null), value_of),
    }
}

/// The usage error for `text`, given for the argument `id` of `command`, which
/// its `parse` gave Null for: the error clap gives for a value its own value
/// parser rejects.
fn invalid_value(command: &Command, id: &str, text: &str) -> clap::Error {
    let arg = command
        .get_arguments()
        .find(|arg| arg.get_id() == id)
        .expect("the argument is the command's")
        .to_string();
    let mut error = clap::Error::new(ErrorKind::ValueValidation).with_cmd(command);
    error.insert(ContextKind::InvalidArg, ContextValue::String(arg));
    error.insert(
        ContextKind::InvalidValue,
        ContextValue::String(text.to_string()),
    );
    error
}
