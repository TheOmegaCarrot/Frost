//! The outcomes besides a value: help, the version, and usage errors, and how
//! their text reads.

use crate::common::{parse, text, usage_error, value};

/// A spec using every part of the help.
const DEPLOY: &str = r"{
    name: 'deploy',
    description: 'Deploy a service',
    version: '1.2.0',
    flags: {
        verbose: {short: 'v', description: 'Say more'},
    },
    options: {
        env: {short: 'e', required: true, choices: ['dev', 'prod'], description: 'Where to'},
        retries: {default: '3', description: 'How many tries'},
        tag: {repeatable: true, value_name: 'TAG'},
    },
    positionals: [
        {name: 'service', description: 'What to deploy'},
        {name: 'hosts', many: true, required: false},
    ],
}";

/// Assert `text` has no trailing whitespace on any line, nor a final newline,
/// and no terminal escapes: it is plain text, ready to print.
fn assert_plain(text: &str) {
    for line in text.lines() {
        assert_eq!(line, line.trim_end(), "no trailing whitespace in:\n{text}");
    }
    assert!(!text.ends_with('\n'), "no final newline in:\n{text}");
    assert!(!text.contains('\u{1b}'), "no terminal escapes in:\n{text}");
}

/// Assert `text` holds each of `parts`.
fn assert_holds(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "{part:?} should be in:\n{text}");
    }
}

#[test]
fn help_is_asked_for_by_long_or_short_name() {
    let long = text("help", DEPLOY, &["--help"]);
    assert_eq!(text("help", DEPLOY, &["-h"]), long);
}

#[test]
fn help_describes_every_argument() {
    let help = text("help", DEPLOY, &["--help"]);
    assert_plain(&help);
    assert!(help.starts_with("Deploy a service\n"), "{help}");
    assert_holds(
        &help,
        &[
            "Usage: deploy [OPTIONS] --env <env> <service> [hosts]...",
            "<service>",
            "What to deploy",
            "[hosts]...",
            "-v, --verbose",
            "Say more",
            "-e, --env <env>",
            "Where to [possible values: dev, prod]",
            "--retries <retries>",
            "How many tries [default: 3]",
            "--tag <TAG>",
            "-h, --help",
            "-V, --version",
        ],
    );
}

#[test]
fn help_lists_flags_and_options_by_name() {
    let help = text(
        "help",
        "{flags: {b: {}, d: {}}, options: {a: {}, c: {}}}",
        &["--help"],
    );
    let at = |name: &str| {
        help.find(name)
            .unwrap_or_else(|| panic!("{name} is in:\n{help}"))
    };
    assert!(
        at("--a") < at("--b") && at("--b") < at("--c") && at("--c") < at("--d"),
        "in name order:\n{help}"
    );
}

#[test]
fn help_wins_over_the_rest_of_the_command_line() {
    // Even when what comes before or after is invalid.
    for arguments in [["--help", "--nope"], ["--verbose", "--help"], ["x", "-h"]] {
        assert_eq!(
            text("help", DEPLOY, &arguments),
            text("help", DEPLOY, &["--help"]),
            "{arguments:?}"
        );
    }
}

#[test]
fn an_error_before_help_wins() {
    let error = usage_error(DEPLOY, &["--nope", "--help"]);
    assert!(error.contains("'--nope'"), "{error}");
}

#[test]
fn help_without_a_version_lists_no_version() {
    let help = text("help", "{}", &["--help"]);
    assert!(!help.contains("version"), "{help}");
    assert_eq!(help, "Usage: prog\n\nOptions:\n  -h, --help  Print help");
}

#[test]
fn the_version_is_asked_for_by_long_or_short_name() {
    for arguments in [["--version"], ["-V"]] {
        let version = text("version", DEPLOY, &arguments);
        assert_eq!(version, "deploy 1.2.0", "{arguments:?}");
    }
}

#[test]
fn without_a_version_none_can_be_asked_for() {
    for arguments in [["--version"], ["-V"]] {
        let error = usage_error("{}", &arguments);
        assert!(error.contains(&format!("'{}'", arguments[0])), "{error}");
    }
}

#[test]
fn a_usage_error_says_why() {
    // The one usage error pinned whole: clap words it, and the extension
    // drops clap's leading `error: ` label and trailing whitespace.
    let error = usage_error(DEPLOY, &["--nope"]);
    assert_plain(&error);
    assert_eq!(
        error,
        r"unexpected argument '--nope' found

  tip: to pass '--nope' as a value, use '-- --nope'

Usage: deploy [OPTIONS] --env <env> <service> [hosts]...

For more information, try '--help'."
    );
}

#[test]
fn every_usage_error_drops_clap_s_label() {
    for arguments in [
        ["--nope"].as_slice(),
        &[],
        &["-e", "qa", "s"],
        &["-v", "-v"],
    ] {
        let error = usage_error(DEPLOY, arguments);
        assert_plain(&error);
        assert!(!error.starts_with("error"), "{arguments:?}:\n{error}");
        assert!(error.contains("--help"), "{arguments:?}:\n{error}");
    }
}

/// The first line of the help for `spec`, run as `program`.
fn usage_line(program: &str, spec: &str) -> String {
    let help = value(&format!(
        "argparse.parse(['{program}', '--help'], {spec}).help"
    ));
    let frostlang::Value::String(help) = help else {
        panic!("help is a String, but got {help:?}");
    };
    help.as_str()
        .lines()
        .next()
        .expect("help has a line")
        .to_string()
}

#[test]
fn without_a_name_the_program_goes_by_its_file_name() {
    for (program, name) in [
        ("prog", "prog"),
        ("/usr/local/bin/tool", "tool"),
        ("scripts/deploy.frst", "deploy.frst"),
        ("./x", "x"),
        ("-e", "-e"),
        ("-", "-"),
    ] {
        assert_eq!(
            usage_line(program, "{}"),
            format!("Usage: {name}"),
            "{program}"
        );
    }
}

#[test]
fn a_name_overrides_the_programs_file_name() {
    assert_eq!(usage_line("/bin/x", "{name: 'tool'}"), "Usage: tool");
    assert_eq!(
        value("argparse.parse(['/bin/x', '-V'], {name: 'tool', version: '2'}).version"),
        value("'tool 2'")
    );
}

#[test]
fn an_outcome_is_a_map_of_one_key() {
    for (arguments, key) in [
        (["-e", "dev", "s"].as_slice(), "value"),
        (&["--help"], "help"),
        (&["--version"], "version"),
        (&["--nope"], "error"),
    ] {
        let outcome = parse(DEPLOY, arguments);
        let frostlang::Value::Map(map) = &outcome else {
            panic!("{arguments:?}: a Map, but got {outcome:?}");
        };
        assert_eq!(map.len(), 1, "{arguments:?}: {outcome:?}");
        assert!(
            map.get(&frostlang::MapKey::from(key)).is_some(),
            "{arguments:?}: keyed `{key}`, but got {outcome:?}"
        );
    }
}

#[test]
fn values_are_always_strings() {
    let parsed = value(
        "argparse.parse(['prog', '-e', 'dev', '--retries', '7', '42'], \
         {options: {env: {short: 'e'}, retries: {}}, positionals: [{name: 'n'}]}).value",
    );
    assert_eq!(parsed, value("{env: 'dev', retries: '7', n: '42'}"));
}
