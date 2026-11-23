//! `hidden`, which keeps an argument or subcommand out of the help while it
//! works all the same, and `epilog`, text for the end of a command's help.

use crate::common::{assert_invalid_specs, assert_parses, text};

/// A spec with one hidden argument of each kind, beside visible ones.
const HIDDEN: &str = r"{
    flags: {
        debug: {hidden: true},
        verbose: {short: 'v', hidden: false},
    },
    options: {
        secret: {short: 's', hidden: true},
        level: {},
    },
    positionals: [
        {name: 'target'},
        {name: 'extra', required: false, hidden: true},
    ],
}";

#[test]
fn a_hidden_argument_is_left_out_of_the_help() {
    let help = text("help", HIDDEN, &["--help"]);
    for hidden in ["debug", "secret", "-s,", "extra"] {
        assert!(!help.contains(hidden), "{hidden:?} is hidden:\n{help}");
    }
    for shown in ["--verbose", "--level", "<target>"] {
        assert!(help.contains(shown), "{shown:?} is shown:\n{help}");
    }
}

#[test]
fn a_hidden_argument_works_all_the_same() {
    assert_parses(
        HIDDEN,
        &[
            (
                &["--debug", "-s", "x", "t", "e"],
                "{debug: true, verbose: false, secret: 'x', level: null, target: 't', extra: 'e'}",
            ),
            // And is in the value when not given, as every argument is.
            (
                &["t"],
                "{debug: false, verbose: false, secret: null, level: null, target: 't', extra: null}",
            ),
        ],
    );
}

/// A program with a visible and a hidden subcommand.
const COMMANDS: &str = r"{
    name: 'pkg',
    commands: {
        add: {},
        internal: {hidden: true, flags: {force: {}}},
    },
}";

#[test]
fn a_hidden_subcommand_is_left_out_of_its_parent_s_help() {
    let help = text("help", COMMANDS, &["--help"]);
    assert!(help.contains("add"), "{help}");
    assert!(!help.contains("internal"), "{help}");
}

#[test]
fn a_hidden_subcommand_works_all_the_same() {
    assert_parses(
        COMMANDS,
        &[(
            &["internal", "--force"],
            "{command: {internal: {force: true}}}",
        )],
    );
    // Its own help shows it in full.
    let help = text("help", COMMANDS, &["internal", "--help"]);
    assert!(help.contains("Usage: pkg internal"), "{help}");
    assert!(help.contains("--force"), "{help}");
}

#[test]
fn the_program_itself_cannot_be_hidden() {
    assert_invalid_specs(&[(
        "{hidden: true}",
        "the program itself cannot be `hidden`; a subcommand hides from its parent's help \
         (at `hidden`)",
    )]);
    // `hidden: false` asks nothing of the program.
    assert_parses("{hidden: false}", &[(&[], "{}")]);
}

#[test]
fn an_epilog_ends_the_help() {
    let spec = r"{
        name: 'tool',
        epilog: 'Examples:\n  tool --fast',
        flags: {fast: {description: 'Go fast'}},
    }";
    for asking in ["-h", "--help"] {
        let help = text("help", spec, &[asking]);
        assert!(
            help.ends_with("\n\nExamples:\n  tool --fast"),
            "{asking}:\n{help}"
        );
        let options = help.find("--fast").expect("the option is listed");
        let epilog = help.find("Examples:").expect("the epilog is shown");
        assert!(options < epilog, "after the options, for {asking}:\n{help}");
    }
}

#[test]
fn each_command_has_its_own_epilog() {
    let spec = r"{
        epilog: 'For the program.',
        commands: {
            add: {epilog: 'For add.'},
            remove: {},
        },
    }";
    let program = text("help", spec, &["--help"]);
    assert!(program.ends_with("For the program."), "{program}");
    let add = text("help", spec, &["add", "--help"]);
    assert!(add.ends_with("For add."), "{add}");
    assert!(
        !add.contains("For the program."),
        "not its parent's:\n{add}"
    );
    let remove = text("help", spec, &["remove", "--help"]);
    assert!(!remove.contains("For the program."), "{remove}");
}

#[test]
fn hidden_is_a_bool_and_epilog_a_string() {
    assert_invalid_specs(&[
        (
            "{flags: {v: {hidden: 'yes'}}}",
            "expected Bool, got String (at `flags.v.hidden`)",
        ),
        (
            "{options: {n: {hidden: 1}}}",
            "expected Bool, got Int (at `options.n.hidden`)",
        ),
        (
            "{positionals: [{name: 'p', hidden: null}]}",
            "expected Bool, got Null (at `positionals[0].hidden`)",
        ),
        (
            "{commands: {sub: {hidden: 'no'}}}",
            "expected Bool, got String (at `commands.sub.hidden`)",
        ),
        ("{epilog: 1}", "expected String, got Int (at `epilog`)"),
        (
            "{commands: {sub: {epilog: ['x']}}}",
            "expected String, got Array (at `commands.sub.epilog`)",
        ),
    ]);
}
