//! Specs that are not valid: a mistake in the script, so an error, where a
//! command line that is not valid is an outcome.

use crate::common::{assert_invalid_specs, parsed, value};

#[test]
fn the_empty_spec_is_valid() {
    assert_eq!(parsed("{}", &[]), value("{}"));
}

#[test]
fn a_spec_holds_only_its_keys_with_their_types() {
    assert_invalid_specs(&[
        (
            "{flag: {}}",
            "unknown field `flag`, expected one of `name`, `description`, `version`, \
             `epilog`, `hidden`, `flags`, `options`, `positionals`, `commands` (at a key)",
        ),
        ("{name: 1}", "expected String, got Int (at `name`)"),
        ("{flags: []}", "expected Map, got Array (at `flags`)"),
        ("{flags: null}", "expected Map, got Null (at `flags`)"),
        (
            "{positionals: {}}",
            "expected Array, got Map (at `positionals`)",
        ),
        (
            "{commands: {a: 1}}",
            "expected Map, got Int (at `commands.a`)",
        ),
        (
            "{flags: {v: {long: 'verbose'}}}",
            "unknown field `long`, expected one of `short`, `description`, `hidden` \
             (at a key of `flags.v`)",
        ),
        (
            "{options: {n: {required: 'yes'}}}",
            "expected Bool, got String (at `options.n.required`)",
        ),
        (
            "{options: {n: {default: 3}}}",
            "expected String, got Int (at `options.n.default`)",
        ),
        (
            "{options: {n: {choices: ['a', 1]}}}",
            "expected String, got Int (at `options.n.choices[1]`)",
        ),
        (
            "{positionals: [{}]}",
            "missing field `name` (at `positionals[0]`)",
        ),
        (
            "{positionals: [{name: 'a', repeatable: true}]}",
            "unknown field `repeatable`, expected one of `name`, `description`, `hidden`, \
             `required`, `default`, `many`, `choices`, `parse` (at a key of `positionals[0]`)",
        ),
        (
            "{options: {n: {many: true}}}",
            "unknown field `many`, expected one of `short`, `description`, `hidden`, \
             `required`, `default`, `repeatable`, `choices`, `value_name`, `parse` \
             (at a key of `options.n`)",
        ),
    ]);
}

#[test]
fn a_key_holding_null_is_present_not_absent() {
    assert_invalid_specs(&[
        (
            "{description: null}",
            "expected String, got Null (at `description`)",
        ),
        (
            "{flags: {v: {short: null}}}",
            "expected String, got Null (at `flags.v.short`)",
        ),
        (
            "{positionals: [{name: 'a', required: null}]}",
            "expected Bool, got Null (at `positionals[0].required`)",
        ),
    ]);
}

#[test]
fn flag_and_option_names_are_ascii_letters_digits_and_underscores_not_leading_with_one() {
    let problem = "is not a flag or option name: one is ASCII letters, digits, and `_`, \
                   not starting with `_`";
    assert_invalid_specs(&[
        (
            "{flags: {['']: {}}}",
            &format!("\"\" {problem} (at `flags[\"\"]`)"),
        ),
        (
            "{flags: {_: {}}}",
            &format!("\"_\" {problem} (at `flags._`)"),
        ),
        (
            "{flags: {_x: {}}}",
            &format!("\"_x\" {problem} (at `flags._x`)"),
        ),
        (
            "{options: {['a b']: {}}}",
            &format!("\"a b\" {problem} (at `options[\"a b\"]`)"),
        ),
        (
            "{options: {['a=b']: {}}}",
            &format!("\"a=b\" {problem} (at `options[\"a=b\"]`)"),
        ),
        (
            "{options: {['caf\\u{e9}']: {}}}",
            &format!("\"caf\u{e9}\" {problem} (at `options[\"caf\u{e9}\"]`)"),
        ),
    ]);
    // Each of these is a flag or option name.
    assert_eq!(
        parsed(
            "{flags: {a_b: {}, ['9']: {}, x_: {}, Up: {}}, options: {n__m: {}}}",
            &["--9", "--x-", "--n--m", "v"],
        ),
        value("{a_b: false, ['9']: true, x_: true, Up: false, n__m: 'v'}"),
    );
}

#[test]
fn a_dashed_flag_or_option_name_is_told_its_underscored_name() {
    let problem = "is not a flag or option name: one is ASCII letters, digits, and `_`, \
                   not starting with `_`";
    assert_invalid_specs(&[
        (
            "{flags: {['dry-run']: {}}}",
            &format!(
                "\"dry-run\" {problem}; \"dry_run\" gives the long form `--dry-run` \
                 (at `flags[\"dry-run\"]`)"
            ),
        ),
        (
            "{options: {['max-line-count']: {}}}",
            &format!(
                "\"max-line-count\" {problem}; \"max_line_count\" gives the long form \
                 `--max-line-count` (at `options[\"max-line-count\"]`)"
            ),
        ),
        // No hint where underscores would not make a name either.
        (
            "{flags: {['--verbose']: {}}}",
            &format!("\"--verbose\" {problem} (at `flags[\"--verbose\"]`)"),
        ),
        (
            "{flags: {['a-b c']: {}}}",
            &format!("\"a-b c\" {problem} (at `flags[\"a-b c\"]`)"),
        ),
    ]);
}

#[test]
fn positional_and_command_names_are_ascii_letters_digits_dashes_and_underscores() {
    let problem = "is not a name: a name is ASCII letters, digits, `-`, and `_`, \
                   not starting with `-`";
    assert_invalid_specs(&[
        (
            "{positionals: [{name: ''}]}",
            &format!("\"\" {problem} (at `positionals[0].name`)"),
        ),
        (
            "{positionals: [{name: 'caf\\u{e9}'}]}",
            &format!("\"caf\u{e9}\" {problem} (at `positionals[0].name`)"),
        ),
        (
            "{positionals: [{name: '-x'}]}",
            &format!("\"-x\" {problem} (at `positionals[0].name`)"),
        ),
        (
            "{commands: {['-x']: {}}}",
            &format!("\"-x\" {problem} (at `commands[\"-x\"]`)"),
        ),
        (
            "{commands: {['a b']: {}}}",
            &format!("\"a b\" {problem} (at `commands[\"a b\"]`)"),
        ),
    ]);
    // Each of these is a positional's or command's name.
    assert_eq!(
        parsed(
            "{positionals: [{name: 'in-file'}, {name: '_'}, {name: '9'}]}",
            &["a", "b", "c"],
        ),
        value("{['in-file']: 'a', _: 'b', ['9']: 'c'}"),
    );
    assert_eq!(
        parsed("{commands: {['set-url']: {}, _x: {}}}", &["set-url"]),
        value("{command: {['set-url']: {}}}"),
    );
}

#[test]
fn a_short_name_is_one_ascii_letter() {
    let specs = ["''", "'ab'", "'-'", "'?'", "' '", "'\\u{e9}'", "'7'", "'0'"]
        .map(|short| format!("{{flags: {{a: {{short: {short}}}}}}}"));
    let cases = specs
        .iter()
        .map(|spec| {
            (
                spec.as_str(),
                "`short` is one ASCII letter (at `flags.a.short`)",
            )
        })
        .collect::<Vec<_>>();
    assert_invalid_specs(&cases);
    assert_eq!(
        parsed("{flags: {a: {short: 'A'}, b: {short: 'z'}}}", &["-Az"]),
        value("{a: true, b: true}"),
    );
}

#[test]
fn no_short_name_can_be_mistaken_for_a_negative_number() {
    // A digit short would be ambiguous with a negative-number value for a
    // waiting positional, so `short` takes none.
    assert_invalid_specs(&[(
        "{options: {n: {short: '1'}}, positionals: [{name: 'p', required: false}]}",
        "`short` is one ASCII letter (at `options.n.short`)",
    )]);
    assert_eq!(
        parsed(
            "{flags: {one: {}}, positionals: [{name: 'p', required: false}]}",
            &["-1"]
        ),
        value("{one: false, p: '-1'}"),
    );
}

#[test]
fn no_two_arguments_share_a_name() {
    assert_invalid_specs(&[
        (
            "{flags: {x: {}}, options: {x: {}}}",
            "`x` names more than one argument (at `options.x`)",
        ),
        (
            "{flags: {x: {}}, positionals: [{name: 'x'}]}",
            "`x` names more than one argument (at `positionals[0].name`)",
        ),
        (
            "{options: {x: {}}, positionals: [{name: 'x'}]}",
            "`x` names more than one argument (at `positionals[0].name`)",
        ),
        (
            "{positionals: [{name: 'x'}, {name: 'x'}]}",
            "`x` names more than one argument (at `positionals[1].name`)",
        ),
    ]);
}

#[test]
fn no_two_arguments_share_a_short_name() {
    assert_invalid_specs(&[
        (
            "{flags: {a: {short: 'x'}, b: {short: 'x'}}}",
            "`-x` names more than one argument (at `flags.b.short`)",
        ),
        (
            "{flags: {a: {short: 'x'}}, options: {b: {short: 'x'}}}",
            "`-x` names more than one argument (at `options.b.short`)",
        ),
    ]);
    // Short names are case-sensitive.
    assert_eq!(
        parsed("{flags: {a: {short: 'x'}, b: {short: 'X'}}}", &["-x"]),
        value("{a: true, b: false}"),
    );
}

#[test]
fn help_is_built_in() {
    assert_invalid_specs(&[
        (
            "{flags: {help: {}}}",
            "`help` is built in (at `flags.help`)",
        ),
        (
            "{options: {help: {}}}",
            "`help` is built in (at `options.help`)",
        ),
        (
            "{positionals: [{name: 'help'}]}",
            "`help` is built in (at `positionals[0].name`)",
        ),
        (
            "{flags: {a: {short: 'h'}}}",
            "`-h` is built in (at `flags.a.short`)",
        ),
        (
            "{commands: {sub: {flags: {help: {}}}}}",
            "`help` is built in (at `commands.sub.flags.help`)",
        ),
    ]);
}

#[test]
fn version_is_built_in_only_given_a_version() {
    let problem = "is built in when the spec has a `version`";
    assert_invalid_specs(&[
        (
            "{version: '1', flags: {version: {}}}",
            &format!("`version` {problem} (at `flags.version`)"),
        ),
        (
            "{version: '1', options: {a: {short: 'V'}}}",
            &format!("`-V` {problem} (at `options.a.short`)"),
        ),
        (
            "{commands: {sub: {version: '1', flags: {version: {}}}}}",
            &format!("`version` {problem} (at `commands.sub.flags.version`)"),
        ),
    ]);
    assert_eq!(
        parsed("{flags: {version: {short: 'V'}}}", &["-V"]),
        value("{version: true}"),
    );
    // A command's version is its own: its subcommands may use the names.
    assert_eq!(
        parsed(
            "{version: '1', commands: {sub: {flags: {version: {short: 'V'}}}}}",
            &["sub", "--version"],
        ),
        value("{command: {sub: {version: true}}}"),
    );
}

#[test]
fn command_is_reserved_only_given_commands() {
    let problem = "`command` holds the chosen command when the spec has `commands`";
    assert_invalid_specs(&[
        (
            "{flags: {command: {}}, commands: {a: {}}}",
            &format!("{problem} (at `flags.command`)"),
        ),
        (
            "{options: {command: {}}, commands: {a: {}}}",
            &format!("{problem} (at `options.command`)"),
        ),
    ]);
    assert_eq!(
        parsed("{positionals: [{name: 'command'}]}", &["go"]),
        value("{command: 'go'}"),
    );
    // A subcommand may itself be named `command`, and have one.
    assert_eq!(
        parsed(
            "{commands: {command: {commands: {command: {}}}}}",
            &["command", "command"],
        ),
        value("{command: {command: {command: {command: {}}}}}"),
    );
}

#[test]
fn required_and_default_are_exclusive() {
    assert_invalid_specs(&[
        (
            "{options: {n: {required: true, default: '1'}}}",
            "an option takes `default` or `required: true`, not both (at `options.n`)",
        ),
        (
            "{positionals: [{name: 'n', required: true, default: '1'}]}",
            "a positional argument takes `default` or `required: true`, not both \
             (at `positionals[0]`)",
        ),
    ]);
    // `required: false` says only what `default` implies.
    assert_eq!(
        parsed(
            "{options: {n: {required: false, default: '1'}}, \
              positionals: [{name: 'p', required: false, default: '2'}]}",
            &[],
        ),
        value("{n: '1', p: '2'}"),
    );
}

#[test]
fn a_repeated_argument_takes_no_default() {
    assert_invalid_specs(&[
        (
            "{options: {n: {repeatable: true, default: '1'}}}",
            "an option takes `default` or `repeatable: true`, not both (at `options.n`)",
        ),
        (
            "{positionals: [{name: 'n', many: true, default: '1'}]}",
            "a positional argument takes `default` or `many: true`, not both \
             (at `positionals[0]`)",
        ),
    ]);
    // Only `true` conflicts: `false` says only what is so by default.
    assert_eq!(
        parsed(
            "{options: {n: {repeatable: false, default: '1'}}, \
              positionals: [{name: 'p', many: false, default: '2'}]}",
            &[],
        ),
        value("{n: '1', p: '2'}"),
    );
}

#[test]
fn choices_are_nonempty_distinct_and_hold_the_default() {
    assert_invalid_specs(&[
        (
            "{options: {n: {choices: []}}}",
            "`choices` is empty (at `options.n.choices`)",
        ),
        (
            "{positionals: [{name: 'n', choices: []}]}",
            "`choices` is empty (at `positionals[0].choices`)",
        ),
        (
            "{options: {n: {choices: ['a', 'b', 'a']}}}",
            "`choices` holds \"a\" more than once (at `options.n.choices`)",
        ),
        (
            "{options: {n: {choices: ['a'], default: 'b'}}}",
            "`default` \"b\" is not among `choices` (at `options.n.choices`)",
        ),
        (
            "{positionals: [{name: 'n', choices: ['a'], default: 'b'}]}",
            "`default` \"b\" is not among `choices` (at `positionals[0].choices`)",
        ),
    ]);
}

#[test]
fn only_the_last_positional_may_be_many() {
    assert_invalid_specs(&[(
        "{positionals: [{name: 'a', many: true}, {name: 'b'}]}",
        "only the last positional argument may be `many` (at `positionals[0]`)",
    )]);
}

#[test]
fn a_required_positional_cannot_follow_an_optional_one() {
    assert_invalid_specs(&[
        (
            "{positionals: [{name: 'a', required: false}, {name: 'b'}]}",
            "a required positional argument cannot follow an optional one (at `positionals[1]`)",
        ),
        (
            "{positionals: [{name: 'a', default: 'x'}, {name: 'b'}]}",
            "a required positional argument cannot follow an optional one (at `positionals[1]`)",
        ),
        (
            "{positionals: [{name: 'a', required: false}, {name: 'b', many: true}]}",
            "a required positional argument cannot follow an optional one (at `positionals[1]`)",
        ),
    ]);
    // Optional positionals may follow one another, in any of their forms.
    assert_eq!(
        parsed(
            "{positionals: [{name: 'a'}, {name: 'b', default: 'x'}, \
              {name: 'c', required: false}, {name: 'd', many: true, required: false}]}",
            &["1"],
        ),
        value("{a: '1', b: 'x', c: null, d: []}"),
    );
}

#[test]
fn a_command_has_positionals_or_commands_not_both() {
    assert_invalid_specs(&[(
        "{positionals: [{name: 'a'}], commands: {sub: {}}}",
        "a command with `commands` takes no `positionals` (at `positionals`)",
    )]);
    // An empty `commands` is none at all.
    assert_eq!(
        parsed("{positionals: [{name: 'a'}], commands: {}}", &["x"]),
        value("{a: 'x'}"),
    );
}

#[test]
fn commands_nest_at_most_thirty_two_deep() {
    // The place of the thirty-third command along the path.
    let too_deep = format!(
        "commands nest at most 32 deep (at `{}`)",
        vec!["commands.c"; 33].join(".")
    );
    assert_invalid_specs(&[
        (
            "reduce(range(33), {}, fn inner, _ -> {commands: {c: inner}})",
            &too_deep,
        ),
        // Whatever the thirty-third command holds.
        (
            "reduce(range(32), {commands: {c: 1}}, fn inner, _ -> {commands: {c: inner}})",
            &too_deep,
        ),
        // However much deeper the spec goes: it is never walked recursively,
        // so it cannot overflow the stack.
        (
            "reduce(range(100000), {}, fn inner, _ -> {commands: {c: inner}})",
            &too_deep,
        ),
    ]);
    // Thirty-two deep is allowed, siblings at every level included.
    assert_eq!(
        value(
            "argparse.parse(['prog', 'c', 'a'], \
             reduce(range(32), {}, fn inner, _ -> {commands: {a: {}, c: inner}})).value"
        ),
        value("{command: {c: {command: {a: {}}}}}"),
    );
}

#[test]
fn a_subcommand_is_named_by_its_key() {
    assert_invalid_specs(&[(
        "{commands: {sub: {name: 'other'}}}",
        "a command in `commands` is named by its key, so takes no `name` \
         (at `commands.sub.name`)",
    )]);
}

#[test]
fn a_subcommands_problems_are_placed_within_it() {
    assert_invalid_specs(&[
        (
            "{commands: {a: {commands: {b: {options: {n: {choices: []}}}}}}}",
            "`choices` is empty (at `commands.a.commands.b.options.n.choices`)",
        ),
        (
            "{commands: {['my-cmd']: {flags: {x: {short: 'xy'}}}}}",
            "`short` is one ASCII letter (at `commands[\"my-cmd\"].flags.x.short`)",
        ),
    ]);
}

#[test]
fn a_place_is_written_as_frost_would_index_it() {
    assert_invalid_specs(&[
        // A keyword cannot follow a `.`, so it is bracketed, as the bridge does.
        (
            "{flags: {['not']: {short: 'xy'}}}",
            "`short` is one ASCII letter (at `flags[\"not\"].short`)",
        ),
        (
            "{commands: {['if']: {flags: {x: {short: 'xy'}}}}}",
            "`short` is one ASCII letter (at `commands[\"if\"].flags.x.short`)",
        ),
        (
            "{commands: {a: {positionals: [{name: 'p', choices: []}]}}}",
            "`choices` is empty (at `commands.a.positionals[0].choices`)",
        ),
    ]);
}
