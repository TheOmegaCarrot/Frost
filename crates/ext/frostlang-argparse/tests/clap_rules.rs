//! Valid specs at the edge of each rule clap holds a command to. clap checks
//! its commands with debug assertions, which panic, so a valid spec that broke
//! one would fail here: these tests build in debug.
//!
//! Each spec is parsed for its help, its value with nothing given, and its
//! value with everything given, which between them build every command and
//! argument in it. The specs one step past each edge are in `spec.rs`, which
//! tests that they raise.

use crate::common::parse;

/// Assert that each spec, with the command lines paired with it, parses to an
/// outcome rather than raising or panicking.
fn assert_builds(cases: &[(&str, &[&[&str]])]) {
    for (spec, command_lines) in cases {
        for arguments in [["--help"].as_slice(), &[]]
            .iter()
            .chain(command_lines.iter())
        {
            // `parse` panics if the script raises.
            let outcome = parse(spec, arguments);
            assert!(
                matches!(outcome, frostlang::Value::Map(_)),
                "{spec} with {arguments:?} gives an outcome Map, but gave {outcome:?}"
            );
        }
    }
}

#[test]
fn argument_ids_long_names_and_short_names_are_unique() {
    assert_builds(&[
        // Names and shorts that differ only in case, or by an underscore.
        (
            "{flags: {a: {short: 'a'}, A: {short: 'A'}, a_b: {}, ab: {}}}",
            &[&["-aA", "--a-b", "--ab"]],
        ),
        // A positional named as a flag's short.
        (
            "{flags: {x: {short: 'v'}}, positionals: [{name: 'v'}]}",
            &[&["-v", "v"]],
        ),
        // A subcommand reusing every name of its parent, and named as one.
        (
            "{flags: {run: {short: 'r'}}, commands: {run: {flags: {run: {short: 'r'}}}}}",
            &[&["-r", "run", "-r"]],
        ),
    ]);
}

#[test]
fn built_in_names_are_free_where_nothing_builds_them_in() {
    assert_builds(&[
        ("{flags: {version: {short: 'V'}}}", &[&["-V"]]),
        (
            "{version: '1', commands: {sub: {flags: {version: {short: 'V'}}}}}",
            &[&["sub", "-V"], &["-V"]],
        ),
        ("{positionals: [{name: 'command'}]}", &[&["x"]]),
        (
            "{commands: {help: {}, version: {}}}",
            &[&["help"], &["version"]],
        ),
    ]);
}

#[test]
fn positionals_at_the_edges_of_their_order() {
    assert_builds(&[
        // Required, then each kind of optional, in every allowed order.
        (
            "{positionals: [{name: 'a'}, {name: 'b', default: 'x'}, \
              {name: 'c', required: false}, {name: 'd', many: true, required: false}]}",
            &[&["1", "2", "3", "4", "5"]],
        ),
        (
            "{positionals: [{name: 'a', required: false}, {name: 'b', default: 'x'}]}",
            &[&["1", "2"]],
        ),
        (
            "{positionals: [{name: 'a'}, {name: 'b'}, {name: 'c', many: true}]}",
            &[&["1", "2", "3"]],
        ),
        (
            "{positionals: [{name: 'a', many: true, required: false}]}",
            &[&["1"]],
        ),
        ("{positionals: [{name: 'a', default: 'x'}]}", &[&["1"]]),
    ]);
}

#[test]
fn values_at_the_edges_of_their_rules() {
    assert_builds(&[
        (
            r"{
                options: {
                    a: {default: ''},
                    b: {choices: [''], default: ''},
                    c: {choices: ['-x', '--', '-1'], repeatable: true, required: true},
                    d: {value_name: ''},
                    e: {repeatable: true, required: false},
                },
            }",
            &[&["--c", "--", "--c=-x", "--d", "v"]],
        ),
        (
            "{positionals: [{name: 'p', choices: ['-1', '-'], many: true}]}",
            &[&["-1", "-"]],
        ),
    ]);
}

#[test]
fn names_and_text_at_the_edges_of_their_rules() {
    assert_builds(&[
        ("{name: '', description: '', version: ''}", &[&["-V"]]),
        ("{name: 'a b\\nc'}", &[]),
        (
            "{flags: {['9']: {}, z_: {}, a__b: {}}}",
            &[&["--9", "--z-", "--a--b"]],
        ),
        (
            "{positionals: [{name: '_'}, {name: 'a-'}, {name: '9-9'}]}",
            &[&["1", "2", "3"]],
        ),
        (
            "{commands: {_: {}, ['9']: {}, ['a-']: {}}}",
            &[&["_"], &["a-"]],
        ),
    ]);
}
