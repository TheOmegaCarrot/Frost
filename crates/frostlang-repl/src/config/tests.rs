//! White-box: the configuration's effects show only on a terminal, which the
//! public API cannot drive. Covers each setting, the `repl` component, and how
//! each problem in a configuration or its prompts is caught.

use std::fs;
use std::path::PathBuf;
use std::process;

use crate::config::{Config, Context, Problem, Prompts, Setting};

/// The configuration `source` sets, with color on, and its problems.
fn load(source: &str) -> (Config, Vec<Problem>) {
    Config::load("repl.frst", source, true)
}

/// The configuration `source` sets, which must have no problem.
#[track_caller]
fn load_clean(source: &str) -> Config {
    let (config, problems) = load(source);
    assert!(problems.is_empty(), "{source:?} has problems: {problems:?}");
    config
}

/// The context of the first segment, after no failure, in `/home`.
fn first() -> Context {
    Context {
        number: 1,
        failed: false,
        cwd: Some("/home".to_owned()),
    }
}

/// The prompts `config` gives for `context`, which must cause no problem.
#[track_caller]
fn prompts(config: &mut Config, context: &Context) -> Prompts {
    let (prompts, problems) = config.prompts(context);
    assert!(problems.is_empty(), "computing prompts: {problems:?}");
    prompts
}

/// The prompt `source` computes for `context`, as `prompt` exports it.
#[track_caller]
fn prompt_for(source: &str, context: &Context) -> Option<String> {
    prompts(&mut load_clean(source), context).prompt
}

fn texts(problems: &[Problem]) -> Vec<String> {
    problems.iter().map(ToString::to_string).collect()
}

// -- Settings --

#[test]
fn an_empty_configuration_keeps_every_default() {
    for source in ["", "# nothing here", "def x = 1", "1 + 2"] {
        let mut config = load_clean(source);
        assert_eq!(
            prompts(&mut config, &first()),
            Prompts::default(),
            "{source:?}"
        );
    }
}

#[test]
fn the_default_configuration_keeps_every_default() {
    let mut config = Config::default();
    assert_eq!(prompts(&mut config, &first()), Prompts::default());
}

#[test]
fn a_string_setting_is_the_prompt_as_written() {
    let mut config = load_clean(
        r"
        export def prompt = 'frost> '
        export def continuation_prompt = '...... '
        ",
    );
    assert_eq!(
        prompts(&mut config, &first()),
        Prompts {
            prompt: Some("frost> ".to_owned()),
            continuation_prompt: Some("...... ".to_owned()),
        }
    );
}

#[test]
fn each_setting_stands_alone() {
    let mut config = load_clean("export def continuation_prompt = '| '");
    assert_eq!(
        prompts(&mut config, &first()),
        Prompts {
            prompt: None,
            continuation_prompt: Some("| ".to_owned()),
        }
    );
    let mut config = load_clean("export def prompt = '$ '");
    assert_eq!(
        prompts(&mut config, &first()),
        Prompts {
            prompt: Some("$ ".to_owned()),
            continuation_prompt: None,
        }
    );
}

#[test]
fn an_empty_string_is_an_empty_prompt_not_the_default() {
    assert_eq!(
        prompt_for("export def prompt = ''", &first()),
        Some(String::new())
    );
}

#[test]
fn a_prompt_function_is_given_the_context() {
    let source = r"
        export defn prompt(context) -> $'${context.number} ${context.failed} ${context.cwd}> '
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
fn the_context_holds_exactly_its_three_entries() {
    let source = "export defn prompt(context) -> join(keys(context), ',')";
    assert_eq!(
        prompt_for(source, &first()).as_deref(),
        Some("cwd,failed,number")
    );
}

#[test]
fn the_context_entries_have_their_types() {
    let source = r"
        export defn prompt(c) -> join([type(c.number), type(c.failed), type(c.cwd)], ',')
    ";
    assert_eq!(
        prompt_for(source, &first()).as_deref(),
        Some("Int,Bool,String")
    );
}

#[test]
fn a_segment_number_past_the_int_range_saturates() {
    let context = Context {
        number: u64::MAX,
        ..first()
    };
    assert_eq!(
        prompt_for("export defn prompt(c) -> to_string(c.number)", &context),
        Some(i64::MAX.to_string())
    );
}

#[test]
fn a_prompt_function_is_called_for_each_segment() {
    let mut config = load_clean(
        r"
        export defn prompt(c) -> $'[${c.number}] '
        export defn continuation_prompt(c) -> $'(${c.number}) '
        ",
    );
    for number in 1..=20 {
        let context = Context { number, ..first() };
        assert_eq!(
            prompts(&mut config, &context),
            Prompts {
                prompt: Some(format!("[{number}] ")),
                continuation_prompt: Some(format!("({number}) ")),
            }
        );
    }
}

#[test]
fn helpers_and_lambdas_shape_a_prompt() {
    let source = r"
        def arrow = '>'
        defn mark(failed) -> if failed: '!' else: ''
        export def prompt = fn c -> $'${mark(c.failed)}${arrow} '
    ";
    assert_eq!(prompt_for(source, &first()).as_deref(), Some("> "));
    let failed = Context {
        failed: true,
        ..first()
    };
    assert_eq!(prompt_for(source, &failed).as_deref(), Some("!> "));
}

#[test]
fn a_definition_not_exported_sets_nothing() {
    // The frontend never sees a plain `def`, even one named like a setting.
    let mut config = load_clean(
        r"
        def prompt = 'hidden> '
        defn continuation_prompt(c) -> 'hidden'
        ",
    );
    assert_eq!(prompts(&mut config, &first()), Prompts::default());
}

#[test]
fn a_native_function_may_be_a_prompt() {
    let mut config = load_clean("export def prompt = to_string");
    let prompt = prompts(&mut config, &first()).prompt.expect("a prompt");
    assert!(
        prompt.starts_with('{') && prompt.contains("number"),
        "the context as a String: {prompt:?}"
    );
}

#[test]
fn a_prompt_function_may_use_the_modules_it_can_import() {
    let source = r"
        def string = import('std.string')
        def math = import('std.math')
        export defn prompt(c) -> $'${string.strip_prefix(c.cwd, '/')} ${math.abs(-3)}> '
    ";
    assert_eq!(prompt_for(source, &first()).as_deref(), Some("home 3> "));
}

#[test]
fn the_documented_example_works() {
    // The example in `TerminalFrontend`'s documentation.
    let source = r"
        def repl = import('repl')

        export defn prompt(context) -> {
            def number = repl.style.bold(to_string(context.number))
            if context.failed: $'${number} ${repl.colors.red('!')}> '
            else: $'${number}> '
        }
    ";
    assert_eq!(
        prompt_for(source, &first()).as_deref(),
        Some("\x1b[1m1\x1b[22m> ")
    );
    let failed = Context {
        failed: true,
        ..first()
    };
    assert_eq!(
        prompt_for(source, &failed).as_deref(),
        Some("\x1b[1m1\x1b[22m \x1b[31m!\x1b[39m> ")
    );
}

#[test]
fn each_call_has_its_own_fuel() {
    // Each call spends a fair share of the limit; together, many times over it.
    let mut config = load_clean(
        r"
        export defn prompt(c) -> to_string(fold(range(2000), 0, fn acc, x -> acc + x))
        ",
    );
    for _ in 0..200 {
        assert_eq!(
            prompts(&mut config, &first()).prompt.as_deref(),
            Some("1999000")
        );
    }
}

// -- The `repl` component --

/// Each of `repl.colors`, and its SGR code.
const COLORS: [(&str, u8); 16] = [
    ("black", 30),
    ("red", 31),
    ("green", 32),
    ("yellow", 33),
    ("blue", 34),
    ("magenta", 35),
    ("cyan", 36),
    ("white", 37),
    ("bright_black", 90),
    ("bright_red", 91),
    ("bright_green", 92),
    ("bright_yellow", 93),
    ("bright_blue", 94),
    ("bright_magenta", 95),
    ("bright_cyan", 96),
    ("bright_white", 97),
];

/// Each of `repl.style`, and its SGR codes on and off.
const STYLES: [(&str, u8, u8); 4] = [
    ("bold", 1, 22),
    ("dim", 2, 22),
    ("italic", 3, 23),
    ("underline", 4, 24),
];

/// What `call`, an expression with `repl` bound, computes as the prompt, with
/// color on or off.
#[track_caller]
fn painted(call: &str, color: bool) -> String {
    let source = format!("def repl = import('repl')\nexport defn prompt(context) -> {call}");
    let (mut config, problems) = Config::load("repl.frst", &source, color);
    assert!(problems.is_empty(), "{source:?}: {problems:?}");
    prompts(&mut config, &first())
        .prompt
        .expect("the prompt is set")
}

#[test]
fn each_color_wraps_its_text_in_its_code() {
    for (name, code) in COLORS {
        assert_eq!(
            painted(&format!("repl.colors.{name}('hi')"), true),
            format!("\x1b[{code}mhi\x1b[39m"),
            "repl.colors.{name}"
        );
    }
}

#[test]
fn each_style_wraps_its_text_in_its_codes() {
    for (name, on, off) in STYLES {
        assert_eq!(
            painted(&format!("repl.style.{name}('hi')"), true),
            format!("\x1b[{on}mhi\x1b[{off}m"),
            "repl.style.{name}"
        );
    }
}

#[test]
fn without_color_every_painter_returns_its_text_unchanged() {
    let painters = COLORS
        .iter()
        .map(|(name, _)| format!("repl.colors.{name}"))
        .chain(
            STYLES
                .iter()
                .map(|(name, _, _)| format!("repl.style.{name}")),
        );
    for painter in painters {
        assert_eq!(
            painted(&format!("{painter}('hi')"), false),
            "hi",
            "{painter}"
        );
    }
}

#[test]
fn the_component_holds_exactly_its_painters() {
    // Keys come out sorted.
    let sorted = |mut names: Vec<&str>| {
        names.sort_unstable();
        names.join(",")
    };
    assert_eq!(painted("join(keys(repl), ',')", true), "colors,style");
    assert_eq!(
        painted("join(keys(repl.colors), ',')", true),
        sorted(COLORS.iter().map(|(name, _)| *name).collect())
    );
    assert_eq!(
        painted("join(keys(repl.style), ',')", true),
        sorted(STYLES.iter().map(|(name, _, _)| *name).collect())
    );
}

#[test]
fn painters_nest_and_wrap_empty_text() {
    assert_eq!(
        painted("repl.style.bold(repl.colors.green('ok'))", true),
        "\x1b[1m\x1b[32mok\x1b[39m\x1b[22m"
    );
    assert_eq!(painted("repl.colors.red('')", true), "\x1b[31m\x1b[39m");
    assert_eq!(painted("repl.colors.red('')", false), "");
}

/// The problems computing the prompt `call` gives, with `repl` bound.
fn painter_problems(call: &str) -> Vec<String> {
    let source = format!("def repl = import('repl')\nexport defn prompt(context) -> {call}");
    let mut config = load_clean(&source);
    texts(&config.prompts(&first()).1)
}

#[test]
fn a_painter_takes_exactly_one_argument() {
    for call in [
        "repl.colors.red()",
        "repl.colors.red('a', 'b')",
        "repl.style.bold()",
        "repl.style.bold('a', 'b')",
    ] {
        let problems = painter_problems(call);
        assert_eq!(problems.len(), 1, "{call}: {problems:?}");
        assert!(problems[0].contains("argument"), "{call}: {problems:?}");
    }
}

#[test]
fn a_painter_takes_only_a_string() {
    // Each error also names the painter by its place.
    for (call, name, type_name) in [
        ("repl.colors.red(5)", "repl.colors.red", "Int"),
        (
            "repl.colors.bright_cyan(null)",
            "repl.colors.bright_cyan",
            "Null",
        ),
        ("repl.style.dim(['a'])", "repl.style.dim", "Array"),
        (
            "repl.style.underline(x'00')",
            "repl.style.underline",
            "Bytes",
        ),
    ] {
        let problems = painter_problems(call);
        assert_eq!(problems.len(), 1, "{call}: {problems:?}");
        assert!(
            problems[0].contains(name)
                && problems[0].contains("String")
                && problems[0].contains(type_name),
            "{call}: {problems:?}"
        );
    }
}

// -- Imports --

#[test]
fn the_configuration_may_import_only_its_modules() {
    for module in ["std.string", "std.math", "repl"] {
        load_clean(&format!("def m = import('{module}')"));
    }
    for module in [
        "std.os",
        "std.fs",
        "std.random",
        "std.regex",
        "std.encoding",
        "ext.json",
        "app",
    ] {
        let (_, problems) = load(&format!("def m = import('{module}')"));
        assert!(
            matches!(problems.as_slice(), [Problem::Run(_)]),
            "importing {module}: {problems:?}"
        );
    }
}

// -- Problems loading the configuration --

#[test]
fn an_unknown_export_is_reported_and_the_settings_still_apply() {
    let (mut config, problems) = load(
        r"
        export def promt = 'typo> '
        export def prompt = 'p> '
        export def colour = 1
        ",
    );
    let mut unknown: Vec<&str> = problems
        .iter()
        .map(|problem| match problem {
            Problem::UnknownExport(name) => name.as_str(),
            other => panic!("only unknown exports: {other:?}"),
        })
        .collect();
    unknown.sort_unstable();
    assert_eq!(unknown, ["colour", "promt"]);
    assert_eq!(
        prompts(&mut config, &first()).prompt.as_deref(),
        Some("p> ")
    );
}

#[test]
fn a_setting_of_the_wrong_type_is_reported_and_the_others_still_apply() {
    for (value, type_name) in [
        ("5", "Int"),
        ("1.5", "Float"),
        ("true", "Bool"),
        ("null", "Null"),
        ("['> ']", "Array"),
        ("{text: '> '}", "Map"),
        ("x'3e'", "Bytes"),
    ] {
        let source = format!("export def prompt = {value}\nexport def continuation_prompt = '.. '");
        let (mut config, problems) = load(&source);
        assert!(
            matches!(
                problems.as_slice(),
                [Problem::WrongType { setting: Setting::Prompt, type_name: t }] if *t == type_name
            ),
            "{source:?}: {problems:?}"
        );
        assert_eq!(
            prompts(&mut config, &first()),
            Prompts {
                prompt: None,
                continuation_prompt: Some(".. ".to_owned()),
            },
            "{source:?}"
        );
    }
}

#[test]
fn a_configuration_that_does_not_compile_sets_nothing() {
    for source in [
        "export def prompt = ",
        "export def prompt = '> '\ndef = 1",
        "export def prompt = unbound_name",
    ] {
        let (mut config, problems) = load(source);
        assert!(
            matches!(problems.as_slice(), [Problem::Compile(_)]),
            "{source:?}: {problems:?}"
        );
        assert_eq!(
            prompts(&mut config, &first()),
            Prompts::default(),
            "{source:?}"
        );
    }
}

#[test]
fn a_configuration_that_fails_while_running_sets_nothing() {
    for source in [
        "export def prompt = '> '\nerror('broken')",
        "export def prompt = 1 / 0",
        // A runaway configuration exhausts its limits.
        "defn forever(n) -> forever(n + 1)\nexport def prompt = forever(0)",
        "defn deep(n) -> 1 + deep(n)\nexport def prompt = to_string(deep(0))",
    ] {
        let (mut config, problems) = load(source);
        assert!(
            matches!(problems.as_slice(), [Problem::Run(_)]),
            "{source:?}: {problems:?}"
        );
        assert_eq!(
            prompts(&mut config, &first()),
            Prompts::default(),
            "{source:?}"
        );
    }
}

// -- Problems computing a prompt --

#[test]
fn a_prompt_function_that_fails_is_reported_once_and_dropped() {
    for body in [
        "error('broken')",
        "c.missing_key",
        "1 / 0",
        "to_upper(c)",
        // Runaway prompts exhaust their limits.
        "forever(0)",
        "to_string(deep(0))",
    ] {
        let source = format!(
            r"
            defn forever(n) -> forever(n + 1)
            defn deep(n) -> 1 + deep(n)
            export defn prompt(c) -> {body}
            "
        );
        let mut config = load_clean(&source);
        let (prompts, problems) = config.prompts(&first());
        assert_eq!(prompts.prompt, None, "{body}: the default stands in");
        assert!(
            matches!(
                problems.as_slice(),
                [Problem::PromptFailed {
                    setting: Setting::Prompt,
                    ..
                }]
            ),
            "{body}: {problems:?}"
        );
        // Never called again: it would fail, and be reported, again.
        for _ in 0..3 {
            let (prompts, problems) = config.prompts(&first());
            assert_eq!(prompts.prompt, None, "{body}");
            assert!(problems.is_empty(), "{body}: called again: {problems:?}");
        }
    }
}

#[test]
fn a_native_prompt_that_fails_is_dropped() {
    let mut config = load_clean("export def prompt = to_upper");
    let (prompts, problems) = config.prompts(&first());
    assert_eq!(prompts.prompt, None);
    assert!(
        matches!(problems.as_slice(), [Problem::PromptFailed { .. }]),
        "{problems:?}"
    );
}

#[test]
fn a_prompt_function_that_takes_no_context_fails() {
    let mut config = load_clean("export defn prompt() -> '> '");
    let (prompts, problems) = config.prompts(&first());
    assert_eq!(prompts.prompt, None);
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
        let mut config = load_clean(&format!("export defn continuation_prompt(c) -> {body}"));
        let (prompts, problems) = config.prompts(&first());
        assert_eq!(prompts.continuation_prompt, None, "{body}");
        assert!(
            matches!(
                problems.as_slice(),
                [Problem::PromptNotString { setting: Setting::ContinuationPrompt, type_name: t }]
                    if *t == type_name
            ),
            "{body}: {problems:?}"
        );
        let (_, problems) = config.prompts(&first());
        assert!(problems.is_empty(), "{body}: called again: {problems:?}");
    }
}

#[test]
fn a_prompt_function_fails_only_when_it_fails() {
    // A prompt that fails only on a failed input works until then.
    let mut config = load_clean(
        r"
        export defn prompt(c) -> if c.failed: error('oops') else: '> '
        export def continuation_prompt = '.. '
        ",
    );
    assert_eq!(prompts(&mut config, &first()).prompt.as_deref(), Some("> "));
    let failed = Context {
        failed: true,
        ..first()
    };
    let (dropped, problems) = config.prompts(&failed);
    assert_eq!(problems.len(), 1, "{problems:?}");
    // The other setting is untouched.
    assert_eq!(
        dropped,
        Prompts {
            prompt: None,
            continuation_prompt: Some(".. ".to_owned()),
        }
    );
    assert_eq!(prompts(&mut config, &first()).prompt, None);
}

#[test]
fn both_prompt_functions_failing_are_both_reported() {
    let mut config = load_clean(
        r"
        export defn prompt(c) -> 1
        export defn continuation_prompt(c) -> error('no')
        ",
    );
    let (prompts, problems) = config.prompts(&first());
    assert_eq!(prompts, Prompts::default());
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
}

// -- What a problem says --

#[test]
fn each_problem_says_what_went_wrong_and_what_stands_in() {
    let cases: [(&str, &[&str]); 6] = [
        (
            "export def promt = '> '",
            &[
                "Warning",
                "exports `promt`",
                "not a setting",
                "`prompt` and `continuation_prompt`",
            ],
        ),
        (
            "export def prompt = 5",
            &[
                "Warning",
                "`prompt` must be a String or a Function, got Int",
                "the default prompt",
            ],
        ),
        (
            "export def continuation_prompt = []",
            &[
                "`continuation_prompt`",
                "got Array",
                "the default continuation prompt",
            ],
        ),
        (
            "error('kaboom')",
            &["Error", "REPL configuration failed", "kaboom"],
        ),
        ("export def prompt = ", &["repl.frst"]),
        ("export def prompt = nope", &["repl.frst", "nope"]),
    ];
    for (source, fragments) in cases {
        let (_, problems) = load(source);
        let said = texts(&problems).join("\n");
        for fragment in fragments {
            assert!(
                said.contains(fragment),
                "{source:?} says {said:?}, lacking {fragment:?}"
            );
        }
    }
}

#[test]
fn a_failed_prompt_says_so_and_that_the_default_stands_in() {
    let mut config = load_clean(
        r"
        export defn prompt(c) -> error('kaboom')
        export defn continuation_prompt(c) -> 5
        ",
    );
    let said = texts(&config.prompts(&first()).1);
    assert_eq!(said.len(), 2, "{said:?}");
    for fragment in [
        "Error",
        "`prompt` failed",
        "the default prompt is used from now on",
        "kaboom",
    ] {
        assert!(said[0].contains(fragment), "{said:?} lacks {fragment:?}");
    }
    for fragment in [
        "Error",
        "`continuation_prompt` must return a String, got Int",
        "the default continuation prompt is used from now on",
    ] {
        assert!(said[1].contains(fragment), "{said:?} lacks {fragment:?}");
    }
}

// -- Reading the file --

/// A fresh, empty directory for one test to work in, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let path = std::env::temp_dir().join(format!("frost-config-test-{}-{test}", process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).expect("the scratch directory should be created");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_missing_file_sets_nothing_and_is_no_problem() {
    let scratch = Scratch::new("missing");
    let (mut config, problems) = Config::read(&scratch.0.join("repl.frst"), true);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(prompts(&mut config, &first()), Prompts::default());
}

#[test]
fn a_file_is_read_and_run() {
    let scratch = Scratch::new("present");
    let path = scratch.0.join("repl.frst");
    fs::write(&path, "export def prompt = 'file> '\n").unwrap();
    let (mut config, problems) = Config::read(&path, true);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(
        prompts(&mut config, &first()).prompt.as_deref(),
        Some("file> ")
    );
}

#[test]
fn a_file_that_does_not_compile_is_reported_under_its_path() {
    let scratch = Scratch::new("broken");
    let path = scratch.0.join("repl.frst");
    fs::write(&path, "export def prompt = \n").unwrap();
    let (_, problems) = Config::read(&path, true);
    let said = texts(&problems).join("\n");
    assert!(
        matches!(problems.as_slice(), [Problem::Compile(_)]),
        "{problems:?}"
    );
    assert!(said.contains(&path.display().to_string()), "{said}");
}

#[test]
fn a_file_that_cannot_be_read_is_reported() {
    let scratch = Scratch::new("unreadable");
    // A directory, and a file that is not UTF-8.
    let directory = scratch.0.join("directory.frst");
    fs::create_dir(&directory).unwrap();
    let binary = scratch.0.join("binary.frst");
    fs::write(&binary, [0xff, 0xfe, 0x00]).unwrap();
    for path in [directory, binary] {
        let (mut config, problems) = Config::read(&path, true);
        assert!(
            matches!(problems.as_slice(), [Problem::Unreadable { path: p, .. }] if *p == path),
            "{path:?}: {problems:?}"
        );
        let said = texts(&problems).join("\n");
        assert!(
            said.contains("cannot read the REPL configuration")
                && said.contains(&path.display().to_string()),
            "{said}"
        );
        assert_eq!(prompts(&mut config, &first()), Prompts::default());
    }
}
