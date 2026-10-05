//! Help for errors that look like habits carried over from other languages.
//! A hint only decorates an error the parser raises anyway: each case here fails to
//! parse with or without its hint.

use frostlang_parse::parse_program;

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

/// Asserts each source fails with exactly the paired help, or with none.
fn assert_help_or_none(cases: &[(&str, Option<&str>)]) {
    for &(source, help) in cases {
        let (message, actual) = diagnosis(source);
        assert_eq!(
            actual.as_deref(),
            help,
            "{source:?} failed with {message:?}"
        );
    }
}

const BLOCK_HELP: &str = "`{` starts a Map here; for a block of statements, use `do { ... }`";

const MAP_ENTRY_HELP: &str = "a Map entry is written `key: value`";

const LAMBDA_HELP: &str = "a lambda is written `fn x -> ...`";

const RANGE_HELP: &str =
    "Frost has no range operator; for a range of Ints, use `range(start, stop)`";

const LOOP_HELP_FOR: &str = "Frost has no `for` loop; use `map`, `filter`, `reduce`, or \
                             `foreach` with a function, like `foreach xs with fn x -> ...`";

const LOOP_HELP_WHILE: &str = "Frost has no `while` loop; use `map`, `filter`, `reduce`, or \
                               `foreach` with a function, like `foreach xs with fn x -> ...`";

const PLACEHOLDER_HELP: &str = "`$` placeholders work only inside `$( ... )`, like `$($ * 2)`";

const COMMENT_HELP: &str = "a comment starts with `#`";

const LINE_CONTINUATION_HELP: &str = "a line continues only when the next line starts with \
                                      `.` or `@`; otherwise, wrap the expression in parentheses";

const CONDITIONAL_HELP: &str = "Frost's conditional is written `if a: b else: c`";

const ELIF_HELP: &str = "use `elif` for another condition";

const REST_HELP: &str = "a rest binding is written `...name`";

const IN_HELP: &str = "Frost has no `in` operator; use `includes(xs, x)` for an Array, \
                       `has(m, k)` for a Map, or `contains(s, part)` for a String";

const IMMUTABLE_HELP: &str =
    "Frost values are immutable; bind an updated value to a new name with `def`";

const GUARD_HELP: &str = "a guard is written `if:` before its condition, like `n if: n > 0 => ...`";

const CATCH_ALL_HELP: &str = "a catch-all arm is `_ => ...`";

const SLICE_HELP: &str = "Frost has no slice syntax; use `slice(xs, start, end)`";

const PATTERN_RANGE_HELP: &str =
    "a pattern cannot be a range; use a guard, like `n if: n >= 1 and n <= 3 => ...`";

const MAP_REST_HELP: &str = "a Map pattern takes no rest binding; it ignores keys it does not name";

const PATTERN_DEFAULTS_HELP: &str = "Frost patterns have no default values";

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

// A modifier between the declaration word and the name is not the name.
#[test]
fn declaration_keywords_with_modifiers() {
    assert_help(&[
        (
            "let mut x = 1",
            "Frost has no `let`; bind a name with `def`: `def x = ...`",
        ),
        (
            "let rec f = 1",
            "Frost has no `let`; bind a name with `def`: `def f = ...`",
        ),
        (
            "local function f() end",
            "Frost has no `function`; define a function with `defn f(...) -> ...`",
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
        (
            "f(x).y = 1",
            "Frost values are immutable; bind an updated value to a new name with `def`",
        ),
    ]);
}

// A name bound already cannot be bound again with `def`, so the help names no `def`.
#[test]
fn reassignment() {
    let after_its_def = r"
        def x = 1
        x = x + 1
    ";
    let a_parameter_in_a_body = r"
        defn f(x) -> {
            x = x + 1
        }
    ";
    assert_help(&[
        (after_its_def, IMMUTABLE_HELP),
        ("fn (x) -> { x = 1 }", IMMUTABLE_HELP),
        (a_parameter_in_a_body, IMMUTABLE_HELP),
    ]);
}

#[test]
fn a_function_defined_by_assignment() {
    assert_help(&[
        ("f(x) = 1", "define a function with `defn f(...) -> ...`"),
        (
            "add(a, b) = a + b",
            "define a function with `defn add(...) -> ...`",
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
        ("{a = 1}", MAP_ENTRY_HELP),
        ("def m = {a = 1}", MAP_ENTRY_HELP),
        ("{a: 1, b = 2}", MAP_ENTRY_HELP),
        // In a `match`, `{a: 1}` matches a value.
        ("match x { {a = 1} => 2 }", MAP_ENTRY_HELP),
        // The parser reads a Map after `->` when it starts `name:`.
        ("def f = fn x -> { a: 1, b = 2 }", MAP_ENTRY_HELP),
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

// A default value for a name a pattern binds, as JavaScript writes one
#[test]
fn pattern_default_values() {
    assert_help(&[
        ("def {a = 1} = m", PATTERN_DEFAULTS_HELP),
        ("def {a: b = 1} = m", PATTERN_DEFAULTS_HELP),
        ("match x { {a: b = 1} => 2 }", PATTERN_DEFAULTS_HELP),
        ("def [a = 1] = m", PATTERN_DEFAULTS_HELP),
    ]);
    // Left unclosed, the pattern is missing its closer, not holding a default.
    assert_help_or_none(&[("def [a, b = [1, 2]", None)]);
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
        ("for x in xs: print(x)", LOOP_HELP_FOR),
        ("while running { step() }", LOOP_HELP_WHILE),
    ]);
}

// A parenthesized loop header reads as a call, and the error falls in or after it.
#[test]
fn c_style_loops() {
    assert_help(&[
        ("for (i = 0; i < 3; i++) {}", LOOP_HELP_FOR),
        ("for (x in xs) { x }", LOOP_HELP_FOR),
        ("while (x) { y }", LOOP_HELP_WHILE),
    ]);
    // Another call's header gets no loop help.
    assert_help_or_none(&[
        ("f(x) { y }", None),
        (
            "f(i = 0)",
            Some("Frost has no named arguments; pass arguments in order"),
        ),
    ]);
}

// A line break or `;` was expected before the definition.
#[test]
fn statements_on_one_line() {
    let help = "separate statements with a line break or `;`";
    assert_help(&[
        ("def a = 1 def b = 2", help),
        ("print(1) def x = 2", help),
        ("def a = 1 defn f(x) -> x", help),
        ("def a = 1 export def b = 2", help),
        ("do { 1 def x = 2; x }", help),
    ]);
}

// `return`, `lambda`, `throw`, and declaration words read as a whole expression, where
// one begins: a body, a branch, an argument, or a definition's value.
#[test]
fn statement_words_where_an_expression_begins() {
    let return_help = "Frost has no `return`; a function's value is its last expression";
    assert_help(&[
        ("def f = fn x -> return x * 2", return_help),
        ("if n <= 1: return 1", return_help),
        ("if c: 1 else: return 2", return_help),
        ("match x { 1 => return 2 }", return_help),
        ("f(1, return 2)", return_help),
        ("def x = return 1", return_help),
        ("f(lambda x: x)", LAMBDA_HELP),
        ("def f = lambda x: x", LAMBDA_HELP),
        ("def f = lambda: 1", LAMBDA_HELP),
        ("map xs with lambda x: x", LAMBDA_HELP),
        (
            "if c: let x = 1",
            "Frost has no `let`; bind a name with `def`: `def x = ...`",
        ),
        (
            "def a = f(let x = 1)",
            "Frost has no `let`; bind a name with `def`: `def x = ...`",
        ),
        (
            "fn x -> throw x",
            "Frost has no `throw`; raise an error with `error(value)`",
        ),
    ]);
    // Away from a statement's start, a declaration word may be a variable, and `return`
    // with no expression after it is no statement.
    assert_help_or_none(&[("f(val x)", None), ("def x = return)", None)]);
}

#[test]
fn errors_from_other_languages() {
    let try_help = "Frost has no `try`; to catch an error, call a function with `try_call(f)`";
    let python_try = r"
        try:
            risky()
        except:
            print(2)
    ";
    assert_help(&[
        ("try { 1 } catch e { 2 }", try_help),
        (python_try, try_help),
        (
            "throw 'x'",
            "Frost has no `throw`; raise an error with `error(value)`",
        ),
        (
            "raise ValueError('x')",
            "Frost has no `raise`; raise an error with `error(value)`",
        ),
    ]);
}

#[test]
fn import_statements() {
    assert_help(&[
        (
            "import std.math",
            "load a module by calling `import`: `def math = import('std.math')`",
        ),
        (
            "import math",
            "load a module by calling `import`: `def math = import('math')`; Frost's own \
             modules are under `std.`, like `std.math`",
        ),
        (
            "import \"std.math\"",
            "load a module by calling `import`: `def math = import('std.math')`",
        ),
        (
            "import numpy as np",
            "load a module by calling `import`: `def np = import('numpy')`; Frost's own \
             modules are under `std.`, like `std.math`",
        ),
    ]);
    // JavaScript's import names no module path to suggest.
    assert_help_or_none(&[("import * as m from 'x'", None)]);
}

// A call needs parentheses; a function is defined with `defn`.
#[test]
fn calls_without_parentheses() {
    let help = "call a function with parentheses: `f(x)`";
    assert_help(&[
        ("print \"hi\"", help),
        ("puts 'hi'", help),
        ("print x", help),
        ("def r = o 1", help),
        ("def y = f x", help),
        ("if c: print x", help),
        ("f x = x + 1", "define a function with `defn f(...) -> ...`"),
        (
            "add a b = a + b",
            "define a function with `defn add(...) -> ...`",
        ),
    ]);
    // A word starting a statement and followed by a name is more often another
    // language's declaration; and a word after the argument makes it no call.
    assert_help_or_none(&[
        ("package main", None),
        ("class Foo:", None),
        ("type T = Int", None),
        ("a xor b", None),
        ("def x 1", None),
    ]);
}

// Several names bound at once are destructured from an Array.
#[test]
fn multiple_assignment() {
    assert_help(&[
        (
            "def a, b = 1, 2",
            "to bind several names, destructure with `def`: `def [a, b] = ...`",
        ),
        (
            "a, b = 1, 2",
            "to bind several names, destructure with `def`: `def [a, b] = ...`",
        ),
        (
            "[a, b] = [1, 2]",
            "to bind several names, destructure with `def`: `def [a, b] = ...`",
        ),
        (
            "{a, b} = m",
            "to bind several names, destructure with `def`: `def {a, b} = ...`",
        ),
    ]);
    // With one item, the brackets may be a computed Map key rather than a pattern.
    assert_help_or_none(&[(
        "[k] = 1",
        Some("Frost values are immutable; bind an updated value to a new name with `def`"),
    )]);
}

// -- Operators --

#[test]
fn logical_operators() {
    assert_help(&[
        ("a && b", "Frost's \"and\" is `and`"),
        ("a || b", "Frost's \"or\" is `or`"),
        ("!a", "Frost's \"not\" is `not`"),
        ("if a && b: 1", "Frost's \"and\" is `and`"),
    ]);
}

// `||` starting a line continues nothing; with no operand before it at all, it is
// Rust's lambda without parameters.
#[test]
fn double_pipe_without_an_operand_before_it() {
    const NO_PARAMETERS_HELP: &str = "a lambda without parameters is written `fn -> ...`";
    let on_the_next_line = r"
        def x = a
            || b
    ";
    assert_help(&[
        (
            on_the_next_line,
            "Frost's \"or\" is `or`; a line continues only when the next line starts with `.` \
             or `@`; otherwise, wrap the expression in parentheses",
        ),
        ("|| 1", NO_PARAMETERS_HELP),
        ("f(|| 1)", NO_PARAMETERS_HELP),
        ("||", NO_PARAMETERS_HELP),
    ]);
}

// In a `match` arm's pattern, `&&` before the `=>` starts a guard.
#[test]
fn double_ampersand_in_a_pattern() {
    assert_help(&[
        ("match x { n is Int && n > 1 => 2 }", GUARD_HELP),
        ("match v { x && y => 1 }", GUARD_HELP),
        // In the guard, or outside a `match`, it is a condition's "and".
        ("match v { x if: x && y => 1 }", "Frost's \"and\" is `and`"),
        (
            "def f = fn x -> { x && y => 1 }",
            "Frost's \"and\" is `and`",
        ),
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

// `?` and `!` mean many things elsewhere; the characters around them tell which.
#[test]
fn question_and_exclamation_marks() {
    let ternary = "Frost's conditional is written `if a: b else: c`";
    let fallback = "for a fallback when a value is null, use `or`: `a or b`";
    let chaining = "Frost has no optional chaining; use `x and x.k`";
    let names = "Frost names cannot contain `?` or `!`";
    assert_help_or_none(&[
        ("a?b:c", Some(ternary)),
        ("f() ? 1 : 2", Some(ternary)),
        ("$'${x ? 1 : 2}'", Some(ternary)),
        ("x ?: y", Some(fallback)),
        ("{a: 1}?.a", Some(chaining)),
        ("print(p?.a)", Some(chaining)),
        ("xs[0]?[1]", Some(chaining)),
        ("xs.empty?", Some(names)),
        ("def done? = true", Some(names)),
        (r#"println!("x")"#, Some(names)),
        // A colon inside brackets after the `?` is not a conditional's.
        ("def v = done? and {a: 1}", Some(names)),
        (
            "match x { n is Int? => 1 }",
            Some("Frost has no optional types; match `null` in an arm of its own"),
        ),
        // Unwrapping a result has no Frost counterpart to suggest.
        ("f()?", None),
        ("xs[0]!", None),
        ("f(!a)", Some("Frost's \"not\" is `not`")),
        // A conditional needs a condition before its `?`.
        ("match x { ? => 1 }", None),
    ]);
}

// A line ends an expression unless the next starts with `.` or `@`.
#[test]
fn line_breaks_inside_expressions() {
    let trailing_operator = r"
        def x = a +
            b
    ";
    let trailing_and = r"
        def x = a and
            b
    ";
    let trailing_equals = r"
        def x =
            1
    ";
    let trailing_thread = r"
        xs @
            f()
    ";
    let leading_operator = r"
        def x = a
            + b
    ";
    let leading_or = r"
        def x = a
            or b
    ";
    let leading_comparison = r"
        def ok = x
            == 1
    ";
    assert_help(&[
        (trailing_operator, LINE_CONTINUATION_HELP),
        (trailing_and, LINE_CONTINUATION_HELP),
        (trailing_equals, LINE_CONTINUATION_HELP),
        (trailing_thread, LINE_CONTINUATION_HELP),
        (leading_operator, LINE_CONTINUATION_HELP),
        (leading_or, LINE_CONTINUATION_HELP),
        (leading_comparison, LINE_CONTINUATION_HELP),
    ]);
}

// Parentheses do not continue a `.` across lines; only a `.` starting the next line does.
#[test]
fn a_dot_ending_a_line() {
    let help = "move the `.` to the start of the next line, where it continues the expression";
    let trailing_dot = r"
        def x = a.
            b
    ";
    let in_parentheses = r"
        print((m.
            b))
    ";
    assert_help(&[(trailing_dot, help), (in_parentheses, help)]);
}

// Where nothing continues the expression on the next line, it is only missing.
#[test]
fn a_missing_operand_gets_no_line_break_help() {
    let a_definition_follows = r"
        def x = 1 +
        def y = 2
    ";
    let a_brace_follows = r"
        fn x -> {
            y +
        }
    ";
    let compound_assignment_on_its_own_line = r"
        x
        += 1
    ";
    let nothing_follows_equals = r"
        def x =
    ";
    let nothing_follows_plus = r"
        def x = 1 +
    ";
    // The `.` here is the Float's, not a continuation's.
    let a_point_float_follows = r"
        def a = 1 +
            .5
    ";
    let nothing_follows_dot = r"
        def x = a.
    ";
    assert_help_or_none(&[
        (nothing_follows_equals, None),
        (nothing_follows_plus, None),
        (a_definition_follows, None),
        (a_brace_follows, None),
        (compound_assignment_on_its_own_line, None),
        (a_point_float_follows, None),
        (nothing_follows_dot, None),
    ]);
}

#[test]
fn bitwise_operators() {
    assert_help(&[
        (
            "a & b",
            "Frost has no bitwise operators; its \"and\" is `and`",
        ),
        (
            "a | b",
            "Frost has no bitwise operators; its \"or\" is `or`",
        ),
        (
            "1 | 2",
            "Frost has no bitwise operators; its \"or\" is `or`",
        ),
        (
            "def x = a | b",
            "Frost has no bitwise operators; its \"or\" is `or`",
        ),
        (
            "a ^ b",
            "Frost has no bitwise operators; for powers, the `std.math` module has `pow`",
        ),
        ("~a", "Frost has no bitwise operators"),
        ("a ~ b", "Frost has no bitwise operators"),
    ]);
    // A `&` before its operand alone is a reference; `=~` matches a pattern; and in a
    // pattern or another language's list syntax, `|` is no operator.
    assert_help_or_none(&[
        ("f(&x)", None),
        ("s =~ t", None),
        ("[x | x <- xs]", None),
        ("{ r | x = 1 }", None),
        ("match x { (1 | 2) => 1 }", None),
        // An intersection of types, which no Frost operator writes
        ("match x { n is Int & Float => 1 }", None),
    ]);
}

// A shell's pipe into a call
#[test]
fn a_pipe_into_a_call() {
    let help = "Frost threads a value into a call with `@`: `x @ f()`";
    assert_help(&[("xs | f(x)", help), ("xs | map(f)", help)]);
}

#[test]
fn inequality_from_other_languages() {
    let help = "Frost's inequality is `!=`";
    assert_help(&[("a <> b", help), ("a ~= b", help)]);
}

#[test]
fn increment_and_join_operators() {
    let increment = "Frost has no `++`; bind the result to a new name with `def`";
    let join = "Frost has no `++`; join with `+`, like `xs + ys`";
    assert_help(&[
        ("x++", increment),
        ("++x", increment),
        ("def b = a++", increment),
        (
            "x--",
            "Frost has no `--`; bind the result to a new name with `def`",
        ),
        ("xs ++ ys", join),
        ("\"a\" ++ \"b\"", join),
    ]);
}

// Python's `*` before a name spreads an argument or gathers a rest.
#[test]
fn star_before_a_name() {
    assert_help(&[
        (
            "f(*args)",
            "Frost has no spread; to pass an Array's elements as arguments, use `call(f, args)`",
        ),
        (
            "[*xs, 1]",
            "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`",
        ),
        ("fn (*args) -> args", "a rest binding is written `...name`"),
        ("def [a, *rest] = xs", "a rest binding is written `...name`"),
        (
            "match v { [h, *t] => h }",
            "a rest binding is written `...name`",
        ),
    ]);
    // C's dereference has no Frost counterpart to suggest.
    assert_help_or_none(&[("(*p)", None)]);
}

#[test]
fn operator_sections() {
    let help = "Frost has no operator sections; use an abbreviated lambda, like `$($ + 1)`";
    assert_help(&[
        ("(+ 1)", help),
        ("def inc = (+ 1)", help),
        ("(*)", help),
        ("xs @ transform(* 2)", help),
    ]);
    // `+1` writes a sign, and `(+ 1 2)` is Lisp's call, not a section.
    assert_help_or_none(&[("(+1)", None), ("(+ 1 2)", None)]);
}

// A declaration word from another language may be a Frost variable's name.
#[test]
fn a_python_conditional_after_a_declaration_word() {
    let last_in_a_block = r"
        fn -> {
            def val = 1
            val if val else 0
        }
    ";
    let help = "Frost's conditional is written `if a: b else: c`";
    assert_help(&[("val if c else 2", help), (last_in_a_block, help)]);
}

// -- Control flow --

#[test]
fn else_if() {
    let source = r"
        if a: 1
        else if b: 2
    ";
    assert_help(&[(source, ELIF_HELP)]);
}

#[test]
fn control_words_from_other_languages() {
    let elsif_on_its_own_line = r"
        if a: 1
        elsif b: 2
    ";
    let unless_help = "Frost has no `unless`; write `if not cond: ...`";
    let switch_help = "Frost has no `switch`; use `match x { ... }`";
    assert_help(&[
        ("if a: 1 elsif b: 2", ELIF_HELP),
        ("if a: 1 elseif b: 2", ELIF_HELP),
        (elsif_on_its_own_line, ELIF_HELP),
        ("x unless a", unless_help),
        ("def y = x unless a", unless_help),
        ("unless x: 1", unless_help),
        (
            "match 1 { _ unless: false => 1 }",
            "a guard is written `if:`; negate it with `not`, like `_ if: not cond => ...`",
        ),
        ("switch x {}", switch_help),
        ("switch (x) { case 1: 2 }", switch_help),
        (
            "until x: 1",
            "Frost has no `until` loop; use `map`, `filter`, `reduce`, or `foreach` with a \
             function, like `foreach xs with fn x -> ...`",
        ),
        ("fn x -> x end", "Frost has no `end`; delete it"),
        ("if a: 1 else: 2 end", "Frost has no `end`; delete it"),
        ("do { 1 } end", "Frost has no `end`; delete it"),
        ("function(a, b) { 1 }", LAMBDA_HELP),
        ("def f = function(a) { a }", LAMBDA_HELP),
    ]);
    // `end` before more of its line is no block's end.
    assert_help_or_none(&[("x end y", None)]);
}

// An `else` or `elif` starting a statement continues no `if`: the `if` branch before it
// took several statements, or there is no `if`.
#[test]
fn branches_without_their_if() {
    let python_branch = r"
        if x:
            print(1)
            print(2)
        else:
            3
    ";
    let second_else = r"
        if a: 1 else: 2
        else: 3
    ";
    let empty_branch = r"
        if x:
        else: 2
    ";
    assert_help_or_none(&[
        (
            python_branch,
            Some("an `if` branch is one expression; for several statements, use `do { ... }`"),
        ),
        ("else: 1", Some(CONDITIONAL_HELP)),
        ("elif x: 1", Some(CONDITIONAL_HELP)),
        (second_else, Some(CONDITIONAL_HELP)),
        (
            "if x: 1 elif: 2",
            Some("`elif` takes a condition; for the last branch, use `else:`"),
        ),
        // The branch is missing, not a statement too many.
        (empty_branch, None),
        // A Map entry, not a statement
        ("match x { 1 => { else => 2 } }", None),
    ]);
}

#[test]
fn a_condition_without_its_colon() {
    let if_help = "`if` takes a colon after its condition: `if x: ...`";
    let on_its_own_line = r"
        if x
            1
    ";
    assert_help(&[
        ("if (x) 1", if_help),
        ("if x then 1 else 2", if_help),
        (on_its_own_line, if_help),
        (
            "if a: 1 elif b 2",
            "`elif` takes a colon after its condition: `elif x: ...`",
        ),
        (
            "if a: 1 else 2",
            "`else` is followed by a colon: `else: ...`",
        ),
        // A bracket opened before the `if` holds it, so the colon is still the `if`'s.
        ("{a: if x 1}", if_help),
        ("f(if x 1)", if_help),
        // The `if` in the condition has its colon; the `elif` owes one.
        (
            "if x: 1 elif if y: 2",
            "`elif` takes a colon after its condition: `elif x: ...`",
        ),
        ("if a: 1 else: if b 2", if_help),
    ]);
}

// Braces after a condition were meant as a block, which Frost writes with `do`.
#[test]
fn a_block_after_a_condition_without_its_colon() {
    let if_help = "`if` takes a colon after its condition; for a block, write `if x: do { ... }`";
    assert_help(&[
        ("if x { 1 }", if_help),
        ("if (x) { 1 }", if_help),
        ("if x { 1 } else { 2 }", if_help),
        (
            "if a: 1 elif b { 2 }",
            "`elif` takes a colon after its condition; for a block, write \
             `elif x: do { ... }`",
        ),
        (
            "if a: 1 elif (b) { 2 }",
            "`elif` takes a colon after its condition; for a block, write \
             `elif x: do { ... }`",
        ),
        (
            "if a: 1 else { 2 }",
            "`else` is followed by a colon; for a block, write `else: do { ... }`",
        ),
    ]);
}

// An `=` before the condition's colon is a comparison written with one `=`.
#[test]
fn assignment_as_a_condition() {
    let help = "Frost's equality is `==`";
    assert_help(&[
        ("if x = 1: 2", help),
        ("if a: 1 elif x = 2: 3", help),
        // The colon missing here is a Map entry's.
        ("if c: {[k] = 1}", MAP_ENTRY_HELP),
    ]);
}

// A colon missing inside a bracket opened after the `if` is not the `if`'s.
#[test]
fn a_map_entry_without_its_colon() {
    assert_help(&[
        ("if c: {[k] 1}", MAP_ENTRY_HELP),
        ("def m = {[k] 1}", MAP_ENTRY_HELP),
        ("match m { {[k] v} => v }", MAP_ENTRY_HELP),
        ("def {[k] v} = m", MAP_ENTRY_HELP),
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

// In a `match`, `=>` belongs; what is missing comes before it.
#[test]
fn arrows_in_a_match_get_no_lambda_help() {
    let type_missing = r"
        match x {
            n is =>1,
        }
    ";
    assert_help_or_none(&[
        (type_missing, None),
        ("match x { 1 | => 1 }", None),
        ("match x { {a} as => a }", None),
        ("match x { [a, b => 1 }", None),
        ("match x { 1 => y => 2 }", None),
    ]);
}

#[test]
fn guards_from_other_languages() {
    let guard_help = "a guard is written `if:` before its condition, like `n if: n > 0 => ...`";
    assert_help(&[
        ("match x { n if n > 0 => n }", guard_help),
        ("match x { n when n > 0 => n }", guard_help),
        ("match x { n is Int and n > 1 => 2 }", guard_help),
        ("match x { n and n > 0 => 1 }", guard_help),
    ]);
}

// An `if` out of place in an arm is a guard: after the result, a second guard, or one
// with no pattern.
#[test]
fn guards_out_of_place() {
    let help = "an arm takes one guard, between its pattern and `=>`, like `n if: n > 0 => ...`; \
                combine conditions with `and`";
    assert_help(&[
        ("match 1 { _ => 1 if: true }", help),
        ("match x { 1 => 2 if y }", help),
        ("match 1 { n if: n > 0 if: true => 1 }", help),
        ("match 1 { if: true => 1 }", help),
        // Python's conditional in a result, and an `if` after the `match`
        ("match x { 1 => b if a else c }", CONDITIONAL_HELP),
        ("match x { 1 => 2 } if y", CONDITIONAL_HELP),
    ]);
}

// A declaration where a pattern starts: a bare name binds there.
#[test]
fn declarations_as_patterns() {
    let help = "a bare name binds in a pattern, like `y => ...`";
    assert_help(&[
        ("match x { let y => 1 }", help),
        ("match 1 { def y = 1 => 2 }", help),
    ]);
}

// Parentheses where a pattern starts, or around the arms, hold no lambda.
#[test]
fn arrows_in_parentheses_in_a_match() {
    let unclosed_group = r"
        match x {
            (y => 1,
        }
    ";
    assert_help_or_none(&[
        (unclosed_group, None),
        (
            "match x ( _ => 1 )",
            Some("`match` arms go in braces: `match x { ... }`"),
        ),
        ("match v { |x| => 1 }", None),
        // A `{` after the call shows it is the value to match.
        ("match f(x => 1) { _ => 1 }", Some(LAMBDA_HELP)),
    ]);
}

#[test]
fn pattern_alternatives_from_other_languages() {
    let help = "pattern alternatives are separated by `|`, like `1 | 2 => ...`";
    // After a type test, each alternative needs its own: `n is Int | String` binds no `n`
    // in its second alternative.
    let typed = "pattern alternatives are separated by `|`, each with its own test: \
                 `n is Int | n is String => ...`";
    assert_help(&[
        ("match x { 1 || 2 => 1 }", help),
        ("match x { 1 or 2 => 1 }", help),
        ("match x { n is Int or n is String => 1 }", typed),
        ("match x { n is Int or String => 1 }", typed),
    ]);
}

// In a pattern, a type is tested with `is` and a type name.
#[test]
fn type_tests_from_other_languages() {
    assert_help(&[
        (
            "match x { n: Int => 1 }",
            "test a type with `is`: `n is Int => ...`",
        ),
        (
            "match x { n is not Int => 1 }",
            "to exclude a type, use a guard: `n if: not is_int(n) => ...`",
        ),
        (
            "match x { n is not Null => 1 }",
            "to exclude a type, use a guard: `n if: not is_null(n) => ...`",
        ),
        // `_` binds nothing for the guard to test.
        (
            "match 1 { _ is not Int => 1 }",
            "to exclude a type, use a guard: `n if: not is_int(n) => ...`",
        ),
        ("match x { n is null => 1 }", "did you mean `Null`?"),
        ("match x { n is None => 1 }", "did you mean `Null`?"),
        ("match x { n is map => 1 }", "did you mean `Map`?"),
    ]);
}

// Outside a `match` pattern, a type is tested with a function.
#[test]
fn type_tests_outside_a_match() {
    assert_help(&[
        (
            "x is None",
            "`is` works only in a `match` pattern; elsewhere, use `is_null(x)`",
        ),
        (
            "x is not None",
            "`is` works only in a `match` pattern; elsewhere, use `not is_null(x)`",
        ),
        (
            "if x is Int: 1",
            "`is` works only in a `match` pattern; elsewhere, use `is_int(x)`",
        ),
        (
            "if x is not Int: 1",
            "`is` works only in a `match` pattern; elsewhere, use `not is_int(x)`",
        ),
        (
            "def ok = n is String",
            "`is` works only in a `match` pattern; elsewhere, use `is_string(n)`",
        ),
        // The name before `is` is echoed only when it is the whole value tested.
        (
            "if m.k is Int: 1",
            "`is` works only in a `match` pattern; elsewhere, use `is_int(x)`",
        ),
        (
            "def r = m.k is Int",
            "`is` works only in a `match` pattern; elsewhere, use `is_int(x)`",
        ),
        (
            "if a + b is Int: 1",
            "`is` works only in a `match` pattern; elsewhere, use `is_int(x)`",
        ),
        // `and`, `or`, and `not` bind loosely, so the name after one is the whole value.
        (
            "if x and a is Int: 1",
            "`is` works only in a `match` pattern; elsewhere, use `is_int(a)`",
        ),
        (
            "if not a is Int: 1",
            "`is` works only in a `match` pattern; elsewhere, use `is_int(a)`",
        ),
        (
            "if a or b is Int: 1",
            "`is` works only in a `match` pattern; elsewhere, use `is_int(b)`",
        ),
        (
            "x is Number",
            "`is` works only in a `match` pattern; elsewhere, test a type with a function \
             like `is_int(x)`",
        ),
        // Python's identity test
        (
            "a is b",
            "`is` works only in a `match` pattern; to compare values, use `==` or `!=`",
        ),
        (
            "if a is not b: 1",
            "`is` works only in a `match` pattern; to compare values, use `==` or `!=`",
        ),
    ]);
}

// A `match` body's braces hold arms, not a Map or a block.
#[test]
fn match_bodies_get_no_map_or_block_help() {
    let multiline = r"
        match x {
            a.b => 2
        }
    ";
    assert_help_or_none(&[
        ("match x { a.b => 2 }", None),
        ("match x { Some(y) => 2 }", None),
        (multiline, None),
        // `$` starting a pattern stands for any value.
        ("match v { $ => 1 }", Some(CATCH_ALL_HELP)),
    ]);
}

// A keyword is a name only where a name's pattern would end after it.
#[test]
fn keywords_starting_other_patterns_get_no_name_help() {
    assert_help_or_none(&[
        ("match x { not 1 => 1 }", None),
        ("match x { is Int => 1 }", None),
        ("match x { fn y -> y => 2 }", None),
        ("def {a: not 1} = m", None),
    ]);
}

// Before an arm's `=>`, `=` is a comparison; after it, the result is an expression.
#[test]
fn equals_in_a_match() {
    let help = "Frost's equality is `==`";
    assert_help(&[
        ("match x { n if: n = 1 => 2 }", help),
        ("match x { n = 1 => 2 }", help),
        ("match x { 1 => y = 2 }", IMMUTABLE_HELP),
    ]);
}

#[test]
fn else_as_a_match_arm() {
    let help = "a catch-all arm is `_ => ...`";
    let after_other_arms = r"
        match x {
            1 => 'one',
            else => 'other'
        }
    ";
    assert_help(&[
        ("match x { else => 1 }", help),
        ("match x { else: 1 }", help),
        ("match x { 1 | else => 3 }", help),
        ("match x { else if y => 1 }", help),
        (after_other_arms, help),
    ]);
}

// Without a value to match, the arms' braces are read as a Map literal.
#[test]
fn a_match_without_its_value() {
    let help = "`match` takes the value to match before its `{`: `match x { ... }`";
    assert_help(&[
        ("match { 1 => 2 }", help),
        ("match { x => 2 }", help),
        ("def y = match { 'a' => 1, _ => 2 }", help),
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
    // An entry written `key: name` renames already.
    assert_help_or_none(&[("def {a: b as c} = x", None)]);
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
        ("fn x: Int = 1 -> x", help),
        ("fn x: List[Int] -> x", help),
        ("fn (x: Int) -> x", help),
        ("defn f(x: Int) -> x", help),
        (
            "defn f(x): Int -> x",
            "Frost has no return types; a function body follows `->`",
        ),
        (
            "defn f(x): Int { x }",
            "Frost has no return types; a function body follows `->`",
        ),
    ]);
}

// An arrow after a lambda's body name is a curried lambda missing its inner `fn`.
#[test]
fn curried_lambdas_without_their_inner_fn() {
    let help = "each lambda takes its own `fn`: `fn x -> fn y -> ...`";
    assert_help(&[
        ("def f = fn x -> y -> z", help),
        ("defn add(x) -> y -> x + y", help),
        ("map xs with fn x -> y -> 1", help),
        ("f(fn x -> y -> 1)", help),
    ]);
}

// A colon after the parameters, with a body rather than a type after it, is Python's
// lambda.
#[test]
fn a_python_lambda_written_with_fn() {
    assert_help(&[
        ("fn x: x + 1", LAMBDA_HELP),
        ("fn x, y: x + y", LAMBDA_HELP),
        ("def f = fn x: x", LAMBDA_HELP),
    ]);
}

// After the parameter list, `=` or `:` stands where the body's `->` belongs.
#[test]
fn a_body_after_something_other_than_its_arrow() {
    let python_body = r"
        defn add(a, b):
            a + b
    ";
    assert_help(&[
        ("defn f(x) = x", "a function body follows `->`, not `=`"),
        (
            "defn parse_line(l) = {",
            "a function body follows `->`, not `=`",
        ),
        (
            "def f = fn (x) = x",
            "a function body follows `->`, not `=`",
        ),
        (python_body, "a function body follows `->`, not `:`"),
        ("defn f(x): x + 1", "a function body follows `->`, not `:`"),
    ]);
}

#[test]
fn a_body_without_its_arrow() {
    let help = "add `->` before the body: `-> ...`";
    assert_help(&[
        ("defn f(x) x + 1", help),
        ("def f = fn (x, y) x + y", help),
        (
            "fn x y -> x",
            "separate parameters with `,`, like `fn x, y -> ...`",
        ),
        (
            "def f = fn x, y z -> x",
            "separate parameters with `,`, like `fn x, y -> ...`",
        ),
    ]);
}

// A type after the arrow, then the body in braces
#[test]
fn return_types_before_a_body() {
    let help = "Frost has no return types; a function body follows `->`";
    let on_lines = r"
        defn f(x) -> Int {
            x
        }
    ";
    assert_help(&[
        ("defn f(x) -> Int { x }", help),
        ("fn f(x) -> i32 { x }", help),
        (on_lines, help),
    ]);
    // A lowercase name may be the body.
    assert_help_or_none(&[("fn x -> y { 1 }", None)]);
}

// Parameters are names; an argument is destructured or matched in the body. The advice
// stays true whatever parameters may become.
#[test]
fn patterns_as_parameters() {
    let map_help =
        "to destructure an argument, use `def` in the body: `fn x -> { def {a, b} = x; ... }`";
    let array_help =
        "to destructure an argument, use `def` in the body: `fn x -> { def [a, b] = x; ... }`";
    let match_help =
        "to handle particular values, `match` the argument in the body: `match n { 0 => ... }`";
    assert_help(&[
        ("fn {first, last} -> first", map_help),
        ("fn ({first, last}) -> first", map_help),
        ("fn [a, b] -> a", array_help),
        ("defn f([a, b]) -> a", array_help),
        ("defn f(x, [a, b]) -> a", array_help),
        ("defn fib(0) -> 1", match_help),
        ("fn (\"a\") -> 1", match_help),
    ]);
    // Without an arrow after the braces, they may be something else.
    assert_help_or_none(&[("fn { x }", None)]);
}

#[test]
fn rest_bindings_without_names() {
    assert_help(&[
        ("match v { [a, ...] => 1 }", REST_HELP),
        ("match v { [...] => 1 }", REST_HELP),
        ("def [a, ...] = xs", REST_HELP),
        ("fn (...) -> x", REST_HELP),
        // Named, the rest would still be out of place.
        (
            "def [..., a] = xs",
            "a rest binding is written `...name`, and comes last",
        ),
    ]);
}

// Another language's lambda with a parameter list, read as parentheses or a statement
#[test]
fn arrow_lambdas_with_parameter_lists() {
    assert_help(&[
        ("(a, b) => a + b", LAMBDA_HELP),
        ("(a, b) -> a + b", LAMBDA_HELP),
        ("() => 1", LAMBDA_HELP),
        ("def f = () -> 1", LAMBDA_HELP),
        ("x, y -> x", LAMBDA_HELP),
        ("x, y => x", LAMBDA_HELP),
        ("def add = (x, y) => x + y", LAMBDA_HELP),
        ("xs @ transform((a, b) => a)", LAMBDA_HELP),
    ]);
    // In a `match`, parentheses before `=>` hold a pattern.
    assert_help_or_none(&[("()", None), ("match x { () => 1 }", None)]);
}

#[test]
fn tuples() {
    let help = "Frost has no tuples; use an Array, like `[1, 2]`";
    assert_help(&[
        ("def q = (1, 2)", help),
        ("(1,)", help),
        ("(a, b)", help),
        // In a `match`, an arrow after the parentheses is the arm's.
        ("match x { (a, b) => 1 }", help),
    ]);
}

#[test]
fn default_parameter_values() {
    let help = "Frost parameters have no default values";
    assert_help(&[
        ("def f = fn x = 1 -> x", help),
        ("def f = fn a, b = 2 -> a + b", help),
    ]);
}

#[test]
fn a_trailing_comma_in_bare_parameters() {
    assert_help(&[(
        "def f = fn x, -> x",
        "parameters without parentheses take no trailing comma",
    )]);
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

// `reduce` takes an initial value with `init:`; `fold` takes it last.
#[test]
fn reduce_with_an_initial_value() {
    let seeded = "`reduce` is an expression, written `reduce xs init: v with f`; its function \
                  form is `fold(xs, f, v)`";
    let before_with = "an initial value goes before `with`: `reduce xs init: v with f`";
    assert_help(&[
        ("reduce(xs, 0, fn (a, x) -> a + x)", seeded),
        ("xs @ reduce(0, f)", seeded),
        ("reduce xs with f from 0", before_with),
        ("reduce xs with f init: 0", before_with),
        // Without the initial value
        (
            "reduce(xs, f)",
            "`reduce` is an expression, written `reduce xs with f`; its function form is \
             `fold(xs, f)`",
        ),
    ]);
}

// -- Collections from other languages --

#[test]
fn slices() {
    let help = "Frost has no slice syntax; use `slice(xs, start, end)`";
    assert_help(&[("xs[1:2]", help), ("xs[:2]", help), ("xs[1:]", help)]);
}

#[test]
fn comprehensions() {
    let help =
        "Frost has no comprehensions; use `map` and `filter`, like `map xs with fn x -> ...`";
    assert_help(&[
        ("[x for x in xs]", help),
        ("[x * 2 for x in xs]", help),
        ("f(x for x in xs)", help),
    ]);
    // Mapping a Map takes a function returning entries; its comprehension gets no help.
    assert_help_or_none(&[("{k: v for k in m}", None)]);
}

#[test]
fn membership_tests() {
    assert_help(&[
        ("3 in xs", IN_HELP),
        ("x in xs", IN_HELP),
        ("x not in xs", IN_HELP),
        ("if x in xs: 1", IN_HELP),
        ("print(3 in [1, 2, 3])", IN_HELP),
    ]);
}

// -- Lists missing commas --

// Items on separate lines are still separated by commas, in every kind of list.
#[test]
fn list_items_on_separate_lines() {
    let match_arms = r"
        match x {
            1 => 'a'
            2 => 'b'
        }
    ";
    let else_arm = r"
        match x {
            1 => 'a'
            else => 'b'
        }
    ";
    let arguments = r"
        f(
            a,
            b
            c
        )
    ";
    let map_entries = r"
        def m = {
            a: 1
            b: 2
        }
    ";
    let shorthand_entries = r"
        def m = {
            a
            b
        }
    ";
    let array_elements = r"
        [
            1
            2
        ]
    ";
    let array_pattern = r"
        match x {
            [a
            b] => 1
        }
    ";
    let map_pattern = r"
        match x {
            {a
            b} => 1
        }
    ";
    let destructure = r"
        def [
            a
            b
        ] = xs
    ";
    let parameters = r"
        defn f(
            a
            b
        ) -> a
    ";
    assert_help(&[
        (
            match_arms,
            "separate `match` arms with `,`, even across lines",
        ),
        (
            else_arm,
            "separate `match` arms with `,`, even across lines; a catch-all arm is `_ => ...`",
        ),
        (arguments, "separate arguments with `,`, even across lines"),
        (
            map_entries,
            "separate Map entries with `,`, even across lines",
        ),
        (
            shorthand_entries,
            "separate Map entries with `,`, even across lines",
        ),
        (
            array_elements,
            "separate Array elements with `,`, even across lines",
        ),
        (
            array_pattern,
            "separate Array elements with `,`, even across lines",
        ),
        (
            map_pattern,
            "separate Map entries with `,`, even across lines",
        ),
        (
            destructure,
            "separate Array elements with `,`, even across lines",
        ),
        (
            parameters,
            "separate parameters with `,`, even across lines",
        ),
    ]);
}

// A list left unclosed reads the next line's statement as its next item; the missing
// closer, not a comma, is the fix.
#[test]
fn an_unclosed_list_gets_no_comma_help() {
    let call = r"
        print(foo(1)
        print(2)
    ";
    let array = r"
        def a = [1, 2
        print(a)
    ";
    let map = r"
        def m = {a: 1
        print(m)
    ";
    let call_in_an_arm = r"
        match x {
            1 => f(2
            3 => 4
        }
    ";
    let call_in_a_block = r"
        defn f(x) -> {
            print(g(x)
            x
        }
    ";
    let semicolon_after_a_call = r"
        print(foo(1);
        print(2)
    ";
    assert_help_or_none(&[
        (call, None),
        (array, None),
        (map, None),
        (call_in_an_arm, None),
        (call_in_a_block, None),
        (semicolon_after_a_call, None),
    ]);
}

// Names on separate lines in a branch's braces were meant as a block's statements.
#[test]
fn names_on_separate_lines_in_branch_braces() {
    let source = r"
        if c: {
            x
            y
        }
    ";
    assert_help(&[(source, BLOCK_HELP)]);
}

#[test]
fn semicolons_between_list_items() {
    let arms_on_lines = r"
        match x {
            1 => 'a';
            2 => 'b'
        }
    ";
    assert_help(&[
        (arms_on_lines, "separate `match` arms with `,`, not `;`"),
        (
            "match x { 1 => 'a'; 2 => 'b' }",
            "separate `match` arms with `,`, not `;`",
        ),
        ("f(a; b)", "separate arguments with `,`, not `;`"),
        ("[1; 2]", "separate Array elements with `,`, not `;`"),
        ("{a: 1; b: 2}", "separate Map entries with `,`, not `;`"),
        ("def {a; b} = m", "separate Map entries with `,`, not `;`"),
    ]);
}

// With no arm after it, a `;` in an arm separates statements meant for its result.
#[test]
fn statements_in_a_match_arm() {
    let source = r"
        match x {
            1 => print(1); print(2),
        }
    ";
    assert_help(&[(
        source,
        "a `match` arm's result is one expression; for several statements, use `do { ... }`",
    )]);
}

// -- Functions from other languages --

#[test]
fn lambdas_from_other_languages() {
    assert_help(&[
        ("def f = x -> x + 1", LAMBDA_HELP),
        ("def f = x => x + 1", LAMBDA_HELP),
        ("def f = (x) => x + 1", LAMBDA_HELP),
        ("xs @ transform(x => x * 2)", LAMBDA_HELP),
        ("def f = \\x -> x + 1", LAMBDA_HELP),
        ("|x| x + 1", LAMBDA_HELP),
        ("|x, y| x + y", LAMBDA_HELP),
        ("lambda x: x + 1", LAMBDA_HELP),
        // A lambda in braces, as Kotlin, Groovy, and Swift write one
        ("def f = { x -> x + 1 }", LAMBDA_HELP),
        ("def f = { x, y -> x }", LAMBDA_HELP),
        ("xs @ transform({ x -> x * 2 })", LAMBDA_HELP),
        // Ruby's lambda
        ("def f = ->(x) { x + 1 }", LAMBDA_HELP),
    ]);
}

// An arrow after something that could not be parameters is not a lambda's.
#[test]
fn arrows_after_other_things_get_no_lambda_help() {
    let in_an_array = r"
        def y = [1, 2,
            3 => 4
    ";
    let an_arm_after_a_match = r"
        match x {
            1 => 2
        }
        y => 3
    ";
    assert_help_or_none(&[
        ("1 => 2 => 3", None),
        (in_an_array, None),
        // A Map entry's value is no lambda's parameter list.
        ("def m = {a: x -> 1}", None),
        ("{ x => x + 1 }", Some(MAP_ENTRY_HELP)),
        // Ruby's and PHP's named argument
        (
            r#"f("a" => 1)"#,
            Some("Frost has no named arguments; pass arguments in order"),
        ),
        // After a complete lambda, or after a `match`'s arms
        ("fn x -> x => 1", None),
        ("defn f(x) -> x => y", None),
        (an_arm_after_a_match, None),
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
        ("${x}", help),
        ("print(${x})", help),
        (
            "def s = $'${a ${b}}'",
            "`${` interpolates only in a format String's text, not inside another `${...}`",
        ),
    ]);
}

#[test]
fn string_prefixes_from_other_languages() {
    let bytes_help = "a Bytes literal is written in hex, like `x'00ff'`; for a String's bytes, use `to_bytes(s)`";
    // Raw Strings take one line.
    let raw_on_lines = r#"
        def x = R"""(
        abc
        )"""
    "#;
    let raw_on_lines_in_single_quotes = r"
        def x = r'''(
        abc
        )'''
    ";
    assert_help(&[
        ("r'abc'", "a raw String is written `R'(...)'`"),
        ("R'a'", "a raw String is written `R'(...)'`"),
        ("def x = r\"abc\"", r#"a raw String is written `R"(...)"`"#),
        (
            raw_on_lines,
            r#"Frost has no multiline raw String; use a multiline String, `"""..."""`"#,
        ),
        (
            raw_on_lines_in_single_quotes,
            "Frost has no multiline raw String; use a multiline String, `'''...'''`",
        ),
        ("b'abc'", bytes_help),
        ("B'68'", bytes_help),
        ("X'00ff'", bytes_help),
        ("match x { b'x' => 3 }", bytes_help),
        ("u'abc'", "a String needs no `u` prefix"),
        ("F'hi'", "a format String is written `$'...${x}...'`"),
    ]);
}

#[test]
fn adjacent_strings() {
    let across_lines = r#"
        print("a"
            "b")
    "#;
    assert_help(&[
        (
            across_lines,
            "adjacent Strings do not join; join them with `+`",
        ),
        (
            "print('a' 'b')",
            "adjacent Strings do not join; join them with `+`, or separate them with `,`",
        ),
        (
            "'a' 'b'",
            "adjacent Strings do not join; join them with `+`",
        ),
        (
            "def s = 'a' \"b\"",
            "adjacent Strings do not join; join them with `+`",
        ),
        // A doubled quote, as SQL and Pascal escape one
        (
            "def a = 'it''s'",
            r"write a quote inside a String with a backslash: `\'`",
        ),
        (
            r#""say ""hi""""#,
            r#"write a quote inside a String with a backslash: `\"`"#,
        ),
    ]);
    // In a pattern, the second String is no argument.
    assert_help_or_none(&[("match x { 'a' 'b' => 1 }", None)]);
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
    // Trailing spaces and a tab after the `\`
    let trailing_whitespace = "def x = 1 + \\  \t\n    2";
    assert_help(&[
        (source, LINE_CONTINUATION_HELP),
        (trailing_whitespace, LINE_CONTINUATION_HELP),
    ]);
}

// A backslash outside a String is a lambda only before parameters and an arrow; before
// an escape's letter, it is an escape, often after a quote that ended the String early.
#[test]
fn backslashes_outside_strings() {
    let escape_help = "a backslash escape works only inside a String";
    assert_help_or_none(&[
        (r"\x -> x", Some(LAMBDA_HELP)),
        (r"\(x) -> x", Some(LAMBDA_HELP)),
        (r"\(a, b) -> a + b", Some(LAMBDA_HELP)),
        (r"x\n", Some(escape_help)),
        (r"print(\n)", Some(escape_help)),
        (r#""C:" + \\ + "x""#, Some(escape_help)),
        (r#"def a = "a' "t\""#, Some(escape_help)),
        (r"def a = $'${\n1\n}'", Some(escape_help)),
        (r"[1, 2\]", None),
        // At the end of an interpolation or the input, no line follows to continue.
        (r"def b = $'${a\}'", None),
        (r"def a = 1 + \", None),
    ]);
}

// -- Source the lexer cannot read --
// These are not habits, but the lexer's error says what it found.

#[test]
fn unreadable_source() {
    let unclosed_multiline_single = r"
        def x = '''
        abc
    ";
    let unclosed_multiline_double = r#"
        def x = """
        abc
    "#;
    let multiline_format_string = r"
        def x = $'''
        abc ${1}
        '''
    ";
    let quote_on_a_later_line = r"
        print($'total: ${n)
        print('done')
    ";
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
            r#"R"abc"#,
            "unclosed String",
            Some(r#"a raw String is written `R"(...)"`"#),
        ),
        // `R` is part of a longer name here, not a raw String's prefix.
        (
            "xR'abc",
            "unclosed String",
            Some("a String ends with `'` on the same line"),
        ),
        (
            "def a = R'abc",
            "unclosed String",
            Some("a raw String is written `R'(...)'`"),
        ),
        (
            unclosed_multiline_single,
            "unclosed multiline String",
            Some("a multiline String ends with `'''`"),
        ),
        (
            unclosed_multiline_double,
            "unclosed multiline String",
            Some(r#"a multiline String ends with `"""`"#),
        ),
        (
            multiline_format_string,
            "unclosed String",
            Some(
                "Frost has no multiline format String; a format String ends with `'` on the \
                 same line",
            ),
        ),
        // A quote inside an interpolation opens a String of its own.
        (
            "def a = $'${ {a: 1 }'",
            "unclosed interpolation in format String",
            Some("inside `${...}`, each `{` needs a `}`, and a `'` starts a nested String"),
        ),
        (
            "def b = $'${'}'",
            "unclosed interpolation in format String",
            Some("inside `${...}`, each `{` needs a `}`, and a `'` starts a nested String"),
        ),
        (
            r#"def a = $"${ {a: 1 }""#,
            "unclosed interpolation in format String",
            Some(r#"inside `${...}`, each `{` needs a `}`, and a `"` starts a nested String"#),
        ),
        // The nested String that runs to the line end opens with the other quote.
        (
            r#"def a = $'${ "abc }'"#,
            "unclosed interpolation in format String",
            Some(r#"inside `${...}`, each `{` needs a `}`, and a `"` starts a nested String"#),
        ),
        (
            r#"def a = $"${ 'abc }""#,
            "unclosed interpolation in format String",
            Some("inside `${...}`, each `{` needs a `}`, and a `'` starts a nested String"),
        ),
        // A quote on a later line is not this format String's closer.
        (
            quote_on_a_later_line,
            "unclosed format String",
            Some("a format String ends with `'` on the same line"),
        ),
        (
            "def b = 1.e3",
            "invalid number `1.e3`",
            Some("a Float needs digits after its point, like `1.0e3`"),
        ),
        (
            "1.E-3",
            "invalid number `1.E-3`",
            Some("a Float needs digits after its point, like `1.0E-3`"),
        ),
        (
            "1 ~ 2",
            "unexpected character `~`",
            Some("Frost has no bitwise operators"),
        ),
        // A backtick in backticks would read as noise.
        (
            "`ls`",
            "unexpected backtick",
            Some("a format String is written `$'...${x}...'`"),
        ),
        (
            "def caf\u{e9} = 1",
            "unexpected character `\u{e9}` (U+00E9)",
            Some("Frost names use only ASCII letters, digits, and `_`"),
        ),
        (
            "1\u{a0}+ 2",
            "unexpected character U+00A0",
            Some("this is not a plain space; replace it with one"),
        ),
        (
            "\u{feff}1",
            "unexpected character U+FEFF",
            Some("the file starts with a byte order mark; save it as UTF-8 without one"),
        ),
        // Unicode with no ASCII counterpart to suggest
        (
            "1 \u{2603} 2",
            "unexpected character `\u{2603}` (U+2603)",
            None,
        ),
    ];
    for (source, message, help) in cases {
        let (actual_message, actual_help) = diagnosis(source);
        assert_eq!(actual_message, message, "{source:?}");
        assert_eq!(actual_help.as_deref(), help, "{source:?}");
    }
}

// Pasted text brings characters that look like Frost's but are not ASCII.
#[test]
fn unicode_lookalikes() {
    let invisible = "this character is invisible; delete it";
    let names = "Frost names use only ASCII letters, digits, and `_`";
    assert_help(&[
        // A byte order mark starts the file; elsewhere, it is a zero-width space.
        (
            "\u{feff}print(1)",
            "the file starts with a byte order mark; save it as UTF-8 without one",
        ),
        ("print(1)\u{feff}", invisible),
        ("1\u{200b}+ 2", invisible),
        (
            "1 +\u{2009}2",
            "this is not a plain space; replace it with one",
        ),
        ("a \u{2260} b", "Frost's operators are ASCII; write `!=`"),
        ("a \u{2264} b", "Frost's operators are ASCII; write `<=`"),
        ("a \u{2265} b", "Frost's operators are ASCII; write `>=`"),
        ("fn x \u{2192} x", "Frost's operators are ASCII; write `->`"),
        ("1 \u{2212} 2", "Frost's operators are ASCII; write `-`"),
        ("1 \u{2013} 2", "Frost's operators are ASCII; write `-`"),
        ("1 \u{2014} 2", "Frost's operators are ASCII; write `-`"),
        ("3 \u{00d7} 4", "Frost's operators are ASCII; write `*`"),
        ("xs\u{2026}", "Frost's operators are ASCII; write `...`"),
        ("def \u{3b1} = 1", names),
        ("def \u{3bb} = 5", names),
        // The lambda calculus's lambda
        ("\u{3bb}x. x", LAMBDA_HELP),
    ]);
}

// -- Definitions where an expression belongs --

#[test]
fn definitions_inside_expressions() {
    let indented_body = r"
        defn f(a) ->
            def y = a
            y
    ";
    let block_help = "a definition is a statement; for statements inside an expression, use a block: \
         `do { ... }`";
    assert_help(&[
        (
            indented_body,
            "a function body with statements goes in braces: `-> { ... }`",
        ),
        ("if true: def x = 1", block_help),
        ("f(defn g(x) -> x)", block_help),
        (
            "do { export def x = 1; x }",
            "`export` is only allowed at the top level",
        ),
        (
            "export fn x -> x",
            "`export` takes a definition: `export def x = ...` or `export defn f(...) -> ...`",
        ),
    ]);
}

// -- Operators Frost lacks, by their touching tokens --

#[test]
fn multi_character_operators() {
    assert_help(&[
        (
            "2 ** 3",
            "Frost has no `**`; for powers, the `std.math` module has `pow`",
        ),
        ("1 === 1", "Frost's equality is `==`"),
        ("1 !== 1", "Frost's inequality is `!=`"),
        ("1 << 2", "Frost has no shift operators"),
        ("8 >> 1", "Frost has no shift operators"),
    ]);
}

#[test]
fn comments_from_other_languages() {
    assert_help(&[
        ("// a note", COMMENT_HELP),
        ("/* a note */", COMMENT_HELP),
        ("f(// a note", COMMENT_HELP),
        ("1 /* a note */ + 2", COMMENT_HELP),
    ]);
}

// After an operand, `//` may be floor division or a comment.
#[test]
fn double_slash_after_an_operand() {
    let help = "Frost has no `//`; `/` on Ints already gives an Int, rounding toward zero; \
                a comment starts with `#`";
    assert_help(&[
        ("def q = 7 // 2", help),
        ("print(10 // 3)", help),
        ("7 // a note", help),
    ]);
}

// Operator characters separated by a space are not one foreign operator.
#[test]
fn spaced_operators_get_no_operator_help() {
    for source in ["2 * * 3", "1 == = 1", "1 < < 2", "7 / / 2"] {
        let (message, help) = diagnosis(source);
        assert_eq!(help, None, "{source:?} failed with {message:?}");
    }
}

// -- Numbers and indexing --

#[test]
fn number_forms_frost_lacks() {
    assert_help(&[
        ("0xFF", "Frost number literals are decimal only"),
        ("0b101", "Frost number literals are decimal only"),
        ("0o17", "Frost number literals are decimal only"),
        ("1_000", "Frost number literals have no `_` separators"),
        ("1.", "a Float needs digits after its point, like `1.0`"),
        ("0XFF", "Frost number literals are decimal only"),
        ("0B1", "Frost number literals are decimal only"),
        ("0O17", "Frost number literals are decimal only"),
        ("1e", "an exponent needs digits, like `1e3`"),
        ("1e+", "an exponent needs digits, like `1e3`"),
        ("def x = 2E", "an exponent needs digits, like `2E3`"),
        ("1.5e", "an exponent needs digits, like `1.5e3`"),
    ]);
}

#[test]
fn tuple_style_indexing() {
    assert_help(&[
        ("t.0", "index an Array with brackets, like `xs[0]`"),
        ("xs.10", "index an Array with brackets, like `xs[0]`"),
        ("f().0", "index an Array with brackets, like `xs[0]`"),
    ]);
    // A version-like literal indexes nothing.
    assert_help_or_none(&[("def a = 1.2.3", None)]);
}

// `..` and `...` between operands, or before one in an expression, are a range habit.
#[test]
fn range_syntax() {
    assert_help(&[
        ("1..5", RANGE_HELP),
        ("def r = 1..10", RANGE_HELP),
        ("def r = 1..n", RANGE_HELP),
        ("0..=n", RANGE_HELP),
        ("a..b", RANGE_HELP),
        ("a .. b", RANGE_HELP),
        ("f(..x)", RANGE_HELP),
        ("print(1..3)", RANGE_HELP),
        ("def r = 1...5", RANGE_HELP),
        // A number after the dots makes a range, however they are spaced.
        ("1 ...5", RANGE_HELP),
        ("1... 5", RANGE_HELP),
        // An arm's result is an expression.
        ("match x { 1 => 1..3 }", RANGE_HELP),
    ]);
}

// In an index, a range is a slice.
#[test]
fn ranges_in_an_index() {
    assert_help(&[
        ("xs[1..5]", SLICE_HELP),
        ("xs[2..]", SLICE_HELP),
        ("xs[..3]", SLICE_HELP),
        ("xs[a..b]", SLICE_HELP),
    ]);
}

// A range where a pattern belongs is tested in a guard.
#[test]
fn ranges_as_patterns() {
    assert_help(&[
        ("match x { 1..3 => 2 }", PATTERN_RANGE_HELP),
        ("match x { 1...3 => 2 }", PATTERN_RANGE_HELP),
        ("match x { 1 ... 3 => 2 }", PATTERN_RANGE_HELP),
        ("match x { 1 .. 3 => 2 }", PATTERN_RANGE_HELP),
        ("match x { 1..=3 => 2 }", PATTERN_RANGE_HELP),
        ("match x { 'a'..'z' => 1 }", PATTERN_RANGE_HELP),
    ]);
    // Between names, the dots are no clear range.
    assert_help_or_none(&[("match x { a..b => 2 }", None)]);
}

// In an Array pattern, dots after a name are a rest binding's, which takes no guard.
#[test]
fn dots_after_a_name_in_an_array_pattern() {
    let after_comma = "a `...rest` binding follows a comma, as in `a, ...rest`";
    assert_help(&[
        ("def [a..b] = xs", after_comma),
        ("def [first..rest] = xs", after_comma),
        ("match x { [h..t] => 1 }", after_comma),
        ("match x { [a..] => 1 }", REST_HELP),
    ]);
}

// `..` beside a String is Lua's join.
#[test]
fn dots_joining_strings() {
    let help = "Frost joins Strings with `+`";
    assert_help(&[
        ("def s = 'a' .. 'b'", help),
        ("def s = 'a' .. name", help),
        ("def s = name .. 'b'", help),
    ]);
}

// -- Keywords where a name belongs --

#[test]
fn keywords_as_fields() {
    assert_help(&[
        (
            "xs.map(f)",
            "`map` is a keyword, not a method; write `map xs with f`, or `xs @ transform(f)`",
        ),
        (
            "xs.filter(f)",
            "`filter` is a keyword, not a method; write `filter xs with f`, or \
             `xs @ select(f)`",
        ),
        (
            "x.init",
            "`init` is a keyword; index with brackets instead: `x[\"init\"]`",
        ),
    ]);
}

#[test]
fn keywords_as_map_keys() {
    assert_help(&[
        (
            "{map: 1}",
            "`map` is a keyword; write the key in brackets: `[\"map\"]: ...`",
        ),
        (
            "{a: 1, if: 2}",
            "`if` is a keyword; write the key in brackets: `[\"if\"]: ...`",
        ),
        // The parser reads a Map after `->` when it starts `name:`.
        (
            "fn -> { a: 1, if: 2 }",
            "`if` is a keyword; write the key in brackets: `[\"if\"]: ...`",
        ),
        (
            "def {with: w} = m",
            "`with` is a keyword; write the key in brackets: `[\"with\"]: ...`",
        ),
    ]);
}

// A shorthand entry names a variable, which a keyword cannot.
#[test]
fn keywords_as_shorthand_map_entries() {
    assert_help(&[
        ("{map}", "`map` is a keyword, so it cannot be a name"),
        (
            "{filter, x}",
            "`filter` is a keyword, so it cannot be a name",
        ),
    ]);
}

#[test]
fn keywords_as_names() {
    assert_help(&[
        ("def map = 1", "`map` is a keyword, so it cannot be a name"),
        (
            "def init = 1",
            "`init` is a keyword, so it cannot be a name",
        ),
        (
            "defn with(x) -> x",
            "`with` is a keyword, so it cannot be a name",
        ),
        (
            "defn f(map) -> map",
            "`map` is a keyword, so it cannot be a name",
        ),
        (
            "defn f(a, do) -> a",
            "`do` is a keyword, so it cannot be a name",
        ),
        (
            "fn filter -> 1",
            "`filter` is a keyword, so it cannot be a name",
        ),
    ]);
}

#[test]
fn keywords_as_names_in_patterns_and_parameters() {
    assert_help(&[
        ("def {if} = m", "`if` is a keyword, so it cannot be a name"),
        (
            "def {a, map} = m",
            "`map` is a keyword, so it cannot be a name",
        ),
        (
            "def {a: if} = m",
            "`if` is a keyword, so it cannot be a name",
        ),
        ("def [if] = xs", "`if` is a keyword, so it cannot be a name"),
        (
            "def [a, [do]] = xs",
            "`do` is a keyword, so it cannot be a name",
        ),
        (
            "match x { [if] => 1 }",
            "`if` is a keyword, so it cannot be a name",
        ),
        (
            "match m { {a: with} => 1 }",
            "`with` is a keyword, so it cannot be a name",
        ),
        (
            "match x { map => 1 }",
            "`map` is a keyword, so it cannot be a name",
        ),
        (
            "def f = fn a, with -> 1",
            "`with` is a keyword, so it cannot be a name",
        ),
        (
            "fn a, b, if -> 1",
            "`if` is a keyword, so it cannot be a name",
        ),
    ]);
}

// A declaration word from another language is followed by a name, so a keyword there
// is a name too; both habits are named.
#[test]
fn keywords_as_names_after_declaration_words() {
    assert_help(&[
        (
            "var if = 1",
            "Frost has no `var`; bind with `def`, and `if` is a keyword, so it cannot be a name",
        ),
        (
            "let map = 1",
            "Frost has no `let`; bind with `def`, and `map` is a keyword, so it cannot be a name",
        ),
        (
            "const fn = 1",
            "Frost has no `const`; bind with `def`, and `fn` is a keyword, so it cannot be a name",
        ),
        (
            "function filter(x) {}",
            "`filter` is a keyword, so it cannot be a name",
        ),
    ]);
}

// -- Braces read as a Map --

#[test]
fn braces_meant_as_a_block() {
    let if_on_lines = r"
        if x: {
            y + 1
        }
    ";
    let def_on_lines = r"
        def a = {
            f(x)
        }
    ";
    let first_on_a_later_line = r"
        if x: {
            1
        }
    ";
    assert_help(&[
        ("if a: { def x = 1; x } else: 2", BLOCK_HELP),
        ("match x { _ => { def y = 1; y } }", BLOCK_HELP),
        ("def x = { 1 }", BLOCK_HELP),
        ("def x = { y + 1 }", BLOCK_HELP),
        ("def x = { f(y) }", BLOCK_HELP),
        ("{ y + 1 }", BLOCK_HELP),
        ("{ x.y }", BLOCK_HELP),
        ("def x = { y + if c: 1 else: 2 }", BLOCK_HELP),
        ("if a: { if b: 1 else: 2 }", BLOCK_HELP),
        (if_on_lines, BLOCK_HELP),
        (def_on_lines, BLOCK_HELP),
        (first_on_a_later_line, BLOCK_HELP),
    ]);
}

// `{ y = 1 }` in a branch is a block with an assignment; elsewhere, a Map entry.
#[test]
fn assignment_in_braces_meant_as_a_block() {
    assert_help(&[
        ("if x: { y = 1 }", BLOCK_HELP),
        ("if x: 0 else: { y = 1 }", BLOCK_HELP),
        ("match v { 1 => { x = 1 } }", BLOCK_HELP),
    ]);
}

// Braces where a block could not stand, or holding Map-shaped contents, get no block help.
#[test]
fn braces_not_meant_as_a_block() {
    // Map-shaped contents get the help for their Map habit; see the tests below.
    // Elsewhere, a block could not stand or would not be the likely intent.
    assert_help_or_none(&[
        ("f({ x + 1 })", None),
        ("[{ 1 }]", None),
        ("{a: { 1 }}", None),
        ("{a: 1 b: 2}", None),
        ("{ $0 + 1 }", Some(PLACEHOLDER_HELP)),
        // A guard's `if:` does not start a branch.
        ("match x { n if: {a = 1} => 2 }", Some(MAP_ENTRY_HELP)),
    ]);
}

// An expression before a `:` in braces is a computed key missing its brackets.
#[test]
fn computed_keys_without_brackets() {
    assert_help(&[
        (
            "def a = {a.b: 1}",
            "a computed key goes in brackets, like `{[a.b]: ...}`",
        ),
        (
            "{x + y: 1}",
            "a computed key goes in brackets, like `{[x + y]: ...}`",
        ),
        (
            "{ f(x): 1 }",
            "a computed key goes in brackets, like `{[f(x)]: ...}`",
        ),
    ]);
}

#[test]
fn set_literals() {
    let help = "Frost has no set literal; use an Array, like `[1, 2, 3]`";
    assert_help(&[("def t = {1, 2, 3}", help), ("{'a', 'b'}", help)]);
}

#[test]
fn map_entries_from_other_languages() {
    let multiline = r"
        def m = {
            a => 1
        }
    ";
    assert_help(&[
        ("{:a => 1}", MAP_ENTRY_HELP),
        ("def x = {a => 1}", MAP_ENTRY_HELP),
        ("{a: 1, b => 2}", MAP_ENTRY_HELP),
        (multiline, MAP_ENTRY_HELP),
        // A literal key is written as it would be before a `:`.
        (
            r#"{"a" => 1}"#,
            "a Map key that is a name needs no quotes, like `{a: ...}`",
        ),
        (
            r#"match x { 1 => {"a" => 1} }"#,
            "a Map key that is a name needs no quotes, like `{a: ...}`",
        ),
        (
            "{1 => 'x'}",
            "a Map key that is not a name goes in brackets: `{[1]: ...}`",
        ),
    ]);
}

// In a `match`, a line starting `key => value` after an unclosed Map may be the next arm.
#[test]
fn an_arm_after_an_unclosed_map_gets_no_map_entry_help() {
    let literal_pattern = r"
        match x {
            1 => {a: 1,
            3 => 4
        }
    ";
    let name_pattern = r"
        match x {
            1 => {a: 1,
            b => 4
        }
    ";
    assert_help_or_none(&[(literal_pattern, None), (name_pattern, None)]);
}

#[test]
fn other_map_habits() {
    assert_help(&[
        (
            "{**m, a: 1}",
            "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`",
        ),
        ("def m = {,}", "an empty Map is written `{}`"),
        // After `do`, braces are a block's.
        (
            r#"def x = do { "a": 1 }"#,
            "a `do` block holds statements; a Map is written without `do`, like `{a: ...}`",
        ),
        (
            "do { 'k': 1 }",
            "a `do` block holds statements; a Map is written without `do`, like `{k: ...}`",
        ),
        (
            "do { 'a b': 1 }",
            "a `do` block holds statements; a Map is written without `do`, like \
             `{['a b']: ...}`",
        ),
    ]);
}

#[test]
fn quoted_map_keys() {
    let multiline_block = r#"
        fn x -> {
            "k": x
        }
    "#;
    let multiline_map = r#"
        if x: {
            "a": 1
        }
    "#;
    let colon_on_its_own_line = r#"
        def m = {
            "a"
            : 1
        }
    "#;
    let block_colon_on_its_own_line = r#"
        fn -> {
            "a"
            : 1
        }
    "#;
    let not_a_name_on_its_own_line = r#"
        def m = {
            "a b"
            : 1
        }
    "#;
    let name_help =
        |name: &str| format!("a Map key that is a name needs no quotes, like `{{{name}: ...}}`");
    let bracket_help =
        |key: &str| format!("a Map key that is not a name goes in brackets: `{{[{key}]: ...}}`");
    let cases = [
        // JSON's quoted names
        (r#"{"a": 1}"#, name_help("a")),
        (r#"{"name": "x"}"#, name_help("name")),
        ("{'a': 1}", name_help("a")),
        ("{b: 1, 'a': 2}", name_help("a")),
        (r#"fn -> { "a": 1 }"#, name_help("a")),
        ("match x { {'a': v} => v }", name_help("a")),
        ("def {'a': v} = m", name_help("a")),
        (r#"fn -> { a: 1, "b": 2 }"#, name_help("b")),
        (multiline_block, name_help("k")),
        (multiline_map, name_help("a")),
        (colon_on_its_own_line, name_help("a")),
        (block_colon_on_its_own_line, name_help("a")),
        // A String that is no name, or is a keyword, keeps its quotes in brackets.
        ("{'a b': 1}", bracket_help("'a b'")),
        (r#"{"if": 1}"#, bracket_help(r#""if""#)),
        ("{'1a': 1}", bracket_help("'1a'")),
        ("{1: 2}", bracket_help("1")),
        (r#"fn -> { "a-b": 1 }"#, bracket_help(r#""a-b""#)),
        (not_a_name_on_its_own_line, bracket_help(r#""a b""#)),
    ];
    for (source, help) in cases {
        assert_help(&[(source, &help)]);
    }
}

#[test]
fn literal_map_keys_of_every_kind() {
    for (source, key) in [
        ("{-1: 2}", "-1"),
        ("{-1.5: 2}", "-1.5"),
        ("{1.5: 2}", "1.5"),
        ("{x'00': 1}", "x'00'"),
        ("{R'(a)': 1}", "R'(a)'"),
        ("{$'a${b}': 1}", "$'a${b}'"),
        ("{true: 1}", "true"),
        ("{a: 1, false: 2}", "false"),
        ("fn -> { -1: 2 }", "-1"),
        ("def {'a b': v} = m", "'a b'"),
    ] {
        let (message, help) = diagnosis(source);
        assert_eq!(
            help,
            Some(format!(
                "a Map key that is not a name goes in brackets: `{{[{key}]: ...}}`"
            )),
            "{source:?} failed with {message:?}"
        );
    }
}

// An interpolation is lexed apart, but the hints still see the brackets opened in it.
#[test]
fn braces_inside_an_interpolation() {
    assert_help(&[(
        r#"$'${ {"a": 1} }'"#,
        "a Map key that is a name needs no quotes, like `{a: ...}`",
    )]);
    // An interpolation holds an expression, not statements.
    assert_help_or_none(&[("$'${x = 1}'", None)]);
}

// The parser tries a `{ [` lambda body as a Map, then backtracks to read a block;
// the hints see only the block.
#[test]
fn a_body_read_as_a_block_after_trying_a_map() {
    assert_help_or_none(&[
        ("fn -> { [k]: 1, b = 2 }", None),
        (
            "fn -> { [k] + 1; x = 2 }",
            Some("Frost has no assignment; bind a new name with `def`: `def x = ...`"),
        ),
    ]);
}

// -- Patterns --

#[test]
fn type_names() {
    let all_types = "the types are `Null`, `Int`, `Float`, `Bool`, `String`, `Bytes`, \
                     `Array`, `Map`, `Function`, `Opaque`, `Primitive`, `Numeric`, \
                     `Structured`, `Flat`, `Nonnull`";
    assert_help(&[
        ("match x { n is int => n }", "did you mean `Int`?"),
        ("match x { n is STRING => n }", "did you mean `String`?"),
        ("match x { n is nonnull => n }", "did you mean `Nonnull`?"),
        ("match x { n is Number => n }", all_types),
    ]);
}

#[test]
fn rest_bindings() {
    let last_help = "a `...rest` binding comes last, with nothing after it";
    assert_help(&[
        ("def [a, ...rest, b] = xs", last_help),
        ("match x { [...a, b] => b }", last_help),
        ("defn f(...r, x) -> r", last_help),
        ("fn (...r,) -> 1", last_help),
        (
            "match x { [a, ..rest] => a }",
            "a rest binding is written `...name`",
        ),
        ("def [a, ..r] = xs", "a rest binding is written `...name`"),
        ("fn ..r -> r", "a rest binding is written `...name`"),
        // Rust's pattern for the elements not named
        (
            "match x { [a, ..] => a }",
            "a rest binding is written `...name`",
        ),
    ]);
}

// A `...` touching only the name after it, as a rest binding does, is missing its comma.
#[test]
fn a_rest_binding_without_its_comma() {
    let help = "a `...rest` binding follows a comma, as in `a, ...rest`";
    assert_help(&[
        ("match x { [a ...rest] => 1 }", help),
        ("def [a ...rest] = xs", help),
        ("fn a ...b -> a", help),
        ("defn f(a ...b) -> a", help),
    ]);
}

#[test]
fn rest_bindings_outside_array_patterns() {
    let help = "only Array patterns take a rest binding: `[a, ...rest]`";
    assert_help(&[
        ("match x { ...a => 2 }", help),
        ("match x { 1 | ...a => 2 }", help),
    ]);
}

// A Map pattern matches a Map with other keys, so it needs no rest in any spelling.
#[test]
fn rest_bindings_in_map_patterns() {
    assert_help(&[
        ("def {a, ...r} = m", MAP_REST_HELP),
        ("match v { {a, ...r} => 1 }", MAP_REST_HELP),
        ("match v { {a: 1, ...} => 1 }", MAP_REST_HELP),
        ("def {a, ...} = m", MAP_REST_HELP),
        ("def {a, ..} = m", MAP_REST_HELP),
        ("match x { {a, ..} => 1 }", MAP_REST_HELP),
        ("def {a, **r} = m", MAP_REST_HELP),
    ]);
}

#[test]
fn spread() {
    assert_help(&[
        (
            "[...xs, 4]",
            "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`",
        ),
        (
            "{...m, k: 1}",
            "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`",
        ),
        (
            "match x { 1 => [...xs] }",
            "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`",
        ),
        (
            "f(...args)",
            "Frost has no spread; to pass an Array's elements as arguments, use `call(f, args)`",
        ),
    ]);
}

// A `...` with nothing after it to spread is an ellipsis standing for code left out.
#[test]
fn ellipses_spreading_nothing() {
    assert_help_or_none(&[
        ("x...", None),
        ("...", None),
        ("fn -> ...", None),
        ("def x = ...", None),
        ("[...]", None),
        ("f(a, ...)", None),
    ]);
}

// A `...` touching only what follows it is a spread missing its comma, not a range.
#[test]
fn a_spread_without_its_comma() {
    assert_help(&[
        (
            "f(a ...b)",
            "Frost has no spread; to pass an Array's elements as arguments, use `call(f, args)`",
        ),
        (
            "def v = [xs ...ys]",
            "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`",
        ),
        (
            "{a: 1 ...m}",
            "Frost has no spread; combine with `+`, like `xs + ys` or `m + {k: v}`",
        ),
    ]);
}

// -- Placeholders --

#[test]
fn placeholders() {
    let names_help = "the placeholders are `$`, `$1` to `$9`, and `$$`";
    assert_help(&[
        ("fn x -> $ * 2", PLACEHOLDER_HELP),
        ("def $ = 1", PLACEHOLDER_HELP),
        ("$($0)", names_help),
        ("$($10)", names_help),
        (
            "print $ 1 + 2",
            "Frost has no `$` operator; call a function with parentheses: `f(x)`",
        ),
        // A name with another language's sigil
        ("$x = 1", "Frost names have no `$` sigil; write `x`"),
        ("$a", "Frost names have no `$` sigil; write `a`"),
        // `_` is no name to read.
        ("$_", PLACEHOLDER_HELP),
    ]);
}

// Inside `$( ... )`, a placeholder is fine; something else, such as an operator, is missing.
#[test]
fn placeholders_inside_an_abbreviated_lambda_get_no_help() {
    assert_help_or_none(&[
        ("$($1 $2)", None),
        ("$(1 $ 2)", None),
        ("$($ + $$ $)", None),
        ("$($'${$ $}')", None),
        // `${` is a format String habit wherever it stands.
        (
            "$(f($ ${x}))",
            Some("a format String is written `$'...${x}...'`"),
        ),
    ]);
}

// -- No hint --
// Errors that match no habit get no help.

#[test]
fn ordinary_errors_have_no_help() {
    let sources = [
        "[1 2]",
        "1 +",
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
