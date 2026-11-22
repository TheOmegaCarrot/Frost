//! Specs and command lines that are odd or excessive, but valid: each parses
//! as the rules say. They build in debug, where clap checks its commands with
//! assertions that panic, so none of them may trip one.

use frostlang::Value;

use crate::common::{parsed, text, usage_error, value};

/// `text` as a Frost String literal.
fn quoted(text: &str) -> String {
    Value::from(text).to_debug_string()
}

/// A spec of commands nested along `path`, the innermost being `innermost`.
fn nested(path: &[&str], innermost: &str) -> String {
    path.iter()
        .rev()
        .fold(innermost.to_string(), |inner, name| {
            format!("{{commands: {{[{}]: {inner}}}}}", quoted(name))
        })
}

/// The value of a command line choosing commands along `path`, the innermost
/// command's own value being `innermost`.
fn nested_value(path: &[&str], innermost: &str) -> String {
    path.iter()
        .rev()
        .fold(innermost.to_string(), |inner, name| {
            format!("{{command: {{[{}]: {inner}}}}}", quoted(name))
        })
}

#[test]
fn commands_nest_deeply_and_reuse_names_at_each_level() {
    let path = ["this", "seems", "excessive", "but", "does", "this", "work"];
    let spec = nested(&path, "{flags: {yes: {short: 'y'}}}");
    let mut arguments = path.to_vec();
    arguments.push("-y");
    assert_eq!(
        parsed(&spec, &arguments),
        value(&nested_value(&path, "{yes: true}"))
    );
    let help = text("help", &spec, &[&path[..], &["--help"]].concat());
    assert!(
        help.starts_with("Usage: prog this seems excessive but does this work [OPTIONS]"),
        "{help}"
    );
    // Stopping short names the command that needs another.
    let error = usage_error(&spec, &path[..3]);
    assert!(error.contains("'prog this seems excessive'"), "{error}");
    assert!(error.contains("but"), "{error}");
}

#[test]
fn commands_nest_as_deep_as_they_may() {
    // Thirty-two levels, the most a spec may nest, each command with an
    // argument of each kind, so that the deepest command line builds and
    // parses an argument at every level.
    let script = r"
        def names = map(range(32), fn level -> $'c${level}')
        def level_spec = fn inner -> {
            flags: {f: {short: 'f'}},
            options: {o: {default: 'd'}},
            commands: inner,
        }
        def innermost = {positionals: [{name: 'last'}]}
        def spec = reduce(reverse(names), innermost, fn inner, name -> level_spec({[name]: inner}))
        def level_value = fn name, inner -> {f: true, o: 'd', command: {[name]: inner}}
        def expected = reduce(reverse(names), {last: 'end'}, fn inner, name -> level_value(name, inner))
        def chosen = flat_map(names, fn name -> ['-f', name])
        def help_usage = $'Usage: prog ${join(names, ' ')} <last>'
        [
            argparse.parse(['prog'] + chosen + ['end'], spec) == {value: expected},
            argparse.parse(['prog'] + names + ['--help'], spec).help @ starts_with(help_usage),
            has(argparse.parse(['prog'] + names, spec), 'error'),
        ]
    ";
    assert_eq!(value(script), value("[true, true, true]"), "{script}");
}

#[test]
fn a_command_may_have_hundreds_of_subcommands() {
    let commands: Vec<String> = (0..300)
        .map(|i| format!("c{i}: {{flags: {{f{i}: {{}}}}}}"))
        .collect();
    let spec = format!("{{commands: {{{}}}}}", commands.join(", "));
    for i in [0, 1, 150, 299] {
        assert_eq!(
            parsed(&spec, &[&format!("c{i}"), &format!("--f{i}")]),
            value(&format!("{{command: {{c{i}: {{f{i}: true}}}}}}")),
            "c{i}"
        );
    }
}

#[test]
fn every_free_letter_may_be_a_short_name_at_once() {
    // `h` is built in, and so is `V` with a version; here there is none.
    let letters: Vec<char> = ('a'..='z').chain('A'..='Z').filter(|&c| c != 'h').collect();
    let flags: Vec<String> = letters
        .iter()
        .map(|c| format!("f_{c}: {{short: '{c}'}}"))
        .collect();
    let spec = format!("{{flags: {{{}}}}}", flags.join(", "));
    let bundle = format!("-{}", letters.iter().collect::<String>());
    let all_true: Vec<String> = letters.iter().map(|c| format!("f_{c}: true")).collect();
    assert_eq!(
        parsed(&spec, &[&bundle]),
        value(&format!("{{{}}}", all_true.join(", ")))
    );
}

#[test]
fn keywords_and_type_names_may_name_anything() {
    let spec = r"{
        flags: {['if']: {}, ['not']: {}, String: {short: 's'}},
        options: {['null']: {}, ['true']: {default: 'false'}},
        commands: {
            ['def']: {positionals: [{name: 'match'}, {name: 'fn', required: false}]},
        },
    }";
    assert_eq!(
        parsed(spec, &["--if", "-s", "--null", "x", "def", "m"]),
        value(
            r"{
                ['if']: true,
                ['not']: false,
                String: true,
                ['null']: 'x',
                ['true']: 'false',
                command: {['def']: {['match']: 'm', ['fn']: null}},
            }"
        ),
    );
}

#[test]
fn names_and_values_may_be_very_long() {
    // Built when the script runs: the lexer of a debug build cannot take a
    // String literal this long (FROST-214).
    let script = r"
        defn repeated(text, count) -> join(map(range(count), fn _ -> text), '')
        def name = repeated('n', 2000)
        def long_value = repeated('v', 100000)
        def spec = {
            options: {[name]: {}},
            positionals: [{name: 'p' + name}],
        }
        def parsed = argparse.parse(['prog', '--' + name, long_value, long_value], spec)
        parsed == {value: {[name]: long_value, ['p' + name]: long_value}}
    ";
    assert_eq!(value(script), Value::Bool(true), "{script}");
}

#[test]
fn a_command_line_may_be_very_long() {
    let values: Vec<String> = (0..5000).map(|i| i.to_string()).collect();
    let mut arguments: Vec<&str> = Vec::new();
    for value in &values {
        arguments.extend(["-t", value]);
    }
    arguments.extend(values.iter().map(String::as_str));
    let all = values
        .iter()
        .map(|value| quoted(value))
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(
        parsed(
            "{options: {tag: {short: 't', repeatable: true}}, \
              positionals: [{name: 'rest', many: true}]}",
            &arguments
        ),
        value(&format!("{{tag: [{all}], rest: [{all}]}}")),
    );
}

#[test]
fn a_command_may_have_a_hundred_positionals() {
    let positionals: Vec<String> = (0..100).map(|i| format!("{{name: 'p{i}'}}")).collect();
    let values: Vec<String> = (0..100).map(|i| format!("v{i}")).collect();
    let arguments: Vec<&str> = values.iter().map(String::as_str).collect();
    let expected: Vec<String> = (0..100).map(|i| format!("p{i}: 'v{i}'")).collect();
    assert_eq!(
        parsed(
            &format!("{{positionals: [{}]}}", positionals.join(", ")),
            &arguments
        ),
        value(&format!("{{{}}}", expected.join(", "))),
    );
}

#[test]
fn values_may_hold_anything_a_string_can() {
    for odd in [
        "",
        " ",
        "\n",
        "a\nb",
        "\t-x",
        "\u{0}",
        "=",
        "==",
        "caf\u{e9} \u{1f600}",
        "\u{202e}rtl",
        "\"'`$",
        "${x}",
    ] {
        assert_eq!(
            parsed(
                "{options: {o: {}}, positionals: [{name: 'p'}]}",
                &[&format!("--o={odd}"), "--", odd]
            ),
            value(&format!("{{o: {}, p: {}}}", quoted(odd), quoted(odd))),
            "{odd:?}"
        );
    }
}

#[test]
fn choices_and_defaults_may_look_like_command_line_syntax() {
    let spec = r"{
        options: {
            o: {choices: ['-x', '--y', '-1', '', '--', '-'], default: '--y'},
        },
        positionals: [{name: 'p', choices: ['-', '-2', '--'], default: '-'}],
    }";
    assert_eq!(parsed(spec, &[]), value("{o: '--y', p: '-'}"));
    for choice in ["-x", "--y", "-1", "", "--", "-"] {
        assert_eq!(
            parsed(spec, &[&format!("--o={choice}")]),
            value(&format!("{{o: {}, p: '-'}}", quoted(choice))),
            "{choice:?}"
        );
    }
    assert_eq!(parsed(spec, &["-2"]), value("{o: '--y', p: '-2'}"));
    assert_eq!(parsed(spec, &["--", "--"]), value("{o: '--y', p: '--'}"));
}

#[test]
fn text_for_the_help_may_hold_anything_a_string_can() {
    let odd = "line one\nline two \u{1f600}\t[brackets] <angles> {braces} %s {0} --flag";
    let spec = format!(
        "{{name: {q}, description: {q}, version: {q}, \
          flags: {{f: {{description: {q}}}}}, \
          options: {{o: {{description: {q}, value_name: {q}}}}}, \
          positionals: [{{name: 'p', description: {q}, required: false}}], }}",
        q = quoted(odd)
    );
    let help = text("help", &spec, &["--help"]);
    assert!(help.contains("line two \u{1f600}"), "{help}");
    let version = text("version", &spec, &["--version"]);
    assert!(version.contains("{braces} %s {0} --flag"), "{version}");
    assert_eq!(
        parsed(&spec, &["--f", "--o", "x", "y"]),
        value("{f: true, o: 'x', p: 'y'}")
    );
}

#[test]
fn a_program_may_have_an_odd_name() {
    for program in ["", " ", "a b", "/", "//", "caf\u{e9}", ".", "..", "-", "--"] {
        let help = value(&format!(
            "argparse.parse([{}, '--help'], {{}}).help",
            quoted(program)
        ));
        assert!(
            matches!(help, Value::String(_)),
            "{program:?}: help is a String, but got {help:?}"
        );
    }
}

#[test]
fn a_subcommand_may_share_a_name_with_its_parents_arguments() {
    let spec = r"{
        flags: {run: {short: 'r'}},
        options: {go: {}},
        commands: {
            run: {flags: {run: {short: 'r'}}, commands: {go: {}}},
            go: {},
        },
    }";
    assert_eq!(
        parsed(spec, &["-r", "--go", "x", "run", "--run", "go"]),
        value("{run: true, go: 'x', command: {run: {run: true, command: {go: {}}}}}"),
    );
    // A value for the option, not the command it names.
    assert_eq!(
        parsed(spec, &["--go", "go", "go"]),
        value("{run: false, go: 'go', command: {go: {}}}"),
    );
}
