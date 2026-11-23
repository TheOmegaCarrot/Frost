//! `parse`: a Function an option or positional argument passes the text of
//! each of its values to, its default included, taking what it returns.

use crate::common::{
    assert_invalid_specs, assert_parses, assert_usage_errors, parsed, raises, text, usage_error,
    value,
};

/// A spec whose arguments parse to Ints, one of each kind.
const INTS: &str = r"{
    options: {
        count: {short: 'c', default: '3', parse: to_int},
        size: {parse: to_int},
        tag: {short: 't', repeatable: true, parse: to_int},
    },
    positionals: [
        {name: 'first', required: false, parse: to_int},
        {name: 'rest', many: true, required: false, parse: to_int},
    ],
}";

#[test]
fn each_value_is_what_parse_makes_of_its_text() {
    assert_parses(
        INTS,
        &[
            (
                &[
                    "-c", "7", "--size", "-2", "-t", "1", "-t", "2", "10", "20", "30",
                ],
                "{count: 7, size: -2, tag: [1, 2], first: 10, rest: [20, 30]}",
            ),
            // An argument not given, without a default, is Null: no text to parse.
            (
                &[],
                "{count: 3, size: null, tag: [], first: null, rest: []}",
            ),
        ],
    );
}

#[test]
fn parse_may_make_any_value() {
    let spec = r"{
        options: {
            point: {parse: fn text -> {at: text, length: len(text)}},
            words: {parse: fn text -> split(text, ',')},
            confirm: {parse: fn text -> text == 'yes'},
        },
    }";
    assert_parses(
        spec,
        &[
            (
                &["--point", "abc", "--words", "a,b"],
                "{point: {at: 'abc', length: 3}, words: ['a', 'b'], confirm: null}",
            ),
            // Only Null is a rejection: `false` is a value like any other.
            (
                &["--confirm", "no"],
                "{point: null, words: null, confirm: false}",
            ),
        ],
    );
}

#[test]
fn a_default_is_text_parsed_as_if_given() {
    // Leaving the argument out is the same as giving its default.
    let spec = r"{
        options: {count: {default: '3', parse: fn text -> {parsed: to_int(text)}}},
        positionals: [{name: 'p', default: '5', parse: to_int}],
    }";
    assert_eq!(parsed(spec, &[]), parsed(spec, &["--count", "3", "5"]));
    assert_eq!(parsed(spec, &[]), value("{count: {parsed: 3}, p: 5}"));
    // So help shows it as the text it is.
    let help = text("help", spec, &["--help"]);
    assert!(help.contains("[default: 3]"), "{help}");
    assert!(help.contains("[default: 5]"), "{help}");
}

#[test]
fn null_for_given_text_is_a_usage_error() {
    assert_usage_errors(
        INTS,
        &[
            (
                &["-c", "many"],
                "invalid value 'many' for '--count <count>'",
            ),
            (&["--size", ""], "invalid value '' for '--size <size>'"),
            (
                &["-t", "1", "-t", "x"],
                "invalid value 'x' for '--tag <tag>'",
            ),
            (&["x"], "invalid value 'x' for '[first]'"),
            (&["1", "2", "y"], "invalid value 'y' for '[rest]...'"),
        ],
    );
    // Laid out as clap lays out a value outside `choices`, but for the line
    // listing them.
    let rejected = usage_error("{options: {n: {parse: fn _ -> null}}}", &["--n", "q"]);
    let outside_choices = usage_error("{options: {n: {choices: ['a']}}}", &["--n", "q"]);
    let without_choices: Vec<&str> = outside_choices
        .lines()
        .filter(|line| !line.trim_start().starts_with("[possible values"))
        .collect();
    assert_eq!(rejected, without_choices.join("\n"));
}

/// Assert that parsing each command line, after a program name, as the spec
/// paired with it raises exactly the message paired with it.
fn assert_parse_raises(cases: &[(&str, &[&str], &str)]) {
    for (spec, arguments, message) in cases {
        let args = std::iter::once("prog")
            .chain(arguments.iter().copied())
            .map(|argument| frostlang::Value::from(argument).to_debug_string())
            .collect::<Vec<_>>()
            .join(", ");
        let expression = format!("argparse.parse([{args}], {spec})");
        assert_eq!(raises(&expression), *message, "{expression}");
    }
}

#[test]
fn null_for_the_default_is_a_spec_error() {
    let nested = r"{
        commands: {
            sub: {positionals: [{name: 'p', default: 'x', parse: to_int}]},
        },
    }";
    assert_parse_raises(&[
        (
            "{options: {n: {default: 'x', parse: to_int}}}",
            &[],
            "Function ext.argparse.parse requires argument 2 (spec) to be valid: \
             `parse` gives Null for `default` \"x\" (at `options.n.default`)",
        ),
        (
            nested,
            &["sub"],
            "Function ext.argparse.parse requires argument 2 (spec) to be valid: \
             `parse` gives Null for `default` \"x\" (at `commands.sub.positionals[0].default`)",
        ),
    ]);
}

#[test]
fn a_default_is_parsed_only_when_taken() {
    // The default `parse` rejects goes unnoticed while the argument is given.
    let spec = "{options: {n: {default: 'x', parse: to_int}}}";
    assert_eq!(parsed(spec, &["--n", "4"]), value("{n: 4}"), "{spec}");
    // So does a `parse` that raises for it.
    let spec = r"{
        options: {
            n: {default: 'x', parse: fn text -> if text == 'x': error('default parsed') else: text},
        },
    }";
    assert_eq!(parsed(spec, &["--n", "4"]), value("{n: '4'}"), "{spec}");
}

#[test]
fn an_error_parse_raises_is_raised() {
    let raising = "fn _ -> error('bad parse')";
    assert_parse_raises(&[
        (
            &format!("{{options: {{n: {{parse: {raising}}}}}}}"),
            &["--n", "1"],
            "bad parse",
        ),
        // Even when raised for the default.
        (
            &format!("{{options: {{n: {{default: '1', parse: {raising}}}}}}}"),
            &[],
            "bad parse",
        ),
    ]);
}

#[test]
fn clap_s_own_errors_come_first() {
    assert_usage_errors(
        INTS,
        &[
            // An unknown argument after a value `parse` would reject.
            (&["-c", "many", "--nope"], "'--nope'"),
            // A missing value before one.
            (&["--size", "x", "--tag"], "'--tag <tag>'"),
        ],
    );
}

#[test]
fn values_are_parsed_in_the_order_the_spec_gives_their_arguments() {
    // Options in name order, then positional arguments: the first that
    // `parse` rejects is the one reported.
    assert_usage_errors(
        INTS,
        &[
            (&["--size", "x", "-c", "y"], "'y' for '--count <count>'"),
            (&["z", "--size", "x"], "'x' for '--size <size>'"),
        ],
    );
}

#[test]
fn choices_are_checked_against_the_text_before_parse() {
    let spec = r"{
        options: {level: {choices: ['low', 'high'], default: 'low', parse: fn text -> {level: text}}},
    }";
    assert_parses(
        spec,
        &[
            (&[], "{level: {level: 'low'}}"),
            (&["--level", "high"], "{level: {level: 'high'}}"),
        ],
    );
    assert_usage_errors(
        spec,
        &[
            (&["--level", "mid"], "'mid'"),
            (&["--level", "mid"], "low, high"),
        ],
    );
    // The help still lists them.
    let help = text("help", spec, &["--help"]);
    assert!(help.contains("[possible values: low, high]"), "{help}");
}

#[test]
fn parse_runs_only_for_values_a_parse_uses() {
    // A `parse` that raises shows whenever it runs.
    let spec = "{version: '1', options: {n: {default: '2', parse: fn _ -> error('ran')}}}";
    // Not for help or the version, even with a value given and a default.
    assert!(text("help", spec, &["--n", "1", "--help"]).starts_with("Usage:"));
    assert_eq!(text("version", spec, &["--n", "1", "-V"]), "prog 1");
    // Not after a usage error of clap's.
    assert_usage_errors(spec, &[(&["--n", "1", "--nope"], "'--nope'")]);
    // Nor for an argument not given, without a default.
    let spec = "{options: {n: {parse: fn _ -> error('ran')}}}";
    assert_eq!(parsed(spec, &[]), value("{n: null}"));
}

#[test]
fn a_subcommand_s_arguments_parse_too() {
    let spec = r"{
        options: {depth: {parse: to_int}},
        commands: {run: {positionals: [{name: 'times', parse: to_int}]}},
    }";
    assert_parses(
        spec,
        &[(
            &["--depth", "2", "run", "5"],
            "{depth: 2, command: {run: {times: 5}}}",
        )],
    );
    let error = usage_error(spec, &["run", "five"]);
    assert!(
        error.starts_with("invalid value 'five' for '<times>'"),
        "{error}"
    );
}

#[test]
fn parse_is_a_function() {
    assert_invalid_specs(&[
        (
            "{options: {n: {parse: 1}}}",
            "expected Function, got Int (at `options.n.parse`)",
        ),
        (
            "{options: {n: {parse: 'to_int'}}}",
            "expected Function, got String (at `options.n.parse`)",
        ),
        (
            "{positionals: [{name: 'p', parse: null}]}",
            "expected Function, got Null (at `positionals[0].parse`)",
        ),
        (
            "{flags: {f: {parse: to_int}}}",
            "unknown field `parse`, expected one of `short`, `description`, `hidden` \
             (at a key of `flags.f`)",
        ),
    ]);
}

#[test]
fn parse_is_called_with_one_argument_the_text() {
    for (parse, expects) in [("fn -> 1", "0 arguments"), ("fn a, b -> 1", "2 arguments")] {
        assert_eq!(
            raises(&format!(
                "argparse.parse(['prog', '--n', '1'], {{options: {{n: {{parse: {parse}}}}}}})"
            )),
            format!("Function <lambda> expects {expects}, but was called with 1"),
            "{parse}"
        );
    }
    assert_parses(
        "{options: {n: {parse: fn ...texts -> texts}}}",
        &[(&["--n", "x"], "{n: ['x']}")],
    );
}
