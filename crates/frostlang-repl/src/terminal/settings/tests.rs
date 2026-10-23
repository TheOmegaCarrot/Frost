//! White-box: the settings take effect only on a terminal, which the public API
//! cannot drive. Covers parsing a section, computing each prompt, and how each
//! problem in a section or its prompts is caught and described.

use frostlang::Value;

use crate::config_runner::ConfigRunner;
use crate::terminal::settings::{Context, EditMode, Problem, Prompts, Setting, Settings};

/// The `terminal` export of the script `source`, run with color on, which must
/// run and export it.
#[track_caller]
fn section(source: &str) -> Value {
    section_colored(source, true)
}

/// The `terminal` export of the script `source`, run with color on or off
/// as `color` says.
#[track_caller]
fn section_colored(source: &str, color: bool) -> Value {
    let exports = ConfigRunner::new(color)
        .run_script("repl.frst", source)
        .unwrap_or_else(|failure| panic!("{source:?} failed: {failure:?}"));
    exports
        .into_iter()
        .find_map(|(name, value)| (name == "terminal").then_some(value))
        .unwrap_or_else(|| panic!("{source:?} exports no `terminal`"))
}

/// The settings the script `source` sets, with color on, and their problems.
#[track_caller]
fn load(source: &str) -> (Settings, Vec<Problem>) {
    Settings::from_section(section(source), true)
}

/// The settings the script `source` sets, which must have no problem.
#[track_caller]
fn load_clean(source: &str) -> Settings {
    let (settings, problems) = load(source);
    assert!(problems.is_empty(), "{source:?} has problems: {problems:?}");
    settings
}

/// The context of the first segment, after no failure, in `/home`.
fn first() -> Context {
    Context {
        number: 1,
        failed: false,
        cwd: Some("/home".to_owned()),
    }
}

/// A context that differs from `first` in having failed.
fn failed() -> Context {
    Context {
        failed: true,
        ..first()
    }
}

/// The prompts `settings` gives for `context`, which must cause no problem.
#[track_caller]
fn prompts(settings: &mut Settings, context: &Context) -> Prompts {
    let (prompts, problems) = settings.prompts(context);
    assert!(problems.is_empty(), "computing prompts: {problems:?}");
    prompts
}

/// `template`, a script, with `BODY` replaced by `body`.
fn with_body(template: &str, body: &str) -> String {
    template.replace("BODY", body)
}

/// The prompt `source` sets as `prompt`, computed for `context`.
#[track_caller]
fn prompt_for(source: &str, context: &Context) -> Option<String> {
    prompts(&mut load_clean(source), context).prompt
}

/// The `prompt` function whose body is `body`, given its context as `c`,
/// computed for `context`.
#[track_caller]
fn prompt_from_body(body: &str, context: &Context) -> Option<String> {
    let source = with_body(
        r"
        defn prompt(c) -> BODY
        export def terminal = {prompt}
        ",
        body,
    );
    prompt_for(&source, context)
}

fn texts(problems: &[Problem]) -> Vec<String> {
    problems.iter().map(ToString::to_string).collect()
}

// -- Settings --

#[test]
fn an_empty_section_sets_nothing_and_reports_nothing() {
    let mut settings = load_clean("export def terminal = {}");
    assert_eq!(
        prompts(&mut settings, &first()),
        Prompts::default(),
        "every default stands"
    );
}

#[test]
fn a_string_setting_is_the_prompt_as_written() {
    let mut settings = load_clean(
        r"
        export def terminal = {
            prompt: 'frost> ',
            continuation_prompt: '...... ',
            right_prompt: ' <frost',
        }
        ",
    );
    assert_eq!(
        prompts(&mut settings, &first()),
        Prompts {
            prompt: Some("frost> ".to_owned()),
            continuation_prompt: Some("...... ".to_owned()),
            right_prompt: Some(" <frost".to_owned()),
        }
    );
}

#[test]
fn each_setting_stands_alone() {
    let cases = [
        (
            "prompt",
            Prompts {
                prompt: Some("x".to_owned()),
                ..Prompts::default()
            },
        ),
        (
            "continuation_prompt",
            Prompts {
                continuation_prompt: Some("x".to_owned()),
                ..Prompts::default()
            },
        ),
        (
            "right_prompt",
            Prompts {
                right_prompt: Some("x".to_owned()),
                ..Prompts::default()
            },
        ),
    ];
    for (name, expected) in cases {
        let source = format!("export def terminal = {{{name}: 'x'}}");
        let mut settings = load_clean(&source);
        assert_eq!(prompts(&mut settings, &first()), expected, "{source}");
    }
}

#[test]
fn an_empty_string_is_an_empty_prompt_not_the_default() {
    let mut settings = load_clean(
        r"
        export def terminal = {prompt: '', continuation_prompt: '', right_prompt: ''}
        ",
    );
    assert_eq!(
        prompts(&mut settings, &first()),
        Prompts {
            prompt: Some(String::new()),
            continuation_prompt: Some(String::new()),
            right_prompt: Some(String::new()),
        }
    );
}

#[test]
fn a_prompt_function_is_given_the_context() {
    let source = r"
        defn prompt(context) -> $'${context.number} ${context.failed} ${context.cwd}> '
        export def terminal = {prompt}
    ";
    let cases = [
        (first(), "1 false /home> "),
        (
            Context {
                number: 7,
                failed: true,
                cwd: Some("/tmp/x".to_owned()),
            },
            "7 true /tmp/x> ",
        ),
        // A working directory that cannot be read is Null.
        (
            Context {
                number: 2,
                failed: false,
                cwd: None,
            },
            "2 false null> ",
        ),
    ];
    for (context, expected) in cases {
        assert_eq!(
            prompt_for(source, &context).as_deref(),
            Some(expected),
            "{context:?}"
        );
    }
}

#[test]
fn every_setting_may_be_a_function_of_the_context() {
    let mut settings = load_clean(
        r"
        export def terminal = {
            prompt: fn c -> $'p${c.number}',
            continuation_prompt: fn c -> $'c${c.number}',
            right_prompt: fn c -> $'r${c.number}',
        }
        ",
    );
    let context = Context {
        number: 4,
        ..first()
    };
    assert_eq!(
        prompts(&mut settings, &context),
        Prompts {
            prompt: Some("p4".to_owned()),
            continuation_prompt: Some("c4".to_owned()),
            right_prompt: Some("r4".to_owned()),
        }
    );
}

#[test]
fn the_context_holds_exactly_its_three_entries() {
    assert_eq!(
        prompt_from_body("join(keys(c), ',')", &first()).as_deref(),
        Some("cwd,failed,number")
    );
}

#[test]
fn the_context_entries_have_their_types() {
    assert_eq!(
        prompt_from_body(
            "join([type(c.number), type(c.failed), type(c.cwd)], ',')",
            &first()
        )
        .as_deref(),
        Some("Int,Bool,String")
    );
    let unreadable = Context {
        cwd: None,
        ..first()
    };
    assert_eq!(
        prompt_from_body("type(c.cwd)", &unreadable).as_deref(),
        Some("Null")
    );
}

#[test]
fn a_segment_number_past_the_int_range_saturates() {
    let context = Context {
        number: u64::MAX,
        ..first()
    };
    assert_eq!(
        prompt_from_body("to_string(c.number)", &context),
        Some(i64::MAX.to_string())
    );
}

#[test]
fn a_prompt_function_is_called_for_each_segment() {
    let mut settings = load_clean(
        r"
        export def terminal = {
            prompt: fn c -> $'[${c.number}] ',
            continuation_prompt: fn c -> $'(${c.number}) ',
            right_prompt: fn c -> $'<${c.number}>',
        }
        ",
    );
    for number in 1..=20 {
        let context = Context { number, ..first() };
        assert_eq!(
            prompts(&mut settings, &context),
            Prompts {
                prompt: Some(format!("[{number}] ")),
                continuation_prompt: Some(format!("({number}) ")),
                right_prompt: Some(format!("<{number}>")),
            }
        );
    }
}

#[test]
fn helpers_and_lambdas_shape_a_prompt() {
    let source = r"
        def arrow = '>'
        defn mark(failed) -> if failed: '!' else: ''
        export def terminal = {prompt: fn c -> $'${mark(c.failed)}${arrow} '}
    ";
    assert_eq!(prompt_for(source, &first()).as_deref(), Some("> "));
    assert_eq!(prompt_for(source, &failed()).as_deref(), Some("!> "));
}

#[test]
fn a_native_function_may_be_a_prompt() {
    let mut settings = load_clean(
        r"
        export def terminal = {prompt: to_string, right_prompt: to_string}
        ",
    );
    let prompts = prompts(&mut settings, &first());
    for prompt in [prompts.prompt, prompts.right_prompt] {
        let prompt = prompt.expect("a prompt");
        assert!(
            prompt.starts_with('{') && prompt.contains("number"),
            "the context as a String: {prompt:?}"
        );
    }
}

#[test]
fn a_prompt_function_may_use_the_modules_it_can_import() {
    let source = r"
        def string = import('std.string')
        def math = import('std.math')
        defn prompt(c) -> $'${string.strip_prefix(c.cwd, '/')} ${math.abs(-3)}> '
        export def terminal = {prompt}
    ";
    assert_eq!(prompt_for(source, &first()).as_deref(), Some("home 3> "));
}

#[test]
fn a_prompt_function_may_use_the_repl_component() {
    let source = r"
        def repl = import('repl')
        export def terminal = {prompt: fn c -> repl.colors.red('!')}
    ";
    assert_eq!(
        prompt_for(source, &first()).as_deref(),
        Some("\x1b[31m!\x1b[39m")
    );
}

#[test]
fn the_documented_example_works() {
    // The example in `TerminalFrontend`'s documentation, as written.
    let source = r"
        def repl = import('repl')

        defn prompt(context) -> {
            def number = repl.style.bold(to_string(context.number))
            if context.failed: $'${number} ${repl.colors.red('!')}> '
            else: $'${number}> '
        }

        export def terminal = {prompt, right_prompt: fn context -> context.cwd or ''}
    ";
    let mut settings = load_clean(source);
    assert_eq!(
        prompts(&mut settings, &first()),
        Prompts {
            prompt: Some("\x1b[1m1\x1b[22m> ".to_owned()),
            continuation_prompt: None,
            right_prompt: Some("/home".to_owned()),
        }
    );
    assert_eq!(
        prompts(&mut settings, &failed()),
        Prompts {
            prompt: Some("\x1b[1m1\x1b[22m \x1b[31m!\x1b[39m> ".to_owned()),
            continuation_prompt: None,
            right_prompt: Some("/home".to_owned()),
        }
    );
    let unreadable = Context {
        cwd: None,
        ..first()
    };
    assert_eq!(
        prompts(&mut settings, &unreadable).right_prompt.as_deref(),
        Some(""),
        "an unreadable working directory gives an empty right prompt"
    );
}

#[test]
fn without_color_the_painters_return_their_text_unchanged() {
    let source = r"
        def repl = import('repl')
        export def terminal = {prompt: fn c -> repl.style.bold(repl.colors.red('!'))}
    ";
    let (mut settings, problems) = Settings::from_section(section_colored(source, false), false);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(
        prompts(&mut settings, &first()).prompt.as_deref(),
        Some("!")
    );
}

#[test]
fn each_call_has_its_own_fuel() {
    // Each call spends a fair share of the limit; together, many times over it.
    let mut settings = load_clean(
        r"
        defn prompt(c) -> to_string(reduce(range(2000), 0, fn acc, x -> acc + x))
        export def terminal = {prompt}
        ",
    );
    for _ in 0..200 {
        assert_eq!(
            prompts(&mut settings, &first()).prompt.as_deref(),
            Some("1999000")
        );
    }
}

// -- The edit mode --

#[test]
fn lines_are_edited_with_emacs_keys_by_default() {
    let settings = load_clean("export def terminal = {prompt: '> '}");
    assert_eq!(settings.edit_mode(), EditMode::Emacs);
}

#[test]
fn edit_mode_names_the_keys_lines_are_edited_with() {
    for (name, mode) in [("emacs", EditMode::Emacs), ("vi", EditMode::Vi)] {
        let source = format!("export def terminal = {{edit_mode: '{name}'}}");
        assert_eq!(load_clean(&source).edit_mode(), mode, "{source}");
    }
}

/// The edit mode is a setting of its own: setting it leaves every prompt at
/// its default, and setting the prompts leaves it at emacs.
#[test]
fn the_edit_mode_stands_apart_from_the_prompts() {
    let mut settings = load_clean("export def terminal = {edit_mode: 'vi'}");
    assert_eq!(prompts(&mut settings, &first()), Prompts::default());
    let settings = load_clean(
        r"
        export def terminal = {
            prompt: '> ',
            continuation_prompt: '. ',
            right_prompt: 'r',
        }
        ",
    );
    assert_eq!(settings.edit_mode(), EditMode::Emacs);
}

/// Only the exact names count: no other spelling, case, or type of value.
/// An invalid section sets nothing, so the valid prompt beside it is lost too.
#[test]
fn an_edit_mode_that_names_none_sets_nothing() {
    for value in [
        "'Vi'",
        "'VI'",
        "'vim'",
        "'Emacs'",
        "' vi'",
        "''",
        "1",
        "true",
        "['vi']",
        "{vi: true}",
        "{vi: null}",
        "{emacs: 1}",
        "fn -> 'vi'",
        "x'7669'",
    ] {
        let source = format!(
            r"
            export def terminal = {{edit_mode: {value}, prompt: 'p> '}}
            "
        );
        let said = assert_sets_nothing(&source);
        assert_says(&source, &said, &["sets nothing"]);
    }
}

/// Like a prompt, a Null `edit_mode` is no opinion: emacs stands, and it is
/// not a problem.
#[test]
fn a_null_edit_mode_is_no_opinion() {
    let mut settings = load_clean("export def terminal = {edit_mode: null, prompt: 'p> '}");
    assert_eq!(settings.edit_mode(), EditMode::Emacs);
    assert_eq!(
        prompts(&mut settings, &first()).prompt.as_deref(),
        Some("p> "),
        "the prompt applies"
    );
}

// -- Problems with the section --

/// The settings the script `source` sets, which must be the defaults (no
/// prompt, emacs editing) with exactly one problem, `Invalid`. Returns what
/// that problem says.
#[track_caller]
fn assert_sets_nothing(source: &str) -> String {
    let (mut settings, problems) = load(source);
    assert!(
        matches!(problems.as_slice(), [Problem::Invalid(_)]),
        "{source:?}: {problems:?}"
    );
    assert_eq!(settings.edit_mode(), EditMode::Emacs, "{source:?}");
    assert_eq!(
        prompts(&mut settings, &first()),
        Prompts::default(),
        "{source:?}"
    );
    texts(&problems).remove(0)
}

#[test]
fn a_section_that_is_not_a_map_sets_nothing() {
    for (value, type_name) in [
        ("5", "Int"),
        ("1.5", "Float"),
        ("true", "Bool"),
        ("'prompt> '", "String"),
        ("x'3e'", "Bytes"),
        ("['prompt']", "Array"),
        ("null", "Null"),
        ("fn c -> 'prompt> '", "Function"),
    ] {
        let source = format!("export def terminal = {value}");
        let said = assert_sets_nothing(&source);
        assert_says(&source, &said, &["sets nothing"]);
        assert_says(&source, &said, &[type_name]);
    }
}

#[test]
fn an_unknown_key_makes_the_whole_section_set_nothing() {
    let said = assert_sets_nothing(
        r"
        export def terminal = {promt: 'typo> ', prompt: 'p> ', edit_mode: 'vi'}
        ",
    );
    assert_says("unknown key", &said, &["sets nothing", "promt"]);
}

#[test]
fn a_valid_section_with_an_unknown_key_sets_none_of_its_settings() {
    let source = r"
        export def terminal = {
            prompt: 'p> ',
            continuation_prompt: '.. ',
            right_prompt: 'r',
            edit_mode: 'vi',
            colour: 1,
        }
    ";
    let said = assert_sets_nothing(source);
    assert_says(source, &said, &["sets nothing", "colour"]);
}

#[test]
fn each_unknown_key_is_named() {
    for key in [
        "promt",
        "colour",
        "Prompt",
        "edit-mode",
        "",
        "right_prompt ",
    ] {
        let source = format!("export def terminal = {{['{key}']: 'x', right_prompt: 'r'}}");
        let said = assert_sets_nothing(&source);
        assert_says(&source, &said, &["sets nothing", key]);
    }
}

#[test]
fn a_key_that_is_not_a_string_is_an_unknown_key() {
    for key in ["1", "true", "2.5"] {
        let source = format!("export def terminal = {{[{key}]: 'x', right_prompt: 'r'}}");
        let said = assert_sets_nothing(&source);
        assert_says(&source, &said, &["sets nothing"]);
    }
}

#[test]
fn a_setting_of_the_wrong_type_makes_the_whole_section_set_nothing() {
    for wrong in Setting::ALL {
        for (value, type_name) in [
            ("5", "Int"),
            ("1.5", "Float"),
            ("true", "Bool"),
            ("['> ']", "Array"),
            ("{text: '> '}", "Map"),
            ("x'3e'", "Bytes"),
        ] {
            // The wrong setting holds `value`; each other holds its own name.
            let entries: Vec<String> = Setting::ALL
                .into_iter()
                .map(|setting| match setting == wrong {
                    true => format!("{}: {value}", setting.name()),
                    false => format!("{0}: '{0}'", setting.name()),
                })
                .collect();
            let source = format!("export def terminal = {{{}}}", entries.join(", "));
            let said = assert_sets_nothing(&source);
            assert_says(
                &source,
                &said,
                &["sets nothing", wrong.name(), &format!("got {type_name}")],
            );
        }
    }
}

#[test]
fn a_null_prompt_is_no_opinion() {
    for setting in Setting::ALL {
        let source = format!("export def terminal = {{{}: null}}", setting.name());
        let mut settings = load_clean(&source);
        assert_eq!(
            prompts(&mut settings, &first()),
            Prompts::default(),
            "{source}"
        );
    }
    let mut settings = load_clean(
        r"
        export def terminal = {prompt: null, continuation_prompt: '.. ', right_prompt: null}
        ",
    );
    assert_eq!(
        prompts(&mut settings, &first()),
        Prompts {
            continuation_prompt: Some(".. ".to_owned()),
            ..Prompts::default()
        },
        "a null prompt leaves the others to apply"
    );
}

#[test]
fn problems_in_a_section_are_reported_as_one() {
    let (_, problems) = load(
        r"
        export def terminal = {prompt: 5, typo: 'x', edit_mode: 'vim'}
        ",
    );
    assert!(
        matches!(problems.as_slice(), [Problem::Invalid(_)]),
        "{problems:?}"
    );
}

// -- Problems computing a prompt --

#[test]
fn a_prompt_function_that_fails_is_reported_once_and_dropped() {
    for setting in Setting::ALL {
        for body in [
            "error('broken')",
            "c.missing_key",
            "1 / 0",
            "to_upper(c)",
            // Runaway prompts exhaust their limits: fuel, then call depth.
            "forever(0)",
            "to_string(deep(0))",
        ] {
            let name = setting.name();
            let source = with_body(
                r"
                defn forever(n) -> forever(n + 1)
                defn deep(n) -> 1 + deep(n)
                defn failing(c) -> BODY
                export def terminal = {NAME: failing}
                ",
                body,
            )
            .replace("NAME", name);
            let mut settings = load_clean(&source);
            let (computed, problems) = settings.prompts(&first());
            assert_eq!(
                computed,
                Prompts::default(),
                "{name}, {body}: the default stands in"
            );
            assert!(
                matches!(
                    problems.as_slice(),
                    [Problem::PromptFailed { setting: s, .. }] if *s == setting
                ),
                "{name}, {body}: {problems:?}"
            );
            // Never called again: it would fail, and be reported, again.
            for _ in 0..3 {
                let (computed, problems) = settings.prompts(&first());
                assert_eq!(computed, Prompts::default(), "{name}, {body}");
                assert!(
                    problems.is_empty(),
                    "{name}, {body}: called again: {problems:?}"
                );
            }
        }
    }
}

#[test]
fn a_native_prompt_that_fails_is_dropped() {
    let mut settings = load_clean("export def terminal = {prompt: to_upper}");
    let (computed, problems) = settings.prompts(&first());
    assert_eq!(computed.prompt, None);
    assert!(
        matches!(problems.as_slice(), [Problem::PromptFailed { .. }]),
        "{problems:?}"
    );
    assert!(settings.prompts(&first()).1.is_empty(), "called again");
}

#[test]
fn a_prompt_function_that_takes_no_context_fails() {
    let mut settings = load_clean(
        r"
        defn prompt() -> '> '
        export def terminal = {prompt}
        ",
    );
    let (computed, problems) = settings.prompts(&first());
    assert_eq!(computed.prompt, None);
    assert!(
        matches!(problems.as_slice(), [Problem::PromptFailed { .. }]),
        "{problems:?}"
    );
}

#[test]
fn a_prompt_function_that_returns_no_string_is_reported_once_and_dropped() {
    for (body, type_name) in [
        ("5", "Int"),
        ("null", "Null"),
        ("['> ']", "Array"),
        ("c", "Map"),
        ("x'3e'", "Bytes"),
        ("fn -> '> '", "Function"),
    ] {
        let source = with_body(
            r"
            defn wrong(c) -> BODY
            export def terminal = {continuation_prompt: wrong, right_prompt: wrong}
            ",
            body,
        );
        let mut settings = load_clean(&source);
        let (computed, problems) = settings.prompts(&first());
        assert_eq!(computed, Prompts::default(), "{body}");
        assert!(
            matches!(
                problems.as_slice(),
                [
                    Problem::PromptNotString { setting: Setting::ContinuationPrompt, type_name: a },
                    Problem::PromptNotString { setting: Setting::RightPrompt, type_name: b },
                ] if *a == type_name && *b == type_name
            ),
            "{body}: {problems:?}"
        );
        let (_, problems) = settings.prompts(&first());
        assert!(problems.is_empty(), "{body}: called again: {problems:?}");
    }
}

#[test]
fn a_prompt_function_fails_only_when_it_fails() {
    // A prompt that fails only on a failed input works until then.
    let mut settings = load_clean(
        r"
        defn prompt(c) -> if c.failed: error('oops') else: '> '
        export def terminal = {prompt, continuation_prompt: '.. ', right_prompt: fn c -> 'r'}
        ",
    );
    let expected = Prompts {
        prompt: Some("> ".to_owned()),
        continuation_prompt: Some(".. ".to_owned()),
        right_prompt: Some("r".to_owned()),
    };
    assert_eq!(prompts(&mut settings, &first()), expected);
    let (dropped, problems) = settings.prompts(&failed());
    assert_eq!(problems.len(), 1, "{problems:?}");
    // The other settings are untouched.
    assert_eq!(
        dropped,
        Prompts {
            prompt: None,
            ..expected.clone()
        }
    );
    assert_eq!(
        prompts(&mut settings, &first()),
        Prompts {
            prompt: None,
            ..expected
        },
        "the dropped prompt stays dropped"
    );
}

#[test]
fn settings_that_fail_are_all_reported_and_latch_independently() {
    let mut settings = load_clean(
        r"
        export def terminal = {
            prompt: fn c -> 1,
            continuation_prompt: fn c -> error('no'),
            right_prompt: fn c -> if c.failed: error('later') else: 'r',
        }
        ",
    );
    let (computed, problems) = settings.prompts(&first());
    assert_eq!(
        computed,
        Prompts {
            right_prompt: Some("r".to_owned()),
            ..Prompts::default()
        }
    );
    assert!(
        matches!(
            problems.as_slice(),
            [
                Problem::PromptNotString {
                    setting: Setting::Prompt,
                    ..
                },
                Problem::PromptFailed {
                    setting: Setting::ContinuationPrompt,
                    ..
                },
            ]
        ),
        "{problems:?}"
    );
    // The right prompt, still live, fails and is dropped in its turn.
    let (computed, problems) = settings.prompts(&failed());
    assert_eq!(computed, Prompts::default());
    assert!(
        matches!(
            problems.as_slice(),
            [Problem::PromptFailed {
                setting: Setting::RightPrompt,
                ..
            }]
        ),
        "{problems:?}"
    );
}

// -- What a problem says --

/// Asserts that `said` holds each of `fragments`.
#[track_caller]
fn assert_says(context: &str, said: &str, fragments: &[&str]) {
    for fragment in fragments {
        assert!(
            said.contains(fragment),
            "{context} says {said:?}, lacking {fragment:?}"
        );
    }
}

#[test]
fn each_section_problem_says_what_went_wrong_and_what_stands_in() {
    let cases: [(&str, &[&str]); 6] = [
        (
            "export def terminal = 5",
            &["Warning", "`terminal`", "sets nothing", "Int"],
        ),
        (
            "export def terminal = {promt: '> '}",
            &["Warning", "`terminal`", "sets nothing", "promt"],
        ),
        (
            "export def terminal = {prompt: 5}",
            &[
                "Warning",
                "sets nothing",
                "expected String or Function, got Int",
                "prompt",
            ],
        ),
        (
            "export def terminal = {continuation_prompt: []}",
            &[
                "sets nothing",
                "expected String or Function, got Array",
                "continuation_prompt",
            ],
        ),
        (
            "export def terminal = {edit_mode: 'vim'}",
            &["Warning", "sets nothing", "vim", "edit_mode"],
        ),
        (
            "export def terminal = {edit_mode: true}",
            &["sets nothing", "edit_mode"],
        ),
    ];
    for (source, fragments) in cases {
        let (_, problems) = load(source);
        assert_eq!(problems.len(), 1, "{source:?}: {problems:?}");
        assert_says(source, &texts(&problems).join("\n"), fragments);
    }
}

#[test]
fn a_failed_prompt_says_so_and_that_the_default_stands_in() {
    let mut settings = load_clean(
        r"
        export def terminal = {
            prompt: fn c -> error('kaboom'),
            continuation_prompt: fn c -> 5,
            right_prompt: fn c -> error('far'),
        }
        ",
    );
    let said = texts(&settings.prompts(&first()).1);
    assert_eq!(said.len(), 3, "{said:?}");
    assert_says(
        "prompt",
        &said[0],
        &[
            "Error",
            "`terminal.prompt` failed",
            "the default prompt is used from now on",
            "kaboom",
        ],
    );
    assert_says(
        "continuation_prompt",
        &said[1],
        &[
            "Error",
            "`terminal.continuation_prompt` must return a String, got Int",
            "the default continuation prompt is used from now on",
        ],
    );
    assert_says(
        "right_prompt",
        &said[2],
        &[
            "Error",
            "`terminal.right_prompt` failed",
            "no right prompt is used from now on",
            "far",
        ],
    );
}
