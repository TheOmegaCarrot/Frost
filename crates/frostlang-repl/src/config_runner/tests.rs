//! White-box: `ConfigRunner` is crate-internal. Covers the exports a script
//! gives, the `repl` component, the import allowlist, how each kind of failure
//! is told apart, and running a function the script returned.

use frostlang::Value;

use crate::config_runner::{ConfigRunner, ScriptFailure};

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

/// The exports of `source`, which must run to its end.
#[track_caller]
fn exports(runner: &mut ConfigRunner, source: &str) -> Vec<(String, Value)> {
    match runner.run_script("test.frst", source) {
        Ok(exports) => exports,
        Err(failure) => panic!("{source:?} failed: {failure:?}"),
    }
}

/// The export `name` of `source`, which must run to its end and export it.
#[track_caller]
fn export(runner: &mut ConfigRunner, source: &str, name: &str) -> Value {
    exports(runner, source)
        .into_iter()
        .find_map(|(key, value)| (key == name).then_some(value))
        .unwrap_or_else(|| panic!("{source:?} does not export {name:?}"))
}

/// The failure of `source`, which must not run to its end.
#[track_caller]
fn failure(runner: &mut ConfigRunner, source: &str) -> ScriptFailure {
    match runner.run_script("test.frst", source) {
        Ok(exports) => panic!("{source:?} should fail, exported {exports:?}"),
        Err(failure) => failure,
    }
}

/// What `call`, an expression with `repl` bound, computes as a String, with
/// color on or off.
#[track_caller]
fn painted(call: &str, color: bool) -> String {
    let source = format!("def repl = import('repl')\nexport def out = {call}");
    let value = export(&mut ConfigRunner::new(color), &source, "out");
    value
        .as_str()
        .unwrap_or_else(|| panic!("{call} gave {value:?}, not a String"))
        .to_owned()
}

/// The message of the run failure of the expression `call`, with `repl` bound.
#[track_caller]
fn painter_error(call: &str) -> String {
    let source = format!("def repl = import('repl')\nexport def out = {call}");
    match failure(&mut ConfigRunner::new(true), &source) {
        ScriptFailure::Run(error) => error.to_string(),
        other => panic!("{call}: expected a run failure, got {other:?}"),
    }
}

// -- Exports --

#[test]
fn a_script_gives_exactly_its_exports() {
    let mut runner = ConfigRunner::new(false);
    let mut given = exports(
        &mut runner,
        r"
        def hidden = 1
        export def b = 2
        export def a = 'one'
        export defn c(x) -> x
        ",
    );
    given.sort_by(|left, right| left.0.cmp(&right.0));
    let names: Vec<&str> = given.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["a", "b", "c"], "plain defs are not exports");
    assert_eq!(given[0].1.as_str(), Some("one"));
    assert_eq!(given[1].1.as_int(), Some(2));
    assert!(given[2].1.is_function(), "{:?}", given[2].1);
}

#[test]
fn a_script_without_exports_gives_none() {
    let mut runner = ConfigRunner::new(false);
    for source in ["", "# nothing here", "def x = 1", "1 + 2"] {
        let given = exports(&mut runner, source);
        assert!(given.is_empty(), "{source:?}: {given:?}");
    }
}

// -- The `repl` component --

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

#[test]
fn an_outer_painter_resumes_after_an_inner_one_ends() {
    // An inner painter's off code would end the outer painter's paint too,
    // so the outer painter's on code is sent again after it. Bold and dim
    // share an off code, so each resumes after the other.
    let cases = [
        (
            "repl.colors.red('a' + repl.colors.blue('b') + 'c')",
            "\x1b[31ma\x1b[34mb\x1b[39m\x1b[31mc\x1b[39m",
        ),
        (
            "repl.style.dim('a' + repl.style.bold('b') + 'c')",
            "\x1b[2ma\x1b[1mb\x1b[22m\x1b[2mc\x1b[22m",
        ),
        (
            "repl.style.bold('a' + repl.style.dim('b') + 'c')",
            "\x1b[1ma\x1b[2mb\x1b[22m\x1b[1mc\x1b[22m",
        ),
        (
            "repl.style.italic('a' + repl.style.italic('b') + 'c')",
            "\x1b[3ma\x1b[3mb\x1b[23m\x1b[3mc\x1b[23m",
        ),
        (
            "repl.colors.red(repl.colors.blue('b') + repl.colors.green('g'))",
            "\x1b[31m\x1b[34mb\x1b[39m\x1b[31m\x1b[32mg\x1b[39m\x1b[31m\x1b[39m",
        ),
        (
            "repl.colors.red('a' + repl.colors.blue('b' + repl.colors.green('g') + 'b') + 'a')",
            "\x1b[31ma\x1b[34mb\x1b[32mg\x1b[39m\x1b[31m\x1b[34mb\x1b[39m\x1b[31ma\x1b[39m",
        ),
    ];
    for (call, expected) in cases {
        assert_eq!(painted(call, true), expected, "{call}");
    }
}

/// A painter resumes only after codes that would end its own paint: a color
/// is untouched by an inner style, and a style by an inner color.
#[test]
fn an_outer_painter_ignores_inner_codes_that_leave_it_on() {
    let cases = [
        (
            "repl.colors.red('a' + repl.style.bold('b') + 'c')",
            "\x1b[31ma\x1b[1mb\x1b[22mc\x1b[39m",
        ),
        (
            "repl.style.bold('a' + repl.colors.red('b') + 'c')",
            "\x1b[1ma\x1b[31mb\x1b[39mc\x1b[22m",
        ),
        (
            "repl.style.bold('a' + repl.style.italic('b') + 'c')",
            "\x1b[1ma\x1b[3mb\x1b[23mc\x1b[22m",
        ),
        (
            "repl.style.underline('a' + repl.style.italic('b') + 'c')",
            "\x1b[4ma\x1b[3mb\x1b[23mc\x1b[24m",
        ),
    ];
    for (call, expected) in cases {
        assert_eq!(painted(call, true), expected, "{call}");
    }
}

#[test]
fn a_painter_takes_exactly_one_argument() {
    let painters = COLORS
        .iter()
        .map(|(name, _)| format!("repl.colors.{name}"))
        .chain(
            STYLES
                .iter()
                .map(|(name, _, _)| format!("repl.style.{name}")),
        );
    for painter in painters {
        for call in [format!("{painter}()"), format!("{painter}('a', 'b')")] {
            let error = painter_error(&call);
            assert!(
                error.contains("argument") && error.contains(&painter),
                "{call}: {error}"
            );
        }
    }
}

#[test]
fn a_painter_takes_only_a_string() {
    let painters: Vec<String> = COLORS
        .iter()
        .map(|(name, _)| format!("repl.colors.{name}"))
        .chain(
            STYLES
                .iter()
                .map(|(name, _, _)| format!("repl.style.{name}")),
        )
        .collect();
    for painter in &painters {
        for (argument, type_name) in [
            ("5", "Int"),
            ("1.5", "Float"),
            ("true", "Bool"),
            ("null", "Null"),
            ("['a']", "Array"),
            ("{a: 1}", "Map"),
            ("x'00'", "Bytes"),
            ("fn -> 1", "Function"),
        ] {
            let call = format!("{painter}({argument})");
            let error = painter_error(&call);
            assert!(
                error.contains(painter.as_str())
                    && error.contains("String")
                    && error.contains(type_name),
                "{call}: {error}"
            );
        }
    }
}

// -- Imports --

#[test]
fn the_configuration_may_import_only_its_modules() {
    let mut runner = ConfigRunner::new(true);
    for module in ["std.string", "std.math", "repl"] {
        exports(&mut runner, &format!("def m = import('{module}')"));
    }
    for module in [
        "std.os",
        "std.fs",
        "std.random",
        "std.regex",
        "std.encoding",
        "ext.anything",
        "nonexistent",
    ] {
        let source = format!("def m = import('{module}')");
        assert!(
            matches!(failure(&mut runner, &source), ScriptFailure::Run(_)),
            "importing {module}"
        );
    }
}

// -- Failures --

#[test]
fn a_script_that_does_not_compile_is_a_compile_failure_naming_the_file() {
    let mut runner = ConfigRunner::new(false);
    for source in [
        "export def prompt = ",
        "export def prompt = '> '\ndef = 1",
        "export def prompt = unbound_name",
    ] {
        match runner.run_script("my_config.frst", source) {
            Err(ScriptFailure::Compile(diagnostics)) => {
                let said = diagnostics.to_string();
                assert!(said.contains("my_config.frst"), "{source:?}: {said}");
            }
            other => panic!("{source:?}: expected a compile failure, got {other:?}"),
        }
    }
}

#[test]
fn a_script_that_fails_while_running_is_a_run_failure() {
    let mut runner = ConfigRunner::new(false);
    for source in [
        "error('broken')",
        "export def x = 1 / 0",
        "def x = to_upper(5)",
    ] {
        assert!(
            matches!(failure(&mut runner, source), ScriptFailure::Run(_)),
            "{source:?}"
        );
    }
}

#[test]
fn a_script_that_raises_says_why() {
    let mut runner = ConfigRunner::new(false);
    match failure(&mut runner, "error('kaboom')") {
        ScriptFailure::Run(error) => {
            assert!(error.to_string().contains("kaboom"), "{error}");
        }
        other => panic!("expected a run failure, got {other:?}"),
    }
}

#[test]
fn a_script_that_runs_away_is_a_run_failure() {
    let mut runner = ConfigRunner::new(false);
    // Fuel runs out.
    let forever = "defn forever(n) -> forever(n + 1)\nexport def x = forever(0)";
    // The call depth runs out.
    let deep = "defn deep(n) -> 1 + deep(n)\nexport def x = to_string(deep(0))";
    // Fuel runs out in a loop of folds.
    let long = "export def x = reduce(range(1000000), 0, fn acc, x -> acc + x)";
    for source in [forever, deep, long] {
        assert!(
            matches!(failure(&mut runner, source), ScriptFailure::Run(_)),
            "{source:?}"
        );
    }
}

#[test]
fn the_runner_stays_usable_after_each_kind_of_failure() {
    let mut runner = ConfigRunner::new(false);
    for source in [
        "export def prompt = ",
        "error('broken')",
        "defn forever(n) -> forever(n + 1)\nexport def x = forever(0)",
        "defn deep(n) -> 1 + deep(n)\nexport def x = to_string(deep(0))",
        "def m = import('std.os')",
    ] {
        let _ = failure(&mut runner, source);
        let value = export(&mut runner, "export def ok = 'fine'", "ok");
        assert_eq!(value.as_str(), Some("fine"), "after {source:?}");
    }
}

#[test]
fn each_script_has_its_own_fuel() {
    // Each run spends a fair share of the limit; together, many times over it.
    let mut runner = ConfigRunner::new(false);
    let source = "export def x = reduce(range(2000), 0, fn acc, x -> acc + x)";
    for _ in 0..200 {
        assert_eq!(export(&mut runner, source, "x").as_int(), Some(1_999_000));
    }
}

// -- Wrapping and calling --

#[cfg(feature = "line-editor")]
mod calling {
    use frostlang::Value;

    use super::{COLORS, export, failure};
    use crate::config_runner::{ConfigRunner, ScriptFailure};

    #[test]
    fn a_wrapped_native_behaves_like_the_native() {
        let mut runner = ConfigRunner::new(false);
        let native = export(&mut runner, "export def f = to_upper", "f");
        let wrapper = runner.wrap(native).expect("a native wraps");
        let result = runner.call(wrapper, Value::from("shout")).expect("a call");
        assert_eq!(result.as_str(), Some("SHOUT"));
    }

    #[test]
    fn a_wrapped_native_reports_its_own_errors() {
        let mut runner = ConfigRunner::new(false);
        let native = export(&mut runner, "export def f = to_upper", "f");
        let wrapper = runner.wrap(native).expect("a native wraps");
        let error = runner
            .call(wrapper.clone(), Value::from(5))
            .expect_err("`to_upper` takes a String");
        assert!(error.to_string().contains("String"), "{error}");
        // The wrapper still works afterwards.
        let result = runner.call(wrapper, Value::from("ok")).expect("a call");
        assert_eq!(result.as_str(), Some("OK"));
    }

    #[test]
    fn a_script_function_is_called_with_its_argument() {
        let mut runner = ConfigRunner::new(false);
        let Value::Closure(function) = export(&mut runner, "export defn f(x) -> $'<${x}>'", "f")
        else {
            panic!("an exported function is a closure");
        };
        let result = runner.call(function, Value::from("a")).expect("a call");
        assert_eq!(result.as_str(), Some("<a>"));
    }

    #[test]
    fn each_call_has_its_own_fuel() {
        // Each call spends a fair share of the limit; together, many times over it.
        let mut runner = ConfigRunner::new(false);
        let Value::Closure(function) = export(
            &mut runner,
            "export defn f(c) -> reduce(range(2000), 0, fn acc, x -> acc + x)",
            "f",
        ) else {
            panic!("an exported function is a closure");
        };
        for _ in 0..200 {
            let result = runner
                .call(function.clone(), Value::Null)
                .expect("a call has fuel");
            assert_eq!(result.as_int(), Some(1_999_000));
        }
    }

    #[test]
    fn a_call_that_raises_is_an_error_and_leaves_the_runner_usable() {
        let mut runner = ConfigRunner::new(false);
        let Value::Closure(function) = export(
            &mut runner,
            "export defn f(x) -> if x: error('kaboom') else: 'fine'",
            "f",
        ) else {
            panic!("an exported function is a closure");
        };
        let error = runner
            .call(function.clone(), Value::from(true))
            .expect_err("the call raises");
        assert!(error.to_string().contains("kaboom"), "{error}");
        let result = runner
            .call(function, Value::from(false))
            .expect("the next call is fine");
        assert_eq!(result.as_str(), Some("fine"));
    }

    #[test]
    fn a_call_that_runs_away_is_an_error() {
        let mut runner = ConfigRunner::new(false);
        let Value::Closure(forever) = export(
            &mut runner,
            "defn spin(n) -> spin(n + 1)\nexport defn f(c) -> spin(0)",
            "f",
        ) else {
            panic!("an exported function is a closure");
        };
        assert!(runner.call(forever, Value::Null).is_err());
        let Value::Closure(deep) = export(
            &mut runner,
            "defn down(n) -> 1 + down(n)\nexport defn f(c) -> down(0)",
            "f",
        ) else {
            panic!("an exported function is a closure");
        };
        assert!(runner.call(deep, Value::Null).is_err());
    }

    #[test]
    fn a_function_may_be_called_on_another_runner() {
        // The terminal frontend calls the functions the Repl's load gave it.
        let mut loader = ConfigRunner::new(false);
        let Value::Closure(function) = export(
            &mut loader,
            r"
            def math = import('std.math')
            export defn f(x) -> math.abs(x) + 1
            ",
            "f",
        ) else {
            panic!("an exported function is a closure");
        };
        let mut caller = ConfigRunner::new(false);
        let result = caller
            .call(function, Value::from(-4))
            .expect("closures are portable");
        assert_eq!(result.as_int(), Some(5));
    }

    #[test]
    fn a_painter_keeps_the_color_it_was_built_with() {
        // The color is fixed when the component is built, so a painter
        // taken from a colored runner still colors on an uncolored one.
        let mut colored = ConfigRunner::new(true);
        let Value::Closure(red) = export(
            &mut colored,
            "def repl = import('repl')\nexport def f = fn x -> repl.colors.red(x)",
            "f",
        ) else {
            panic!("an exported function is a closure");
        };
        let mut plain = ConfigRunner::new(false);
        let result = plain
            .call(red, Value::from("hi"))
            .expect("a painter is portable");
        assert_eq!(
            result.as_str(),
            Some(format!("\x1b[{}mhi\x1b[39m", COLORS[1].1).as_str())
        );
    }

    #[test]
    fn a_wrapper_made_on_one_runner_may_be_called_on_another() {
        let mut first = ConfigRunner::new(false);
        let native = export(&mut first, "export def f = to_upper", "f");
        let wrapper = first.wrap(native).expect("a native wraps");
        let mut second = ConfigRunner::new(false);
        let result = second.call(wrapper, Value::from("up")).expect("a call");
        assert_eq!(result.as_str(), Some("UP"));
    }

    #[test]
    fn a_failed_script_leaves_calls_working() {
        let mut runner = ConfigRunner::new(false);
        let native = export(&mut runner, "export def f = to_upper", "f");
        let wrapper = runner.wrap(native).expect("a native wraps");
        assert!(matches!(
            failure(&mut runner, "error('broken')"),
            ScriptFailure::Run(_)
        ));
        let result = runner.call(wrapper, Value::from("still")).expect("a call");
        assert_eq!(result.as_str(), Some("STILL"));
    }
}
