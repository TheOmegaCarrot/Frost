//! The output globals, from Frost source: `print`, `mformat`, and `mprint`.
//!
//! `print(value)` hands `value`, rendered as `to_string` renders it, to the Vm's
//! print sink, and returns Null.
//!
//! `mformat(format, replacements)` replaces each `${key}` placeholder in
//! `format` with `replacements[key]` as `to_string` renders it. `\$` and `\\`
//! escape; any other backslash, and any `$` not opening a placeholder, is
//! literal. `mprint` prints what `mformat` returns.
//!
//! Most cases write the template as a Frost raw string, `R'(...)'`, so it
//! reaches `mformat` exactly as the Rust test spells it, backslashes included.
//! The harness runs every case under every optimization permutation, so a call
//! over literals is checked both folded and at run time.

mod source;

use frostlang_runtime::Value;
use source::assertions::{Library, library_assertions};
use source::{Script, raises, run};

library_assertions!(Library::GLOBALS);

/// The source of an `mformat` call over `template`, as a raw string, and the
/// Frost expression `replacements`.
fn mformat_source(template: &str, replacements: &str) -> String {
    format!("mformat(R'({template})', {replacements})")
}

/// What `mformat` returns for `template` and `replacements`, which must be a String.
fn formatted(template: &str, replacements: &str) -> String {
    let source = mformat_source(template, replacements);
    match run(&source) {
        Value::String(text) => text.to_string(),
        other => panic!("{source:?} should return a String, but returned {other:?}"),
    }
}

/// The error `mformat` raises for `template` and `replacements`.
fn format_error(template: &str, replacements: &str) -> String {
    raises(&mformat_source(template, replacements))
}

/// Assert each `(template, expected)` formats to `expected` with `replacements`.
fn assert_formats(replacements: &str, cases: &[(&str, &str)]) {
    for (template, expected) in cases {
        assert_eq!(
            formatted(template, replacements),
            *expected,
            "{template:?} with {replacements}"
        );
    }
}

/// Assert each `(template, message)` raises exactly `message` with `replacements`.
fn assert_format_errors(replacements: &str, cases: &[(&str, &str)]) {
    for (template, message) in cases {
        assert_eq!(
            format_error(template, replacements),
            *message,
            "{template:?} with {replacements}"
        );
    }
}

// --- Substitution ---

#[test]
fn placeholders_are_replaced_by_their_values() {
    assert_formats(
        "{a: 'foo', b: 'bar', greeting: 'hello'}",
        &[
            ("${greeting} ${b}!", "hello bar!"),
            // At either boundary, and adjacent.
            ("${a}suffix", "foosuffix"),
            ("prefix${a}", "prefixfoo"),
            ("${a}${b}", "foobar"),
            ("${a}", "foo"),
            // A key may repeat.
            ("${a} ${a}", "foo foo"),
        ],
    );
}

#[test]
fn a_template_without_placeholders_is_unchanged() {
    assert_formats(
        "{k: 'v', [1]: 'ignored'}",
        &[
            ("plain text", "plain text"),
            ("", ""),
            ("caf\u{e9} \u{1f600}", "caf\u{e9} \u{1f600}"),
        ],
    );
    assert_eq!(formatted("plain", "{}"), "plain", "an empty map");
}

#[test]
fn values_render_as_to_string_renders_them() {
    for value in [
        "'text'",
        "''",
        "42",
        "-1.5",
        "true",
        "null",
        "x'6869'",
        "[1, 'two', [3]]",
        "{k: 'v'}",
        "[]",
    ] {
        let replacements = format!("{{v: {value}}}");
        assert_eq!(
            run(&mformat_source("<${v}>", &replacements)),
            run(&format!("'<' + to_string({value}) + '>'")),
            "{value}"
        );
    }
}

#[test]
fn a_substituted_value_is_never_expanded() {
    // A value is inserted as is: placeholders and escapes in it are text.
    assert_formats(
        r"{a: '${b}', b: 'x', c: R'(\${b} \\)', d: '${'}",
        &[
            ("${a}", "${b}"),
            ("${a}${b}", "${b}x"),
            ("${c}", r"\${b} \\"),
            ("${d}", "${"),
        ],
    );
}

#[test]
fn unused_and_non_string_keys_are_ignored() {
    assert_formats(
        "{ok: 'yes', unused: null, [1]: 'int key', [true]: 'bool key'}",
        &[("${ok}", "yes")],
    );
}

#[test]
fn only_a_string_key_satisfies_a_placeholder() {
    // A Bytes key with the same octets as the placeholder's name is a different key.
    assert_eq!(
        format_error("${k}", "{[x'6b']: 'bytes key'}"),
        "Missing replacement for key: k"
    );
}

#[test]
fn a_placeholder_key_is_any_identifier_shaped_name() {
    let long_key = "a".repeat(1024);
    let replacements = format!(
        "{{_: 'u', _x: 'ux', a1_b2: 'ab', CamelCase: 'cc', {long_key}: 'long', \
         ['if']: 'keyword', ['null']: 'null keyword'}}"
    );
    assert_formats(
        &replacements,
        &[
            ("${_} ${_x} ${a1_b2} ${CamelCase}", "u ux ab cc"),
            (&format!("${{{long_key}}}"), "long"),
            // Reserved words are identifier-shaped, so they may name a key.
            ("${if} ${null}", "keyword null keyword"),
        ],
    );
}

#[test]
fn a_placeholder_key_may_be_a_dollar_name() {
    assert_formats(
        "{['$']: 'one', ['$$']: 'rest', ['$0']: 'zero', ['$9']: 'nine'}",
        &[("${$} ${$$} ${$0} ${$9}", "one rest zero nine")],
    );
}

// --- Escapes and literal characters ---

#[test]
fn a_backslash_escapes_a_dollar_or_a_backslash() {
    assert_formats(
        "{k: 'v'}",
        &[
            (r"\${k}", "${k}"),
            (r"\$", "$"),
            (r"\\${k}", r"\v"),
            (r"\\", r"\"),
            (r"\\\${k}", r"\${k}"),
            (r"x\${k}y${k}", "x${k}yv"),
        ],
    );
}

#[test]
fn any_other_backslash_is_literal() {
    assert_formats(
        "{k: 'v'}",
        &[
            (r"\n\t", r"\n\t"),
            (r"a\b", r"a\b"),
            (r"\{k}", r"\{k}"),
            (r"${k}\", r"v\"),
            (r"\", r"\"),
        ],
    );
}

#[test]
fn a_dollar_not_opening_a_placeholder_is_literal() {
    assert_formats(
        "{a: 'x', x: 'unused'}",
        &[
            ("$", "$"),
            ("$$$$$$$", "$$$$$$$"),
            ("$$ and $x and price: $5", "$$ and $x and price: $5"),
            ("$ { } $ { }", "$ { } $ { }"),
            // A `$` does not escape a placeholder that follows it.
            ("$${a}", "$x"),
            ("$ ${a}", "$ x"),
            ("${a}$", "x$"),
        ],
    );
}

#[test]
fn a_brace_outside_a_placeholder_is_literal() {
    assert_formats(
        "{a: 'x'}",
        &[
            ("}", "}"),
            ("}}}", "}}}"),
            ("{a}", "{a}"),
            ("${a}}", "x}"),
            ("${a}{", "x{"),
            ("}${a}", "}x"),
        ],
    );
}

// --- Errors ---

#[test]
fn a_missing_key_is_an_error() {
    assert_format_errors(
        "{k1: 'hello'}",
        &[
            ("${k1} ${k2}", "Missing replacement for key: k2"),
            ("${K1}", "Missing replacement for key: K1"),
        ],
    );
    assert_eq!(
        format_error("${k}", "{}"),
        "Missing replacement for key: k",
        "an empty map"
    );
}

#[test]
fn an_unterminated_placeholder_is_an_error() {
    // The message quotes the template from the unterminated placeholder on.
    assert_format_errors(
        "{a: 'x', k1: 'y'}",
        &[
            ("${", "Unterminated format placeholder: ${"),
            ("${k1", "Unterminated format placeholder: ${k1"),
            ("${a}${a}${", "Unterminated format placeholder: ${"),
            ("x${a}y${a b", "Unterminated format placeholder: ${a b"),
        ],
    );
}

#[test]
fn a_placeholder_key_that_is_not_a_name_is_an_error() {
    assert_format_errors(
        "{a: 'x', ['1abc']: 'y', ['a-b']: 'z'}",
        &[
            ("${}", "Invalid format placeholder: ${}"),
            ("${1abc}", "Invalid format placeholder: ${1abc}"),
            ("${a-b}", "Invalid format placeholder: ${a-b}"),
            ("${a.b}", "Invalid format placeholder: ${a.b}"),
            ("${ a}", "Invalid format placeholder: ${ a}"),
            ("${a }", "Invalid format placeholder: ${a }"),
            ("${a\t}", "Invalid format placeholder: ${a\t}"),
            ("${na\u{ef}ve}", "Invalid format placeholder: ${na\u{ef}ve}"),
            ("${\u{1f600}}", "Invalid format placeholder: ${\u{1f600}}"),
            // Dollar names are only `$`, `$$`, and `$` with one digit.
            ("${$10}", "Invalid format placeholder: ${$10}"),
            ("${$a}", "Invalid format placeholder: ${$a}"),
            ("${$$$}", "Invalid format placeholder: ${$$$}"),
            // A placeholder runs to the first `}`, so placeholders do not nest.
            ("${${a}}", "Invalid format placeholder: ${${a}"),
            ("${x${a}y}", "Invalid format placeholder: ${x${a}"),
        ],
    );
}

#[test]
fn the_whole_template_is_checked_before_any_key_is_looked_up() {
    // The missing key comes first, but the malformed placeholder is reported.
    assert_eq!(
        format_error("${missing} ${1bad}", "{}"),
        "Invalid format placeholder: ${1bad}"
    );
    assert_eq!(
        format_error("${missing} ${", "{}"),
        "Unterminated format placeholder: ${"
    );
}

#[test]
fn mformat_formats_runtime_values() {
    let formatted = Script::new("mformat(t, r)")
        .capture("t", Value::from(r"\${name} is ${name}"))
        .capture("r", Value::map([("name", Value::from("frost"))]))
        .run();
    assert_eq!(formatted, Value::from("${name} is frost"));
}

#[test]
fn mformat_checks_its_argument_types() {
    let message =
        |requires: &str, ty: &str| format!("Function mformat requires {requires}, got {ty}");
    for (source, requires, ty) in [
        (
            "mformat(42, {})",
            "String as argument 1 (format string)",
            "Int",
        ),
        (
            "mformat(x'6869', {})",
            "String as argument 1 (format string)",
            "Bytes",
        ),
        (
            "mformat(null, {})",
            "String as argument 1 (format string)",
            "Null",
        ),
        (
            "mformat('${k}', 'nope')",
            "Map as argument 2 (replacement map)",
            "String",
        ),
        (
            "mformat('${k}', [1])",
            "Map as argument 2 (replacement map)",
            "Array",
        ),
        (
            "mformat('${k}', null)",
            "Map as argument 2 (replacement map)",
            "Null",
        ),
    ] {
        assert_eq!(raises(source), message(requires, ty), "{source:?}");
    }
}

#[test]
fn mformat_takes_exactly_two_arguments() {
    for (source, argc) in [
        ("mformat()", 0),
        ("mformat('x')", 1),
        ("mformat('x', {}, {})", 3),
    ] {
        assert_eq!(
            raises(source),
            format!("Function mformat expects 2 arguments, but was called with {argc}"),
            "{source:?}"
        );
    }
}

// --- print ---

#[test]
fn a_string_prints_as_its_text() {
    assert_eq!(printed(r#"print("hello")"#), ["hello"]);
    assert_eq!(printed(r#"print("")"#), [""]);
    assert_eq!(printed(r#"print("héllo, 世界")"#), ["héllo, 世界"]);
}

#[test]
fn any_value_prints_as_to_string_renders_it() {
    for value in [
        "null",
        "true",
        "-5",
        "1.5",
        "x'00ff'",
        r#"[1, "a", [null]]"#,
        r#"{a: "b"}"#,
        "plus",
        "fn -> 1",
    ] {
        let rendered = run(&format!("to_string({value})")).to_frost_string();
        assert_eq!(printed(&format!("print({value})")), [rendered], "{value}");
    }
}

#[test]
fn print_returns_null() {
    assert_eq!(run("print(1)"), Value::Null);
    assert_eq!(run("[print(1), print(2)]"), run("[null, null]"));
}

#[test]
fn each_print_is_one_text_without_a_line_terminator() {
    // A newline inside the text is passed through; none is added.
    let two_lines = r"b
c";
    let source = r#"
        print("a")
        print("b\nc")
    "#;
    assert_eq!(printed(source), ["a", two_lines]);
}

#[test]
fn print_takes_exactly_one_argument() {
    assert_arity("print", 1, &[0, 2]);
    // An arity error prints nothing.
    assert_eq!(Script::new("print(1, 2)").printed(), Vec::<String>::new());
}

// --- mprint ---

#[test]
fn mprint_prints_what_mformat_returns() {
    assert_eq!(
        printed("mprint('${a} and ${b}', {a: 1, b: 'two'})"),
        ["1 and two"]
    );
    assert_eq!(printed(r"mprint(R'(\${a})', {a: 1})"), ["${a}"]);
    assert_eq!(printed("mprint('', {})"), [""]);
    // A substituted value is never expanded.
    assert_eq!(printed("mprint('${a}', {a: '${b}', b: 'x'})"), ["${b}"]);
}

#[test]
fn mprint_returns_null() {
    assert_eq!(run("mprint('x', {})"), Value::Null);
}

#[test]
fn mprint_prints_each_call_in_order() {
    let source = r"
        mprint('${n}', {n: 1})
        mprint('${n}', {n: 2})
    ";
    assert_eq!(printed(source), ["1", "2"]);
}

#[test]
fn mprint_prints_nothing_when_formatting_fails() {
    for (source, message) in [
        ("mprint('${k}', {})", "Missing replacement for key: k"),
        ("mprint('${', {})", "Unterminated format placeholder: ${"),
        ("mprint('${1}', {})", "Invalid format placeholder: ${1}"),
    ] {
        let script = Script::new(source);
        assert_eq!(script.raises(), message, "{source:?}");
        assert!(script.printed().is_empty(), "{source:?} prints nothing");
    }
}

#[test]
fn mprint_checks_its_argument_types() {
    let message =
        |requires: &str, ty: &str| format!("Function mprint requires {requires}, got {ty}");
    for (source, requires, ty) in [
        (
            "mprint(42, {})",
            "String as argument 1 (format string)",
            "Int",
        ),
        (
            "mprint(x'6869', {})",
            "String as argument 1 (format string)",
            "Bytes",
        ),
        (
            "mprint('${k}', 'nope')",
            "Map as argument 2 (replacement map)",
            "String",
        ),
        (
            "mprint('${k}', null)",
            "Map as argument 2 (replacement map)",
            "Null",
        ),
    ] {
        let script = Script::new(source);
        assert_eq!(script.raises(), message(requires, ty), "{source:?}");
        assert!(script.printed().is_empty(), "{source:?} prints nothing");
    }
}

#[test]
fn mprint_takes_exactly_two_arguments() {
    for (source, argc) in [
        ("mprint()", 0),
        ("mprint('x')", 1),
        ("mprint('x', {}, {})", 3),
    ] {
        assert_eq!(
            raises(source),
            format!("Function mprint expects 2 arguments, but was called with {argc}"),
            "{source:?}"
        );
    }
}
