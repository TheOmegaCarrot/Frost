//! The spec a script describes its command line with: read from Frost, then
//! checked, so that every spec that reaches clap is one clap accepts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display};

use frostlang::native::{ArgSite, FromArg, Function};
use frostlang::{
    FrostError, FrostType, MapKey, TypeSet, Value, from_value, is_identifier_like_and_not_keyword,
};
use serde::Deserialize;

/// A whole command line's spec.
pub(crate) struct Spec {
    /// The program's name, if the spec gives one.
    pub(crate) name: Option<String>,
    pub(crate) command: CommandSpec,
}

/// The spec of the program, or of one of its subcommands.
pub(crate) struct CommandSpec {
    pub(crate) description: Option<String>,
    pub(crate) version: Option<String>,
    /// Shown after the option list in help.
    pub(crate) epilog: Option<String>,
    /// Left out of its parent's list of commands in help.
    pub(crate) hidden: bool,
    /// In name order.
    pub(crate) flags_and_options: Vec<FlagOrOption>,
    pub(crate) operands: Operands,
}

/// What follows a command's flags and options: positionals, or a subcommand.
pub(crate) enum Operands {
    Positionals(Vec<Positional>),
    /// Never empty: a spec whose `commands` is empty or absent has
    /// `Positionals`, possibly none.
    Commands(BTreeMap<String, CommandSpec>),
}

pub(crate) struct FlagOrOption {
    /// Its long form, given after `--`, is the [`long_form`] of this.
    pub(crate) name: String,
    pub(crate) short: Option<char>,
    pub(crate) description: Option<String>,
    /// Left out of help.
    pub(crate) hidden: bool,
    pub(crate) kind: FlagOrOptionKind,
}

pub(crate) enum FlagOrOptionKind {
    Flag,
    Option {
        values: Values,
        value_name: Option<String>,
    },
}

pub(crate) struct Positional {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    /// Left out of help.
    pub(crate) hidden: bool,
    pub(crate) values: Values,
}

/// The values an option or positional takes.
pub(crate) struct Values {
    pub(crate) occurrence: Occurrence,
    /// The only values accepted, if limited; never empty.
    pub(crate) choices: Option<Vec<String>>,
    pub(crate) parse: Option<Parse>,
}

/// The script's function that makes an argument's value from each text it
/// takes, its default included.
pub(crate) struct Parse {
    pub(crate) function: Function,
    /// The spec error for `function` giving Null for the argument's default,
    /// if it has one. Built while the spec is read, the only place a spec error
    /// can be worded, and raised when a command line leaves the argument to its
    /// default.
    null_for_default: Option<FrostError>,
}

impl Parse {
    /// See [`Parse::null_for_default`](#structfield.null_for_default).
    pub(crate) fn null_for_default(&self) -> FrostError {
        self.null_for_default
            .clone()
            .expect("only an argument with a default can take it")
    }
}

/// Words a problem with the spec as the error the native raises for it.
type WordSpecError<'a> = &'a dyn Fn(&SpecError) -> FrostError;

/// How many values an option or positional takes, and what it is without one.
pub(crate) enum Occurrence {
    /// Exactly one.
    Required,
    /// One or none.
    Optional,
    /// One, or else this.
    Defaulted(String),
    /// Any number, or at least one if `required`.
    Repeated { required: bool },
}

/// The name a parse's result holds the chosen subcommand under.
pub(crate) const COMMAND: &str = "command";

/// The names clap gives the arguments it builds in.
const HELP: &str = "help";
const VERSION: &str = "version";
const HELP_SHORT: char = 'h';
const VERSION_SHORT: char = 'V';

/// The long form of the flag or option named `name`, given after `--`.
pub(crate) fn long_form(name: &str) -> String {
    name.replace('_', "-")
}

/// The deepest a spec's `commands` may nest: far deeper than any command line
/// needs, and shallow enough that reading the spec, and clap building and
/// parsing its command, stay well within a small thread stack.
const MAX_COMMAND_DEPTH: usize = 32;

impl FromArg for Spec {
    const TYPES: TypeSet = FrostType::MAP;

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        let invalid = |problem: &dyn Display| site.requires(format_args!("valid: {problem}"));
        // Before anything recurses through the spec.
        check_command_depth(&value).map_err(|error| invalid(&error))?;
        let raw: RawCommand = from_value(value).map_err(|error| invalid(&error.message()))?;
        if raw.hidden {
            let place = Place::default().key("hidden");
            return Err(invalid(&place.problem(
                "only a command in `commands` is `hidden`, from its parent's help",
            )));
        }
        let name = raw.name.clone();
        let word: WordSpecError<'_> = &|error| invalid(error);
        let command =
            check_command(raw, &Place::default(), word).map_err(|error| invalid(&error))?;
        Ok(Spec { name, command })
    }
}

// -- The spec as a script writes it --

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCommand {
    name: Option<String>,
    description: Option<String>,
    version: Option<String>,
    epilog: Option<String>,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    flags: BTreeMap<String, RawFlag>,
    #[serde(default)]
    options: BTreeMap<String, RawOption>,
    #[serde(default)]
    positionals: Vec<RawPositional>,
    #[serde(default)]
    commands: BTreeMap<String, RawCommand>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFlag {
    short: Option<String>,
    description: Option<String>,
    #[serde(default)]
    hidden: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOption {
    short: Option<String>,
    description: Option<String>,
    #[serde(default)]
    hidden: bool,
    required: Option<bool>,
    #[serde(rename = "default")]
    default_value: Option<String>,
    #[serde(default)]
    repeatable: bool,
    choices: Option<Vec<String>>,
    value_name: Option<String>,
    parse: Option<Function>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPositional {
    name: String,
    description: Option<String>,
    #[serde(default)]
    hidden: bool,
    required: Option<bool>,
    #[serde(rename = "default")]
    default_value: Option<String>,
    #[serde(default)]
    many: bool,
    choices: Option<Vec<String>>,
    parse: Option<Function>,
}

// -- Places in the spec --

/// A place in the spec, written as Frost would index along it from the top,
/// as the serde bridge writes the places of the errors it finds.
// TODO: write places with `frostlang`'s own path writer once it is public
// (FROST-274), rather than this copy of it.
#[derive(Clone, Default)]
struct Place(Vec<Segment>);

#[derive(Clone)]
enum Segment {
    Key(String),
    Index(usize),
}

impl Place {
    fn key(&self, key: &str) -> Place {
        let mut place = self.clone();
        place.0.push(Segment::Key(key.to_string()));
        place
    }

    fn index(&self, index: usize) -> Place {
        let mut place = self.clone();
        place.0.push(Segment::Index(index));
        place
    }

    /// The error that `problem` is at this place.
    fn problem(&self, problem: impl Display) -> SpecError {
        SpecError {
            problem: problem.to_string(),
            place: self.clone(),
        }
    }
}

impl Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.0.iter().enumerate() {
            match segment {
                Segment::Key(key) if is_identifier_like_and_not_keyword(key) => {
                    if i > 0 {
                        f.write_str(".")?;
                    }
                    f.write_str(key)?;
                }
                Segment::Key(key) => write!(f, "[{}]", quoted(key))?,
                Segment::Index(index) => write!(f, "[{index}]")?,
            }
        }
        Ok(())
    }
}

/// `text` as a script would write it, quoted.
fn quoted(text: &str) -> String {
    Value::from(text).to_debug_string()
}

/// What is wrong with a spec, and where.
struct SpecError {
    problem: String,
    place: Place,
}

impl Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.problem)?;
        if !self.place.0.is_empty() {
            write!(f, " (at `{}`)", self.place)?;
        }
        Ok(())
    }
}

// -- Names --

/// The rule a name in a spec must follow, which depends on what it names.
#[derive(Clone, Copy)]
enum NameRule {
    /// ASCII letters, digits, and `_`, not starting with `_`. A flag's or
    /// option's long form writes each `_` of its name as `-`, so a script
    /// writes `dry_run` for `--dry-run`, and the result's key needs no
    /// brackets.
    FlagOrOption,
    /// ASCII letters, digits, `-`, and `_`, not starting with `-`.
    PositionalOrCommand,
}

/// Whether `name` can name a flag or option.
fn is_flag_or_option_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `name` can name a positional or command.
fn is_positional_or_command_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Check that `name` follows `rule`. clap would take a leading `-` as part of
/// the option syntax, and other characters would make awkward command lines.
fn check_name(name: &str, rule: NameRule, place: &Place) -> Result<(), SpecError> {
    match rule {
        NameRule::FlagOrOption if is_flag_or_option_name(name) => Ok(()),
        NameRule::FlagOrOption => {
            let mut problem = format!(
                "{} is not a flag or option name: one is ASCII letters, digits, and `_`, \
                 not starting with `_`",
                quoted(name)
            );
            let underscored = name.replace('-', "_");
            if is_flag_or_option_name(&underscored) {
                problem.push_str(&format!(
                    "; {} gives the long form `--{name}`",
                    quoted(&underscored)
                ));
            }
            Err(place.problem(problem))
        }
        NameRule::PositionalOrCommand if is_positional_or_command_name(name) => Ok(()),
        NameRule::PositionalOrCommand => Err(place.problem(format_args!(
            "{} is not a name: a name is ASCII letters, digits, `-`, and `_`, \
             not starting with `-`",
            quoted(name)
        ))),
    }
}

/// The names and short names a command's arguments may not take: those it
/// reserves, and those its arguments have taken so far.
struct TakenNames {
    /// Each reserved name, with why it is.
    reserved_names: BTreeMap<&'static str, &'static str>,
    reserved_shorts: BTreeMap<char, &'static str>,
    names: BTreeSet<String>,
    shorts: BTreeSet<char>,
}

impl TakenNames {
    /// The names `raw` reserves for what clap builds in and for the result.
    fn reserved_by(raw: &RawCommand) -> TakenNames {
        let mut reserved_names = BTreeMap::from([(HELP, "`help` is built in")]);
        let mut reserved_shorts = BTreeMap::from([(HELP_SHORT, "`-h` is built in")]);
        if raw.version.is_some() {
            reserved_names.insert(
                VERSION,
                "`version` is built in when the spec has a `version`",
            );
            reserved_shorts.insert(
                VERSION_SHORT,
                "`-V` is built in when the spec has a `version`",
            );
        }
        if !raw.commands.is_empty() {
            reserved_names.insert(
                COMMAND,
                "`command` holds the chosen command when the spec has `commands`",
            );
        }
        TakenNames {
            reserved_names,
            reserved_shorts,
            names: BTreeSet::new(),
            shorts: BTreeSet::new(),
        }
    }

    fn take_name(&mut self, name: &str, rule: NameRule, place: &Place) -> Result<(), SpecError> {
        check_name(name, rule, place)?;
        if let Some(reason) = self.reserved_names.get(name) {
            return Err(place.problem(reason));
        }
        if !self.names.insert(name.to_string()) {
            return Err(place.problem(format_args!("`{name}` names more than one argument")));
        }
        Ok(())
    }

    fn take_short(
        &mut self,
        short: Option<String>,
        place: &Place,
    ) -> Result<Option<char>, SpecError> {
        let Some(short) = short else {
            return Ok(None);
        };
        let place = place.key("short");
        let mut chars = short.chars();
        // Not a digit: `-1` may be a negative number given as a value.
        let short = match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_ascii_alphabetic() => c,
            _ => return Err(place.problem("`short` is one ASCII letter")),
        };
        if let Some(reason) = self.reserved_shorts.get(&short) {
            return Err(place.problem(reason));
        }
        if !self.shorts.insert(short) {
            return Err(place.problem(format_args!("`-{short}` names more than one argument")));
        }
        Ok(Some(short))
    }
}

// -- Checking --

/// Check that the commands of `spec`, a spec as a script wrote it, nest at most
/// [`MAX_COMMAND_DEPTH`] deep. It walks the spec without recursing, as a spec
/// nested deeply enough would overflow the stack of anything that recursed.
/// Only nesting is checked: whatever is not a Map of commands where one would
/// be is left for reading the spec to reject.
fn check_command_depth(spec: &Value) -> Result<(), SpecError> {
    let commands_key = MapKey::from("commands");
    let mut pending = vec![(spec, Place::default(), 0)];
    while let Some((command, place, depth)) = pending.pop() {
        let Value::Map(command) = command else {
            continue;
        };
        let Some(Value::Map(commands)) = command.get(&commands_key) else {
            continue;
        };
        let place = place.key("commands");
        for (name, subcommand) in commands {
            let MapKey::String(name) = name else {
                continue;
            };
            let place = place.key(name.as_str());
            if depth == MAX_COMMAND_DEPTH {
                return Err(place.problem(format_args!(
                    "commands nest at most {MAX_COMMAND_DEPTH} deep"
                )));
            }
            pending.push((subcommand, place, depth + 1));
        }
    }
    Ok(())
}

fn check_command(
    raw: RawCommand,
    place: &Place,
    word: WordSpecError<'_>,
) -> Result<CommandSpec, SpecError> {
    let mut taken = TakenNames::reserved_by(&raw);
    let RawCommand {
        name: _,
        description,
        version,
        epilog,
        hidden,
        flags,
        options,
        positionals,
        commands,
    } = raw;
    let mut flags_and_options = check_flags(flags, &mut taken, place)?;
    flags_and_options.extend(check_options(options, &mut taken, place, word)?);
    flags_and_options.sort_by(|a, b| a.name.cmp(&b.name));
    let operands = match (commands.is_empty(), positionals.is_empty()) {
        (true, _) => {
            Operands::Positionals(check_positionals(positionals, &mut taken, place, word)?)
        }
        (false, true) => Operands::Commands(check_commands(commands, place, word)?),
        (false, false) => {
            return Err(place
                .key("positionals")
                .problem("a command with `commands` takes no `positionals`"));
        }
    };
    Ok(CommandSpec {
        description,
        version,
        epilog,
        hidden,
        flags_and_options,
        operands,
    })
}

fn check_flags(
    flags: BTreeMap<String, RawFlag>,
    taken: &mut TakenNames,
    place: &Place,
) -> Result<Vec<FlagOrOption>, SpecError> {
    let mut checked = Vec::new();
    for (name, flag) in flags {
        let place = place.key("flags").key(&name);
        taken.take_name(&name, NameRule::FlagOrOption, &place)?;
        checked.push(FlagOrOption {
            short: taken.take_short(flag.short, &place)?,
            description: flag.description,
            hidden: flag.hidden,
            kind: FlagOrOptionKind::Flag,
            name,
        });
    }
    Ok(checked)
}

fn check_options(
    options: BTreeMap<String, RawOption>,
    taken: &mut TakenNames,
    place: &Place,
    word: WordSpecError<'_>,
) -> Result<Vec<FlagOrOption>, SpecError> {
    let mut checked = Vec::new();
    for (name, option) in options {
        let place = place.key("options").key(&name);
        taken.take_name(&name, NameRule::FlagOrOption, &place)?;
        let short = taken.take_short(option.short, &place)?;
        let values = check_values(
            ValuesSpec {
                required: option.required,
                default_value: option.default_value,
                repeated: option.repeatable,
                choices: option.choices,
                parse: option.parse,
            },
            ValueArgument::Option,
            &place,
            word,
        )?;
        checked.push(FlagOrOption {
            short,
            description: option.description,
            hidden: option.hidden,
            kind: FlagOrOptionKind::Option {
                values,
                value_name: option.value_name,
            },
            name,
        });
    }
    Ok(checked)
}

fn check_positionals(
    positionals: Vec<RawPositional>,
    taken: &mut TakenNames,
    place: &Place,
    word: WordSpecError<'_>,
) -> Result<Vec<Positional>, SpecError> {
    let count = positionals.len();
    let mut checked = Vec::new();
    let mut follows_optional = false;
    for (index, positional) in positionals.into_iter().enumerate() {
        let place = place.key("positionals").index(index);
        taken.take_name(
            &positional.name,
            NameRule::PositionalOrCommand,
            &place.key("name"),
        )?;
        if positional.many && index + 1 < count {
            return Err(place.problem("only the last positional argument may be `many`"));
        }
        let values = check_values(
            ValuesSpec {
                required: positional.required,
                default_value: positional.default_value,
                repeated: positional.many,
                choices: positional.choices,
                parse: positional.parse,
            },
            ValueArgument::Positional,
            &place,
            word,
        )?;
        let required = matches!(
            values.occurrence,
            Occurrence::Required | Occurrence::Repeated { required: true }
        );
        if required && follows_optional {
            return Err(
                place.problem("a required positional argument cannot follow an optional one")
            );
        }
        follows_optional |= !required;
        checked.push(Positional {
            name: positional.name,
            description: positional.description,
            hidden: positional.hidden,
            values,
        });
    }
    Ok(checked)
}

fn check_commands(
    commands: BTreeMap<String, RawCommand>,
    place: &Place,
    word: WordSpecError<'_>,
) -> Result<BTreeMap<String, CommandSpec>, SpecError> {
    let mut checked = BTreeMap::new();
    for (name, command) in commands {
        let place = place.key("commands").key(&name);
        check_name(&name, NameRule::PositionalOrCommand, &place)?;
        if command.name.is_some() {
            return Err(place
                .key("name")
                .problem("a command in `commands` is named by its key, so takes no `name`"));
        }
        let command = check_command(command, &place, word)?;
        checked.insert(name, command);
    }
    Ok(checked)
}

/// The keys an option and a positional share for the values they take.
struct ValuesSpec {
    required: Option<bool>,
    default_value: Option<String>,
    repeated: bool,
    choices: Option<Vec<String>>,
    parse: Option<Function>,
}

/// A kind of argument that takes values, which decides how its spec asks for
/// any number of them and whether it is required when the spec does not say.
#[derive(Clone, Copy)]
enum ValueArgument {
    /// Asks with `repeatable`, and is optional by default.
    Option,
    /// Asks with `many`, and is required by default.
    Positional,
}

impl ValueArgument {
    fn repeat_key(self) -> &'static str {
        match self {
            ValueArgument::Option => "repeatable",
            ValueArgument::Positional => "many",
        }
    }

    fn required_by_default(self) -> bool {
        matches!(self, ValueArgument::Positional)
    }

    /// The argument, as error messages name it.
    fn noun(self) -> &'static str {
        match self {
            ValueArgument::Option => "an option",
            ValueArgument::Positional => "a positional argument",
        }
    }
}

/// The spec error for a `parse` giving Null for the default of an argument
/// with `occurrence` at `place`, if it has a default.
fn null_for_default(
    occurrence: &Occurrence,
    place: &Place,
    word: WordSpecError<'_>,
) -> Option<FrostError> {
    let Occurrence::Defaulted(text) = occurrence else {
        return None;
    };
    let problem = format!("`parse` gives Null for `default` {}", quoted(text));
    Some(word(&place.key("default").problem(problem)))
}

fn check_values(
    spec: ValuesSpec,
    argument: ValueArgument,
    place: &Place,
    word: WordSpecError<'_>,
) -> Result<Values, SpecError> {
    let ValuesSpec {
        required,
        default_value,
        repeated,
        choices,
        parse,
    } = spec;
    if let Some(choices) = &choices {
        let place = place.key("choices");
        if choices.is_empty() {
            return Err(place.problem("`choices` is empty"));
        }
        let mut seen = BTreeSet::new();
        if let Some(twice) = choices.iter().find(|choice| !seen.insert(*choice)) {
            return Err(place.problem(format_args!(
                "`choices` holds {} more than once",
                quoted(twice)
            )));
        }
        if let Some(default_value) = &default_value
            && !choices.contains(default_value)
        {
            return Err(place.problem(format_args!(
                "`default` {} is not among `choices`",
                quoted(default_value)
            )));
        }
    }
    // A `default` makes an argument optional, so only an explicit `required`
    // conflicts with one.
    let required_without_default = required.unwrap_or(argument.required_by_default());
    let exclusive_with_default = |key: &str| {
        place.problem(format_args!(
            "{} takes `default` or `{key}: true`, not both",
            argument.noun()
        ))
    };
    let occurrence = match (repeated, default_value) {
        (true, Some(_)) => return Err(exclusive_with_default(argument.repeat_key())),
        (true, None) => Occurrence::Repeated {
            required: required_without_default,
        },
        (false, Some(_)) if required == Some(true) => {
            return Err(exclusive_with_default("required"));
        }
        (false, Some(default_value)) => Occurrence::Defaulted(default_value),
        (false, None) if required_without_default => Occurrence::Required,
        (false, None) => Occurrence::Optional,
    };
    let parse = parse.map(|function| Parse {
        function,
        null_for_default: null_for_default(&occurrence, place, word),
    });
    Ok(Values {
        occurrence,
        choices,
        parse,
    })
}
