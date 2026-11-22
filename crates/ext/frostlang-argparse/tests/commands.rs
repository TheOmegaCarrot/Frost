//! Subcommands: choosing one, what each takes, and their help.

use crate::common::{assert_parses, assert_usage_errors, text};

/// A program with a flag of its own and two subcommands, one with its own.
const PKG: &str = r"{
    name: 'pkg',
    flags: {quiet: {short: 'q'}},
    commands: {
        add: {
            description: 'Add a package',
            flags: {dev: {short: 'D'}},
            positionals: [{name: 'package'}],
            commands: {},
        },
        remove: {
            positionals: [{name: 'packages', many: true}],
        },
        list: {},
    },
}";

#[test]
fn the_chosen_command_holds_its_own_value() {
    assert_parses(
        PKG,
        &[
            (
                &["add", "serde"],
                "{quiet: false, command: {add: {dev: false, package: 'serde'}}}",
            ),
            (
                &["-q", "add", "-D", "serde"],
                "{quiet: true, command: {add: {dev: true, package: 'serde'}}}",
            ),
            (
                &["add", "serde", "--dev"],
                "{quiet: false, command: {add: {dev: true, package: 'serde'}}}",
            ),
            (
                &["remove", "a", "b"],
                "{quiet: false, command: {remove: {packages: ['a', 'b']}}}",
            ),
            (&["list"], "{quiet: false, command: {list: {}}}"),
        ],
    );
}

#[test]
fn a_command_must_be_chosen() {
    assert_usage_errors(
        PKG,
        &[
            // The error lists the commands.
            (&[], "add, list, remove"),
            (&["-q"], "add, list, remove"),
            (&["nope"], "'nope'"),
            // A command is named in full, in its case.
            (&["ad"], "'ad'"),
            (&["ADD"], "'ADD'"),
        ],
    );
}

#[test]
fn a_commands_arguments_are_its_own() {
    assert_usage_errors(
        PKG,
        &[
            // The program's flag does not follow its subcommand,
            (&["add", "-q", "serde"], "'-q'"),
            // nor a subcommand's precede it,
            (&["-D", "add", "serde"], "'-D'"),
            // nor one subcommand's go to another.
            (&["remove", "--dev", "a"], "'--dev'"),
        ],
    );
}

#[test]
fn a_command_may_reuse_its_parents_names() {
    let spec = r"{
        flags: {verbose: {short: 'v'}},
        options: {level: {short: 'l'}},
        commands: {
            run: {
                flags: {verbose: {short: 'v'}},
                positionals: [{name: 'level'}],
            },
        },
    }";
    assert_parses(
        spec,
        &[
            (
                &["-v", "run", "high"],
                "{verbose: true, level: null, command: {run: {verbose: false, level: 'high'}}}",
            ),
            (
                &["-l", "low", "run", "-v", "high"],
                "{verbose: false, level: 'low', command: {run: {verbose: true, level: 'high'}}}",
            ),
        ],
    );
}

#[test]
fn a_commands_usage_error_shows_its_usage() {
    assert_usage_errors(PKG, &[(&["add"], "<package>"), (&["add"], "pkg add")]);
}

#[test]
fn commands_nest() {
    let spec = r"{
        commands: {
            remote: {
                flags: {verbose: {short: 'v'}},
                commands: {
                    add: {positionals: [{name: 'url'}]},
                    show: {},
                },
            },
        },
    }";
    assert_parses(
        spec,
        &[
            (
                &["remote", "show"],
                "{command: {remote: {verbose: false, command: {show: {}}}}}",
            ),
            (
                &["remote", "-v", "add", "u"],
                "{command: {remote: {verbose: true, command: {add: {url: 'u'}}}}}",
            ),
        ],
    );
    assert_usage_errors(
        spec,
        &[(&["remote"], "prog remote"), (&["remote"], "add, show")],
    );
}

#[test]
fn the_programs_help_lists_its_commands() {
    let help = text("help", PKG, &["--help"]);
    assert!(help.contains("Usage: pkg [OPTIONS] <COMMAND>"), "{help}");
    assert!(help.contains("Add a package"), "{help}");
    assert!(help.contains("remove"), "{help}");
    // `help` is not a command: a command's help is its `--help`.
    assert!(!help.contains("Print this message"), "{help}");
}

#[test]
fn a_command_has_help_of_its_own() {
    let help = text("help", PKG, &["add", "--help"]);
    assert!(help.starts_with("Add a package\n"), "{help}");
    assert!(
        help.contains("Usage: pkg add [OPTIONS] <package>"),
        "{help}"
    );
    assert!(help.contains("-D, --dev"), "{help}");
    assert!(!help.contains("--quiet"), "only its own arguments:\n{help}");
    assert_eq!(text("help", PKG, &["add", "-h"]), help);
    assert_usage_errors(PKG, &[(&["help"], "'help'")]);
}

#[test]
fn a_command_has_a_version_of_its_own() {
    let spec = "{version: '1', commands: {sub: {version: '2'}, other: {}}}";
    assert_eq!(text("version", spec, &["--version"]), "prog 1");
    // A subcommand's version names it git-style, as `man git-add` would.
    assert_eq!(text("version", spec, &["sub", "--version"]), "prog-sub 2");
    assert_usage_errors(spec, &[(&["other", "--version"], "'--version'")]);
}
