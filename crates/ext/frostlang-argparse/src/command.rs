//! A checked spec as a clap `Command`, and a parse's matches as Frost values.

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use frostlang::Value;

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
        .args(spec.flags_and_options.iter().map(flag_or_option_arg));
    let command = with_some(command, spec.description.as_ref(), Command::about);
    let command = with_some(command, spec.version.as_ref(), Command::version);
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
        .short(spec.short);
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
    let arg = Arg::new(positional.name.clone());
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

/// The Map of what `matches` found for each argument of `spec`, by name.
pub(crate) fn read(spec: &CommandSpec, matches: &ArgMatches) -> Value {
    let flags_and_options = spec.flags_and_options.iter().map(|arg| {
        let value = match &arg.kind {
            FlagOrOptionKind::Flag => Value::Bool(matches.get_flag(&arg.name)),
            FlagOrOptionKind::Option { values, .. } => read_values(matches, &arg.name, values),
        };
        (arg.name.as_str(), value)
    });
    let operands: Vec<(&str, Value)> = match &spec.operands {
        Operands::Positionals(positionals) => positionals
            .iter()
            .map(|positional| {
                let value = read_values(matches, &positional.name, &positional.values);
                (positional.name.as_str(), value)
            })
            .collect(),
        Operands::Commands(commands) => {
            let (name, sub_matches) = matches
                .subcommand()
                .expect("the command requires a subcommand");
            let chosen = string_keyed_map([(name, read(&commands[name], sub_matches))]);
            vec![(COMMAND, chosen)]
        }
    };
    string_keyed_map(flags_and_options.chain(operands))
}

/// What `matches` found for the argument `id`, which takes `values`.
fn read_values(matches: &ArgMatches, id: &str, values: &Values) -> Value {
    let string = |value: &String| Value::from(value.as_str());
    match values.occurrence {
        Occurrence::Repeated { .. } => matches
            .get_many::<String>(id)
            .into_iter()
            .flatten()
            .map(string)
            .collect(),
        _ => matches.get_one::<String>(id).map_or(Value::Null, string),
    }
}
