//! Many specs, generated from a fixed seed out of the parts most likely to
//! break a rule: reserved and invalid names, clashing short names, every mix
//! of `required`, `default`, `repeatable`, `many`, and `choices`, nesting, and
//! positionals beside commands. Each is parsed for the help of every command
//! in it, and for command lines made of its own names and odd values.
//!
//! Whatever the spec, `parse` must give an outcome or raise its spec error;
//! nothing else. These tests build in debug, where clap checks each command it
//! builds with assertions that panic, so a spec that passed the extension's
//! checks yet broke one of clap's rules would fail here, where in release it
//! would misbehave.

use std::fmt::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};

use frostlang::{MapKey, Value};

use crate::common::run;

/// How many specs each seed generates.
const SPECS_PER_SEED: usize = 400;

/// The seeds, fixed so every run checks the same specs.
const SEEDS: [u64; 3] = [1, 0x5eed, 0xa59a_95e0];

/// The deepest a generated command nests.
const MAX_DEPTH: usize = 3;

/// Names that follow every rule, few enough that they collide often.
const NAMES: &[&str] = &[
    "a", "b", "c", "x", "v", "V", "h", "dry_run", "9", "a_b", "x_", "if", "String",
];

/// Names some command reserves, or some rule rejects.
const RESERVED_NAMES: &[&str] = &["help", "version", "command"];
const BAD_NAMES: &[&str] = &["in-file", "_x", "-x", "", "a b", "caf\u{e9}", "--"];

/// Short names: valid, built in, and not one ASCII letter.
const SHORTS: &[&str] = &["a", "b", "x", "v", "A", "z"];
const RESERVED_SHORTS: &[&str] = &["h", "V"];
const BAD_SHORTS: &[&str] = &["1", "ab", "", "-", "\u{e9}"];

/// Values, for defaults, choices, and command lines, that look like
/// command-line syntax.
const VALUES: &[&str] = &["d", "e", "-1", "--", "-", "", "-x", "--y", "=", "a b"];

/// A generated command, kept as parts so that the command lines generated for
/// it can use its names.
struct Command {
    /// The Frost source of its spec.
    source: String,
    /// The tokens that name its flags and options on a command line.
    tokens: Vec<String>,
    /// Its subcommands, by name.
    commands: Vec<(String, Command)>,
}

struct Generator {
    rng: fastrand::Rng,
}

impl Generator {
    /// One time in `n`.
    fn one_in(&mut self, n: u32) -> bool {
        self.rng.u32(0..n) == 0
    }

    fn pick(&mut self, items: &[&str]) -> String {
        items[self.rng.usize(0..items.len())].to_string()
    }

    /// A name: now and then one a command reserves or some rule rejects.
    fn name(&mut self) -> String {
        if self.one_in(40) {
            self.pick(BAD_NAMES)
        } else if self.one_in(30) {
            self.pick(RESERVED_NAMES)
        } else {
            self.pick(NAMES)
        }
    }

    /// Up to `max` distinct names; a Map literal takes each key once.
    fn names(&mut self, max: usize) -> Vec<String> {
        let mut names = Vec::new();
        for _ in 0..self.rng.usize(0..=max) {
            let name = self.name();
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }

    fn value(&mut self) -> String {
        self.pick(VALUES)
    }

    /// `key: <Frost source>` one time in `n`, else nothing.
    fn sometimes(
        &mut self,
        n: u32,
        key: &str,
        source: impl FnOnce(&mut Self) -> String,
    ) -> Option<String> {
        self.one_in(n).then(|| format!("{key}: {}", source(self)))
    }

    fn bool_source(&mut self) -> String {
        self.rng.bool().to_string()
    }

    fn short(&mut self) -> Option<String> {
        if !self.one_in(2) {
            return None;
        }
        Some(if self.one_in(30) {
            self.pick(BAD_SHORTS)
        } else if self.one_in(20) {
            self.pick(RESERVED_SHORTS)
        } else {
            self.pick(SHORTS)
        })
    }

    fn choices_source(&mut self) -> String {
        let count = if self.one_in(15) {
            0
        } else {
            self.rng.usize(1..4)
        };
        let choices: Vec<String> = (0..count).map(|_| quoted(&self.value())).collect();
        format!("[{}]", choices.join(", "))
    }

    /// The keys an option and a positional share for the values they take,
    /// with `repeat_key` naming the key asking for any number of them.
    fn value_keys(&mut self, repeat_key: &str) -> Vec<String> {
        [
            self.sometimes(4, "required", Self::bool_source),
            self.sometimes(4, "default", |g| quoted(&g.value())),
            self.sometimes(4, repeat_key, Self::bool_source),
            self.sometimes(5, "choices", Self::choices_source),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn command(&mut self, depth: usize) -> Command {
        let mut keys = Vec::new();
        let mut tokens = Vec::new();
        if let Some(version) = self.sometimes(4, "version", |g| quoted(&g.value())) {
            keys.push(version);
            tokens.extend(["--version".to_string(), "-V".to_string()]);
        }
        if let Some(name) = self.sometimes(30, "name", |_| quoted("nested")) {
            keys.push(name);
        }

        let mut flags = Vec::new();
        for name in self.names(3) {
            let short = self.short();
            tokens.extend(named_tokens(&name, short.as_deref()));
            let short = short.map(|short| format!("short: {}", quoted(&short)));
            flags.push(format!(
                "[{}]: {{{}}}",
                quoted(&name),
                short.unwrap_or_default()
            ));
        }
        keys.push(format!("flags: {{{}}}", flags.join(", ")));

        let mut options = Vec::new();
        for name in self.names(3) {
            let short = self.short();
            tokens.extend(named_tokens(&name, short.as_deref()));
            let mut option_keys = self.value_keys("repeatable");
            option_keys.extend(short.map(|short| format!("short: {}", quoted(&short))));
            option_keys.extend(self.sometimes(4, "value_name", |g| quoted(&g.value())));
            options.push(format!(
                "[{}]: {{{}}}",
                quoted(&name),
                option_keys.join(", ")
            ));
        }
        keys.push(format!("options: {{{}}}", options.join(", ")));

        let mut commands = Vec::new();
        let has_commands = depth < MAX_DEPTH && self.one_in(3);
        if has_commands {
            for name in self.names(3) {
                let command = self.command(depth + 1);
                commands.push((name, command));
            }
            let sources: Vec<String> = commands
                .iter()
                .map(|(name, command)| format!("[{}]: {}", quoted(name), command.source))
                .collect();
            keys.push(format!("commands: {{{}}}", sources.join(", ")));
        }
        // Now and then both, which no spec may have.
        if !has_commands || self.one_in(20) {
            let positionals: Vec<String> = (0..self.rng.usize(0..4))
                .map(|_| {
                    let mut positional_keys = vec![format!("name: {}", quoted(&self.name()))];
                    positional_keys.extend(self.value_keys("many"));
                    format!("{{{}}}", positional_keys.join(", "))
                })
                .collect();
            keys.push(format!("positionals: [{}]", positionals.join(", ")));
        }

        Command {
            source: format!("{{{}}}", keys.join(", ")),
            tokens,
            commands,
        }
    }

    /// A command line for `command`: a path into its subcommands, then tokens
    /// of the last command along it and odd values, mixed.
    fn command_line(&mut self, command: &Command) -> Vec<String> {
        let mut line = Vec::new();
        let mut current = command;
        while !current.commands.is_empty() && !self.one_in(4) {
            let (name, next) = &current.commands[self.rng.usize(0..current.commands.len())];
            line.push(name.clone());
            current = next;
        }
        for _ in 0..self.rng.usize(0..6) {
            let token = match self.rng.u32(0..4) {
                0 | 1 if !current.tokens.is_empty() => {
                    current.tokens[self.rng.usize(0..current.tokens.len())].clone()
                }
                2 => format!("--x={}", self.value()),
                _ => self.value(),
            };
            line.push(token);
        }
        line
    }
}

/// The tokens naming a flag or option `name` with `short`, valid or not: its
/// long form, its name as written, and its short name.
fn named_tokens(name: &str, short: Option<&str>) -> Vec<String> {
    let mut tokens = vec![format!("--{}", name.replace('_', "-")), format!("--{name}")];
    tokens.extend(short.map(|short| format!("-{short}")));
    tokens
}

/// The path to every command in `command`, itself included as the empty path.
fn command_paths(command: &Command) -> Vec<Vec<String>> {
    let mut paths = vec![Vec::new()];
    for (name, sub) in &command.commands {
        for mut path in command_paths(sub) {
            path.insert(0, name.clone());
            paths.push(path);
        }
    }
    paths
}

fn quoted(text: &str) -> String {
    Value::from(text).to_debug_string()
}

/// A Frost Array of the program name, then `arguments`.
fn args_source(arguments: &[String]) -> String {
    let quoted: Vec<String> = std::iter::once("prog")
        .chain(arguments.iter().map(String::as_str))
        .map(quoted)
        .collect();
    format!("[{}]", quoted.join(", "))
}

/// What came of parsing a generated spec.
#[derive(Default)]
struct Tally {
    invalid_specs: usize,
    valid_specs: usize,
    outcomes: [usize; 4],
}

const OUTCOME_KEYS: [&str; 4] = ["value", "help", "version", "error"];

/// Parse every command line in `lines` as `spec`, all in one script, and
/// check what comes of it.
fn check(spec: &str, lines: &[Vec<String>], tally: &mut Tally) {
    let mut script = format!("def spec = {spec}\n[");
    for line in lines {
        write!(script, "argparse.parse({}, spec), ", args_source(line)).unwrap();
    }
    script.push(']');
    let ran = catch_unwind(AssertUnwindSafe(|| run(&script)))
        .unwrap_or_else(|_| panic!("parsing panicked; the script was:\n{script}"));
    match ran {
        Err(error) => {
            let message = error.message();
            assert!(
                message.starts_with(
                    "Function ext.argparse.parse requires argument 2 (spec) to be valid: "
                ),
                "only a spec error may be raised, but got {message:?}; the script was:\n{script}"
            );
            tally.invalid_specs += 1;
        }
        Ok(Value::Array(outcomes)) => {
            tally.valid_specs += 1;
            for outcome in &outcomes {
                let key = match outcome {
                    Value::Map(map) if map.len() == 1 => map.iter().next().map(|(key, _)| key),
                    _ => None,
                };
                let index = OUTCOME_KEYS
                    .iter()
                    .position(|name| key == Some(&MapKey::from(*name)))
                    .unwrap_or_else(|| {
                        panic!("{outcome:?} is not an outcome; the script was:\n{script}")
                    });
                tally.outcomes[index] += 1;
            }
        }
        Ok(other) => panic!("the script gives an Array, but gave {other:?}:\n{script}"),
    }
}

#[test]
fn every_generated_spec_gives_outcomes_or_raises_its_spec_error() {
    let mut tally = Tally::default();
    for seed in SEEDS {
        let mut generator = Generator {
            rng: fastrand::Rng::with_seed(seed),
        };
        for _ in 0..SPECS_PER_SEED {
            let command = generator.command(0);
            let mut lines: Vec<Vec<String>> = command_paths(&command)
                .into_iter()
                .map(|mut path| {
                    path.push("--help".to_string());
                    path
                })
                .collect();
            lines.extend((0..8).map(|_| generator.command_line(&command)));
            check(&command.source, &lines, &mut tally);
        }
    }
    // Neither kind of spec, nor any kind of outcome, may be so rare that the
    // test checks little of it.
    let specs = tally.valid_specs + tally.invalid_specs;
    assert!(
        tally.valid_specs * 4 >= specs && tally.invalid_specs * 10 >= specs,
        "{} valid and {} invalid of {specs} specs",
        tally.valid_specs,
        tally.invalid_specs
    );
    for (key, count) in OUTCOME_KEYS.iter().zip(tally.outcomes) {
        assert!(count >= 50, "only {count} `{key}` outcomes");
    }
}
