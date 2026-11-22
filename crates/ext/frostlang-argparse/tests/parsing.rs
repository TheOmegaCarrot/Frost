//! How a command line parses: the `value` Map a valid one gives, and the
//! usage errors an invalid one gives.
//!
//! clap words the usage errors, so these tests check only that each names
//! what is wrong; `outcomes.rs` pins the layout of one.

use crate::common::{assert_parses, assert_usage_errors, text};

/// A spec with flags and options of each kind, and no positionals.
const NAMED: &str = r"{
    flags: {
        verbose: {short: 'v'},
        all: {short: 'a'},
        dry_run: {},
    },
    options: {
        name: {short: 'n'},
        level: {default: 'low'},
        tag: {short: 't', repeatable: true},
    },
}";

/// [`NAMED`]'s value when the command line gives nothing.
const NAMED_UNGIVEN: &str = r"{
    verbose: false,
    all: false,
    dry_run: false,
    name: null,
    level: 'low',
    tag: [],
}";

/// `NAMED_UNGIVEN`, with `changes` merged over it.
fn named(changes: &str) -> String {
    format!("{NAMED_UNGIVEN} + {changes}")
}

#[test]
fn every_argument_the_spec_gives_is_in_the_value() {
    assert_parses(NAMED, &[(&[], NAMED_UNGIVEN)]);
}

#[test]
fn a_flag_is_true_given_by_name_or_short_name() {
    assert_parses(
        NAMED,
        &[
            (&["--verbose"], &named("{verbose: true}")),
            (&["-v"], &named("{verbose: true}")),
            (&["-v", "--all"], &named("{verbose: true, all: true}")),
        ],
    );
}

#[test]
fn a_flag_or_option_is_given_with_a_dash_for_each_underscore() {
    assert_parses(
        "{flags: {dry_run: {}}, options: {max_line_count: {}}}",
        &[(
            &["--dry-run", "--max-line-count", "3"],
            "{dry_run: true, max_line_count: '3'}",
        )],
    );
    assert_usage_errors(
        NAMED,
        &[(&["--dry_run"], "--dry_run"), (&["--dryrun"], "--dryrun")],
    );
    let help = text("help", NAMED, &["--help"]);
    assert!(help.contains("--dry-run"), "{help}");
    assert!(!help.contains("dry_run"), "{help}");
}

#[test]
fn short_flags_combine() {
    assert_parses(
        NAMED,
        &[
            (&["-va"], &named("{verbose: true, all: true}")),
            (&["-av"], &named("{verbose: true, all: true}")),
        ],
    );
}

#[test]
fn an_options_value_follows_it_in_each_form() {
    for arguments in [
        ["--name", "x"].as_slice(),
        &["--name=x"],
        &["-n", "x"],
        &["-nx"],
        &["-n=x"],
    ] {
        assert_parses(NAMED, &[(arguments, &named("{name: 'x'}"))]);
    }
}

#[test]
fn the_last_of_combined_short_flags_may_be_an_option() {
    assert_parses(
        NAMED,
        &[
            (
                &["-van", "x"],
                &named("{verbose: true, all: true, name: 'x'}"),
            ),
            (&["-vanx"], &named("{verbose: true, all: true, name: 'x'}")),
        ],
    );
}

#[test]
fn an_options_value_is_kept_exactly() {
    for value in ["", " spaced  out ", "caf\u{e9}", "a=b", "--", "-"] {
        let expected = named(&format!(
            "{{name: {}}}",
            frostlang::Value::from(value).to_debug_string()
        ));
        assert_parses(NAMED, &[(&[&format!("--name={value}")], &expected)]);
    }
    // A value that is not an option's own may follow it as a separate item.
    assert_parses(
        NAMED,
        &[(&["--name", " spaced "], &named("{name: ' spaced '}"))],
    );
}

#[test]
fn an_option_given_once_takes_its_default_otherwise() {
    assert_parses(
        NAMED,
        &[
            (&["--level", "high"], &named("{level: 'high'}")),
            (&["--level="], &named("{level: ''}")),
        ],
    );
}

#[test]
fn a_repeatable_option_gathers_its_values_in_order() {
    assert_parses(
        NAMED,
        &[
            (&["-t", "a"], &named("{tag: ['a']}")),
            (
                &["-t", "b", "--tag=a", "-tc"],
                &named("{tag: ['b', 'a', 'c']}"),
            ),
            (&["-t", "a", "-t", "a"], &named("{tag: ['a', 'a']}")),
        ],
    );
}

#[test]
fn a_value_may_be_a_negative_number() {
    assert_parses(
        NAMED,
        &[
            (&["--name", "-5"], &named("{name: '-5'}")),
            (&["-n", "-1.5"], &named("{name: '-1.5'}")),
            (&["-t", "-1", "-t", "-2"], &named("{tag: ['-1', '-2']}")),
        ],
    );
    assert_parses(
        "{positionals: [{name: 'a'}, {name: 'b', many: true}]}",
        &[(&["-1", "-2", "-3"], "{a: '-1', b: ['-2', '-3']}")],
    );
}

#[test]
fn an_option_like_value_needs_an_equals_sign() {
    assert_parses(NAMED, &[(&["--name=-v"], &named("{name: '-v'}"))]);
    assert_usage_errors(NAMED, &[(&["--name", "--verbose"], "--name")]);
}

#[test]
fn an_unknown_argument_is_a_usage_error() {
    assert_usage_errors(
        NAMED,
        &[
            (&["--nope"], "'--nope'"),
            (&["-x"], "'-x'"),
            (&["-vx"], "'-x'"),
            (&["stray"], "'stray'"),
            // A long name is never abbreviated.
            (&["--verb"], "'--verb'"),
        ],
    );
}

#[test]
fn a_usage_error_suggests_a_near_name() {
    assert_usage_errors(NAMED, &[(&["--verbos"], "'--verbose'")]);
}

#[test]
fn a_flag_or_single_option_given_twice_is_a_usage_error() {
    assert_usage_errors(
        NAMED,
        &[
            (&["-v", "--verbose"], "--verbose"),
            (&["-vv"], "--verbose"),
            (&["-n", "a", "--name", "b"], "--name"),
            (&["--level", "a", "--level", "b"], "--level"),
        ],
    );
}

#[test]
fn a_flag_takes_no_value() {
    assert_usage_errors(NAMED, &[(&["--verbose=yes"], "'yes'")]);
}

#[test]
fn an_option_without_its_value_is_a_usage_error() {
    assert_usage_errors(
        NAMED,
        &[
            (&["--name"], "--name"),
            (&["-n"], "--name"),
            (&["-vn"], "--name"),
        ],
    );
}

#[test]
fn a_required_option_must_be_given() {
    let spec = "{options: {env: {required: true}, tags: {repeatable: true, required: true}}}";
    assert_parses(
        spec,
        &[(&["--env", "a", "--tags", "b"], "{env: 'a', tags: ['b']}")],
    );
    assert_usage_errors(
        spec,
        &[(&["--tags", "b"], "--env"), (&["--env", "a"], "--tags")],
    );
}

#[test]
fn an_option_with_choices_takes_only_those() {
    let spec = r"{
        options: {
            env: {choices: ['dev', 'prod'], default: 'dev'},
            arch: {choices: ['x86', 'arm'], repeatable: true},
        },
    }";
    assert_parses(
        spec,
        &[
            (&[], "{env: 'dev', arch: []}"),
            (&["--env", "prod"], "{env: 'prod', arch: []}"),
            (
                &["--arch", "arm", "--arch", "x86"],
                "{env: 'dev', arch: ['arm', 'x86']}",
            ),
        ],
    );
    assert_usage_errors(
        spec,
        &[
            (&["--env", "qa"], "'qa'"),
            // The error lists the choices.
            (&["--env", "qa"], "dev, prod"),
            // Choices are exact: no case folding, no prefixes.
            (&["--env", "Dev"], "'Dev'"),
            (&["--env", "pro"], "'pro'"),
            (&["--arch", "arm", "--arch", "mips"], "'mips'"),
        ],
    );
}

/// A spec with positionals of each kind.
const POSITIONALS: &str = r"{
    flags: {verbose: {short: 'v'}},
    positionals: [
        {name: 'source'},
        {name: 'mode', default: 'copy'},
        {name: 'target', required: false},
        {name: 'rest', many: true, required: false},
    ],
}";

#[test]
fn positionals_take_values_in_order() {
    assert_parses(
        POSITIONALS,
        &[
            (
                &["a"],
                "{verbose: false, source: 'a', mode: 'copy', target: null, rest: []}",
            ),
            (
                &["a", "move"],
                "{verbose: false, source: 'a', mode: 'move', target: null, rest: []}",
            ),
            (
                &["a", "move", "b"],
                "{verbose: false, source: 'a', mode: 'move', target: 'b', rest: []}",
            ),
            (
                &["a", "move", "b", "c", "d"],
                "{verbose: false, source: 'a', mode: 'move', target: 'b', rest: ['c', 'd']}",
            ),
        ],
    );
}

#[test]
fn flags_and_options_may_come_among_positionals() {
    assert_parses(
        POSITIONALS,
        &[
            (
                &["a", "-v", "move"],
                "{verbose: true, source: 'a', mode: 'move', target: null, rest: []}",
            ),
            (
                &["a", "b", "c", "d", "--verbose", "e"],
                "{verbose: true, source: 'a', mode: 'b', target: 'c', rest: ['d', 'e']}",
            ),
        ],
    );
}

#[test]
fn after_a_double_dash_every_item_is_a_positional() {
    assert_parses(
        POSITIONALS,
        &[
            (
                &["--", "-v"],
                "{verbose: false, source: '-v', mode: 'copy', target: null, rest: []}",
            ),
            (
                &["-v", "--", "--", "-x", "--verbose", "-"],
                "{verbose: true, source: '--', mode: '-x', target: '--verbose', rest: ['-']}",
            ),
        ],
    );
}

#[test]
fn a_lone_dash_is_a_positional() {
    assert_parses(
        POSITIONALS,
        &[(
            &["-", "-"],
            "{verbose: false, source: '-', mode: '-', target: null, rest: []}",
        )],
    );
}

#[test]
fn a_required_positional_must_be_given() {
    assert_usage_errors(POSITIONALS, &[(&[], "<source>")]);
    let spec = "{positionals: [{name: 'files', many: true}]}";
    assert_parses(
        spec,
        &[
            (&["a"], "{files: ['a']}"),
            (&["a", "b"], "{files: ['a', 'b']}"),
        ],
    );
    assert_usage_errors(spec, &[(&[], "<files>")]);
}

#[test]
fn a_positional_beyond_the_last_is_a_usage_error() {
    assert_usage_errors("{positionals: [{name: 'a'}]}", &[(&["x", "y"], "'y'")]);
}

#[test]
fn a_positional_with_choices_takes_only_those() {
    let spec = "{positionals: [{name: 'mode', choices: ['on', 'off']}, \
                {name: 'rest', many: true, required: false, choices: ['x']}]}";
    assert_parses(
        spec,
        &[
            (&["on"], "{mode: 'on', rest: []}"),
            (&["off", "x", "x"], "{mode: 'off', rest: ['x', 'x']}"),
        ],
    );
    assert_usage_errors(spec, &[(&["maybe"], "'maybe'"), (&["on", "x", "y"], "'y'")]);
}
