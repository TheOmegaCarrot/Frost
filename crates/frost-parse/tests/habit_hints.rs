//! Help for errors that look like habits carried over from other languages.
//! A hint only decorates an error the parser raises anyway: each case here fails to
//! parse with or without its hint.

use frost_parse::parse_program;

/// The message and help of the error `src` fails with.
fn diagnosis(src: &str) -> (String, Option<String>) {
    let err = parse_program("test.frst", src).expect_err(src);
    (err.message().to_owned(), err.help().map(str::to_owned))
}

/// Asserts each source fails with exactly the paired help.
fn assert_help(cases: &[(&str, &str)]) {
    for &(source, help) in cases {
        let (message, actual) = diagnosis(source);
        assert_eq!(
            actual.as_deref(),
            Some(help),
            "{source:?} failed with {message:?}"
        );
    }
}

// -- Binding and statements --

#[test]
fn declaration_keywords() {
    assert_help(&[
        (
            "let x = 5",
            "Frost has no `let`; bind a name with `def`: `def x = ...`",
        ),
        (
            "var count = 0",
            "Frost has no `var`; bind a name with `def`: `def count = ...`",
        ),
        (
            "const name = 'frost'",
            "Frost has no `const`; bind a name with `def`: `def name = ...`",
        ),
        (
            "local x = 1",
            "Frost has no `local`; bind a name with `def`: `def x = ...`",
        ),
        (
            "val x = 1",
            "Frost has no `val`; bind a name with `def`: `def x = ...`",
        ),
        (
            "mut x = 1",
            "Frost has no `mut`; bind a name with `def`: `def x = ...`",
        ),
    ]);
}

#[test]
fn assignment() {
    let in_a_block = r"
        do {
            x = 1
            x
        }
    ";
    assert_help(&[
        (
            "x = 42",
            "Frost has no assignment; bind a new name with `def`: `def x = ...`",
        ),
        (
            in_a_block,
            "Frost has no assignment; bind a new name with `def`: `def x = ...`",
        ),
        (
            "xs[0] = 1",
            "Frost values are immutable; bind an updated value to a new name with `def`",
        ),
        (
            "m.k = 1",
            "Frost values are immutable; bind an updated value to a new name with `def`",
        ),
    ]);
}

#[test]
fn compound_assignment() {
    assert_help(&[
        (
            "x += 1",
            "Frost has no `+=`; bind the result to a new name with `def`",
        ),
        (
            "x -= 1",
            "Frost has no `-=`; bind the result to a new name with `def`",
        ),
        (
            "x *= 2",
            "Frost has no `*=`; bind the result to a new name with `def`",
        ),
        (
            "x /= 2",
            "Frost has no `/=`; bind the result to a new name with `def`",
        ),
        (
            "x %= 2",
            "Frost has no `%=`; bind the result to a new name with `def`",
        ),
    ]);
}

// `=` inside a bracket is not assignment, and the advice follows the bracket.
#[test]
fn equals_inside_brackets() {
    assert_help(&[
        ("{a = 1}", "a Map entry is written `key: value`"),
        (
            "f(a = 1)",
            "Frost has no named arguments; pass arguments in order",
        ),
        ("fn (x = 1) -> x", "Frost parameters have no default values"),
        (
            "fn f(x = 1) -> x",
            "Frost parameters have no default values",
        ),
        (
            "defn f(x = 1) -> x",
            "Frost parameters have no default values",
        ),
    ]);
}

#[test]
fn return_statement() {
    assert_help(&[(
        "return 5",
        "Frost has no `return`; a function's value is its last expression",
    )]);
}

#[test]
fn function_keywords() {
    let function = r"
        function add(a, b) {
            a + b
        }
    ";
    assert_help(&[
        (
            function,
            "Frost has no `function`; define a function with `defn add(...) -> ...`",
        ),
        (
            "func f(x) {}",
            "Frost has no `func`; define a function with `defn f(...) -> ...`",
        ),
        (
            "fun f(x) {}",
            "Frost has no `fun`; define a function with `defn f(...) -> ...`",
        ),
    ]);
}

#[test]
fn loops() {
    assert_help(&[
        (
            "for x in xs: print(x)",
            "Frost has no `for` loop; use `map`, `filter`, `reduce`, or `foreach` with a \
             function, like `foreach xs with fn x -> ...`",
        ),
        (
            "while running { step() }",
            "Frost has no `while` loop; use `map`, `filter`, `reduce`, or `foreach` with a \
             function, like `foreach xs with fn x -> ...`",
        ),
    ]);
}

// -- Operators --

#[test]
fn logical_operators() {
    assert_help(&[
        ("a && b", "Frost's \"and\" is `and`"),
        ("a || b", "Frost's \"or\" is `or`"),
        ("!a", "Frost's \"not\" is `not`"),
    ]);
}

#[test]
fn other_operators() {
    assert_help(&[
        (
            "[1] |> f()",
            "Frost threads a value into a call with `@`: `x @ f()`",
        ),
        (
            "a ?? b",
            "for a fallback when a value is null, use `or`: `a or b`",
        ),
        (
            "a ? b : c",
            "Frost's conditional is written `if a: b else: c`",
        ),
        (
            "b if a else c",
            "Frost's conditional is written `if a: b else: c`",
        ),
    ]);
}

// -- Control flow --

#[test]
fn else_if() {
    let source = r"
        if a: 1
        else if b: 2
    ";
    assert_help(&[(source, "use `elif` for another condition")]);
}

#[test]
fn a_condition_without_its_colon() {
    let if_help = "`if` takes a colon after its condition: `if x: ...`";
    let on_its_own_line = r"
        if x
            1
    ";
    assert_help(&[
        ("if x { 1 }", if_help),
        ("if (x) { 1 }", if_help),
        ("if x then 1 else 2", if_help),
        (on_its_own_line, if_help),
        (
            "if a: 1 elif b { 2 }",
            "`elif` takes a colon after its condition: `elif x: ...`",
        ),
        (
            "if a: 1 else { 2 }",
            "`else` is followed by a colon: `else: ...`",
        ),
    ]);
}

// -- Match --

#[test]
fn match_arms_from_other_languages() {
    let arm_help = "a `match` arm is written `pattern => result`";
    assert_help(&[
        ("match x { 1 -> 2 }", arm_help),
        ("match x { 1: 2 }", arm_help),
        (
            "match x { case 1: 2 }",
            "Frost's `match` has no `case`; an arm is written `pattern => result`",
        ),
    ]);
}

#[test]
fn guards_from_other_languages() {
    let guard_help = "a guard is written `if:` before its condition, like `n if: n > 0 => ...`";
    assert_help(&[
        ("match x { n if n > 0 => n }", guard_help),
        ("match x { n when n > 0 => n }", guard_help),
    ]);
}

#[test]
fn as_outside_a_map_pattern() {
    let as_help = "`as` binds a whole Map pattern, like `{name} as person`";
    assert_help(&[
        ("match x { [a, b] as m => m }", as_help),
        ("match x { n as m => m }", as_help),
        ("def [a, b] as m = xs", as_help),
        (
            "match m { {a as b} => b }",
            "rename a Map entry with `key: name`, like `{a: b}`",
        ),
        (
            "def {a as b} = m",
            "rename a Map entry with `key: name`, like `{a: b}`",
        ),
    ]);
}

// -- Functions --

#[test]
fn a_lambda_without_its_arrow() {
    let arrow_help = "add `->` before the body: `-> { ... }`";
    assert_help(&[
        ("fn x { x }", arrow_help),
        ("defn f(x) { x }", arrow_help),
        ("fn x => x", "a lambda's arrow is `->`: `fn x -> ...`"),
    ]);
}

#[test]
fn type_annotations() {
    let help = "Frost parameters have no type annotations";
    assert_help(&[
        ("fn x: Int -> x", help),
        ("fn (x: Int) -> x", help),
        ("defn f(x: Int) -> x", help),
    ]);
}

#[test]
fn iterative_expressions_written_as_calls() {
    let map_help =
        "`map` is an expression, written `map xs with f`; its function form is `transform(xs, f)`";
    let threaded_mid_file = r"
        def a = xs @ map(f)
        print(a)
    ";
    assert_help(&[
        ("map(xs, f)", map_help),
        ("xs @ map(f)", map_help),
        (threaded_mid_file, map_help),
        (
            "filter(xs, f)",
            "`filter` is an expression, written `filter xs with f`; its function form is \
             `select(xs, f)`",
        ),
        (
            "xs @ reduce(f)",
            "`reduce` is an expression, written `reduce xs with f`; its function form is \
             `fold(xs, f)`",
        ),
        (
            "foreach xs",
            "`foreach` is an expression, written `foreach xs with f`; its function form is \
             `each(xs, f)`",
        ),
    ]);
}

// -- Functions from other languages --

#[test]
fn lambdas_from_other_languages() {
    let help = "a lambda is written `fn x -> ...`";
    assert_help(&[
        ("def f = x -> x + 1", help),
        ("def f = x => x + 1", help),
        ("def f = (x) => x + 1", help),
        ("def f = \\x -> x + 1", help),
        ("|x| x + 1", help),
        ("|x, y| x + y", help),
        ("lambda x: x + 1", help),
    ]);
}

// -- Strings --

#[test]
fn format_strings_from_other_languages() {
    let help = "a format String is written `$'...${x}...'`";
    assert_help(&[
        ("`hi ${x}`", help),
        ("f\"hi {x}\"", help),
        ("f'hi {x}'", help),
    ]);
}

#[test]
fn curly_quotes() {
    assert_help(&[(
        "\u{201c}hi\u{201d}",
        "Frost Strings use straight quotes, `'` or `\"`",
    )]);
}

#[test]
fn backslash_line_continuation() {
    let source = r"
        def x = 1 + \
            2
    ";
    assert_help(&[(
        source,
        "a line continues only when the next line starts with `.` or `@`; otherwise, \
         wrap the expression in parentheses",
    )]);
}

// -- Source the lexer cannot read --
// These are not habits, but the lexer's error says what it found.

#[test]
fn unreadable_source() {
    let cases = [
        (
            "'abc",
            "unclosed String",
            Some("a String ends with `'` on the same line"),
        ),
        (
            "\"abc",
            "unclosed String",
            Some("a String ends with `\"` on the same line"),
        ),
        (
            "$'abc ${x",
            "unclosed format String",
            Some("a format String ends with `'` on the same line"),
        ),
        (
            "R'(abc",
            "unclosed raw String",
            Some("a raw String ends with `)'` on the same line"),
        ),
        (
            "x'zz'",
            "invalid Bytes literal",
            Some("a Bytes literal holds pairs of hex digits, like `x'00ff'`"),
        ),
        (
            "x'686'",
            "invalid Bytes literal",
            Some("a Bytes literal holds pairs of hex digits, like `x'00ff'`"),
        ),
        ("1 ~ 2", "unexpected character `~`", None),
        (
            "def caf\u{e9} = 1",
            "unexpected character `\u{e9}` (U+00E9)",
            None,
        ),
        ("1\u{a0}+ 2", "unexpected character U+00A0", None),
        ("\u{feff}1", "unexpected character U+FEFF", None),
    ];
    for (source, message, help) in cases {
        let (actual_message, actual_help) = diagnosis(source);
        assert_eq!(actual_message, message, "{source:?}");
        assert_eq!(actual_help.as_deref(), help, "{source:?}");
    }
}

// -- No hint --
// Errors that match no habit get no help.

#[test]
fn ordinary_errors_have_no_help() {
    let sources = [
        "[1 2]",
        "1 +",
        "1 | 2",
        "def x 1",
        "f(1 2)",
        "{a: 1 b: 2}",
        "match x { 1 => 2 3 => 4 }",
    ];
    for source in sources {
        let (message, help) = diagnosis(source);
        assert_eq!(help, None, "{source:?} failed with {message:?}");
    }
}
