//! Command-line argument parsing for Frost scripts: flags, options,
//! positional arguments, and subcommands, described by a spec, with generated
//! help.
//!
//! A host installs the extension with [`extension`], and scripts import it as
//! `ext.argparse`:
//!
//! ```
//! use frostlang::ImporterBuilder;
//!
//! let importer = ImporterBuilder::new()
//!     .with_extension(frostlang_argparse::extension())
//!     .expect("no other extension is named `argparse`")
//!     .build();
//! ```

mod command;
mod spec;

use std::path::Path;

use clap::error::ErrorKind;
use frostlang::native::{ArgSite, FromArg};
use frostlang::{
    Extension, FrostError, FrostResult, FrostType, MapKey, NativeCtx, TypeSet, Value, frost_fn,
};

use crate::command::ReadError;
use crate::spec::Spec;

/// The `ext.argparse` extension.
///
/// `parse(args, spec)` parses the command line `args`, an Array of Strings
/// whose first is the program's name, as the Map `spec` describes. It returns
/// a Map of one key, saying how the parse went:
///
/// - `{value}`: the command line is valid, and `value` is a Map of what it
///   gives each argument, by name.
/// - `{help}` or `{version}`: the command line asked for help or the version,
///   which this String holds, for the script to print.
/// - `{error}`: the command line is invalid. This String says why, for the
///   script to show its user, and how to get help.
///
/// A script matches on the outcome:
///
/// ```frost
/// def argparse = import('ext.argparse')
///
/// def spec = {
///     name: 'greet',
///     description: 'Greet someone',
///     flags: {shout: {short: 's', description: 'Greet loudly'}},
///     options: {greeting: {short: 'g', default: 'Hello'}},
///     positionals: [{name: 'who', description: 'Whom to greet'}],
/// }
///
/// defn greet(options) -> {
///     def line = $'${options.greeting}, ${options.who}!'
///     print(if options.shout: to_upper(line) else: line)
/// }
///
/// match argparse.parse(args, spec) {
///     {value} => greet(value),
///     {help: text} | {version: text} => print(text),
///     {error: message} => fatal({message, code: 2}),
/// }
/// ```
///
/// Run as `greet -s -g Hi Ada`, it prints `HI, ADA!`; as `greet --help`, its
/// generated help. Run as `greet`, missing `who`, it ends with `fatal`: the
/// `frost` command prints the message, as given, to standard error, and exits
/// with status 2, the usual status for a usage error.
///
/// A spec that is not valid is an error.
///
/// # The spec
///
/// Every key is optional.
///
/// - `name`: the program's name, in help and errors. Without it, the name is
///   the last component of the path in the first of `args`.
/// - `description`: a line or more for the top of the help.
/// - `version`: what `--version` (or `-V`) shows, after the name.
/// - `epilog`: text for the end of the help, after the options, such as
///   examples.
/// - `flags`: a Map of flags by name. A flag named `dry_run` is given as
///   `--dry-run`. Each is a Map of:
///   - `short`: one ASCII letter, given after `-`.
///   - `description`: a line for the help.
///   - `hidden`: whether to leave it out of the help. It works all the same.
/// - `options`: a Map of options by name, each taking a value, and given as a
///   flag is. Each is a Map of `short`, `description`, and `hidden`, as a
///   flag's, and:
///   - `required`: whether the option must be given. Without it, it need not.
///   - `default`: the text taken, as if given, when the option is not. Not
///     with `required: true` or `repeatable`.
///   - `repeatable`: whether the option may be given more than once.
///   - `choices`: an Array of the only text it accepts, checked before any
///     `parse`.
///   - `value_name`: what the help calls its value.
///   - `parse`: a Function that makes its value from the text (see
///     [Parsing values](#parsing-values)).
/// - `positionals`: an Array of positional arguments, in order. Each is a Map
///   of:
///   - `name`: its name (required).
///   - `description`: a line for the help.
///   - `hidden`: whether to leave it out of the help, as a flag's.
///   - `required`: whether it must be given. Without it, it must unless it
///     has a `default`. A required one cannot follow one that is not.
///   - `default`: the text taken, as if given, when it is not. Not with
///     `required: true` or `many`.
///   - `many`: whether it takes every remaining value. Only the last may.
///   - `choices`: an Array of the only text it accepts, checked before any
///     `parse`.
///   - `parse`: a Function that makes its value from the text (see
///     [Parsing values](#parsing-values)).
/// - `commands`: a Map of subcommands by name, each a spec of its own, without
///   `name`. A command with `commands` takes no `positionals`, and its command
///   line must choose one of them. Commands nest at most 32 deep. A subcommand
///   may also be `hidden`, from its parent's list of commands.
///
/// A flag's or option's name is ASCII letters, digits, and `_`, not starting
/// with `_`. A positional argument's or command's is ASCII letters, digits, `-`, and
/// `_`, not starting with `-`. No two of a command's arguments share a name or
/// a `short`, though a subcommand's may reuse its parent's. Every command
/// builds in `--help` and `-h`, and one with a `version` builds in
/// `--version` and `-V`, so the spec cannot also use them. A command with
/// `commands` cannot name a flag or option `command`, the key of its choice.
///
/// # The value
///
/// The `value` Map holds every argument the spec gives, whether or not the
/// command line gave it:
///
/// - A flag is `true` if given, else `false`.
/// - An option or positional argument is its value: its text, or what its
///   `parse` makes of it. When not given, it is its `default`'s, or Null.
/// - A `repeatable` option or `many` positional argument is an Array of its
///   values, in order, which may be empty.
/// - With `commands`, `command` is a Map of one key, the chosen command's
///   name, whose value is that command's own `value` Map.
///
/// # Parsing values
///
/// An option or positional argument with `parse` passes each of its values'
/// text to it, its `default` included, and takes what it returns. `choices`
/// are checked against the text first.
///
/// - If `parse` returns Null for the command line's text, the command line is
///   invalid, and the outcome is that `{error}`, unless the command line has
///   another error, which is reported instead.
/// - If it returns Null for the `default`, the spec is invalid: an error. The
///   `default` is parsed only when it is taken; note that this can obscure a
///   malformed default.
/// - An error it raises is raised.
///
/// The `to_*` functions work as `parse` directly, since each returns Null for
/// text it cannot convert, and a script's own function can check what it
/// accepts too:
///
/// ```frost
/// defn positive_int(text) -> to_int(text) @ and_then(fn n -> if n > 0: n else: null)
///
/// def spec = {
///     name: 'fetch',
///     options: {
///         retries: {short: 'r', default: '3', parse: positive_int, description: 'A positive Int'},
///     },
///     positionals: [{name: 'url'}],
/// }
/// ```
///
/// With this spec, `fetch -r 5 x` gives `{retries: 5, url: 'x'}` and
/// `fetch x` gives `{retries: 3, url: 'x'}`, while `fetch -r 0 x` is invalid:
/// `invalid value '0' for '--retries <retries>'`.
///
/// # Subcommands
///
/// The chosen command's name is the one key of `command`, so a script
/// matches on it:
///
/// ```frost
/// def argparse = import('ext.argparse')
///
/// def spec = {
///     name: 'todo',
///     commands: {
///         add: {
///             description: 'Add an item',
///             options: {priority: {short: 'p', choices: ['low', 'high'], default: 'low'}},
///             positionals: [{name: 'item'}],
///         },
///         done: {
///             description: 'Mark items done',
///             positionals: [{name: 'ids', many: true}],
///         },
///     },
/// }
///
/// match argparse.parse(args, spec) {
///     {value: {command: {add}}} => print($'adding ${add.item} at ${add.priority} priority'),
///     {value: {command: {done}}} => each(done.ids, fn id -> print($'done: ${id}')),
///     {help: text} | {version: text} => print(text),
///     {error: message} => fatal({message, code: 2}),
/// }
/// ```
///
/// Run as `todo add -p high 'buy milk'`, it prints
/// `adding buy milk at high priority`; as `todo done 3 4`, a line for each.
///
/// # The command line
///
/// An option's value follows it as `--name value`, `--name=value`, `-n value`,
/// or `-nvalue`. Short flags combine, as `-abc`, and the last may be an option
/// taking the next value. A value may be a negative number. After `--`, every
/// item is a positional argument, as is `-` alone.
pub fn extension() -> Extension {
    Extension::builder("argparse")
        .function(parse_frost())
        .build()
}

/// `parse(args, spec)`: how the command line `args` parses as `spec` says.
#[frost_fn]
fn parse(#[frost(ctx)] ctx: &mut NativeCtx<'_>, args: CommandLine, spec: Spec) -> FrostResult {
    let CommandLine(args) = args;
    let program = spec.name.clone().unwrap_or_else(|| program_name(&args[0]));
    let mut command = command::build(&program, &spec.command).bin_name(program);
    // Parsed in place, so that `command` keeps what clap builds as it parses,
    // such as each subcommand's name in usage, for the errors `read` words.
    let outcome = match command.try_get_matches_from_mut(args) {
        Ok(matches) => match command::read(&spec.command, &matches, &command, ctx) {
            Ok(value) => ("value", value),
            Err(ReadError::Invalid(error)) => error_outcome(&error),
            Err(ReadError::Raised(error)) => return Err(error),
        },
        Err(error) => error_outcome(&error),
    };
    Ok(string_keyed_map([outcome]))
}

/// The outcome for `error`, which is not always a failure: clap reports a
/// request for help or the version as an error too.
fn error_outcome(error: &clap::Error) -> (&'static str, Value) {
    let text = trim_trailing_whitespace(&error.render().to_string());
    match error.kind() {
        ErrorKind::DisplayHelp => ("help", text.into()),
        ErrorKind::DisplayVersion => ("version", text.into()),
        // The script decides how to present the error, so the label clap
        // starts it with is dropped.
        _ => (
            "error",
            text.strip_prefix("error: ").unwrap_or(&text).into(),
        ),
    }
}

/// `text` without the trailing whitespace clap leaves where a column is
/// empty, or at the end.
fn trim_trailing_whitespace(text: &str) -> String {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    lines.join("\n").trim_end().to_string()
}

/// The Map of `entries`, each keyed by a String.
pub(crate) fn string_keyed_map<'a>(entries: impl IntoIterator<Item = (&'a str, Value)>) -> Value {
    entries
        .into_iter()
        .map(|(key, value)| (MapKey::from(key), value))
        .collect()
}

/// The name a program run as `invoked_as` goes by: the last component of its
/// path, if it has one.
fn program_name(invoked_as: &str) -> String {
    Path::new(invoked_as).file_name().map_or_else(
        || invoked_as.to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// A command line: the program's name, then its arguments.
struct CommandLine(Vec<String>);

impl FromArg for CommandLine {
    const TYPES: TypeSet = FrostType::ARRAY;

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        let args = Vec::<String>::from_arg(value, site)?;
        if args.is_empty() {
            return Err(site.requires("non-empty, starting with the program's name"));
        }
        Ok(CommandLine(args))
    }
}
