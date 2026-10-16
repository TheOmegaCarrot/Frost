//! Where a line break may fall around a `do` block's `{`, a lambda's `->`, and a `:`.
//!
//! A line starting with `->` continues the line before, wherever it is, as one starting
//! with `.` or `@` does. A `do` block's `{` may start a later line only inside brackets.
//! A `:` never starts a line: one after a line break is an error, inside brackets too.

use frostlang_parse::parse_program;

use crate::helpers::*;

/// Asserts `multiline` parses to the same program as `one_line`.
fn assert_same_as_one_line(multiline: &str, one_line: &str) {
    assert_eq!(
        parse(multiline),
        parse(one_line),
        "expected the same program as {one_line:?} for:\n{multiline}"
    );
}

// ============================================================
// A `do` block's `{` on the line after `do`
// ============================================================

mod do_brace {
    use super::*;

    #[test]
    fn may_start_a_later_line_inside_each_kind_of_bracket() {
        // (the layout, the same on one line)
        let cases = [
            (
                r"
                (do
                    { 1 })
                ",
                "(do { 1 })",
            ),
            (
                r"
                [do
                    { 1 }]
                ",
                "[do { 1 }]",
            ),
            (
                r"
                f(do
                    { 1 })
                ",
                "f(do { 1 })",
            ),
            (
                r"
                {a: do
                    { 1 }}
                ",
                "{a: do { 1 }}",
            ),
            (
                r"
                xs[do
                    { 1 }]
                ",
                "xs[do { 1 }]",
            ),
            (
                r"
                $(do
                    { $ })
                ",
                "$(do { $ })",
            ),
            (
                r"
                match x {
                    _ => do
                        { 1 },
                }
                ",
                "match x { _ => do { 1 } }",
            ),
            (
                r"
                (do # a note

                    { 1 })
                ",
                "(do { 1 })",
            ),
        ];
        for (multiline, one_line) in cases {
            assert_same_as_one_line(multiline, one_line);
        }
    }

    #[test]
    fn may_not_start_a_later_line_outside_brackets() {
        let at_top_level = r"
            def x = do
            {
                1
            }
        ";
        let in_a_block = r"
            fn -> {
                def x = do
                { 1 }
                x
            }
        ";
        // A block's line breaks are significant, even inside brackets.
        let in_a_block_in_parentheses = r"
            (fn -> {
                def x = do
                { 1 }
                x
            })
        ";
        for source in [at_top_level, in_a_block, in_a_block_in_parentheses] {
            assert_eq!(
                parse_err_message(source),
                "expected `{`, but found a line break",
                "for:\n{source}"
            );
        }
    }
}

// ============================================================
// A lambda's `->` at the start of a line
// ============================================================

mod arrow_continuation {
    use super::*;

    /// Every lambda head, with a body for it.
    const LAMBDAS: [(&str, &str); 10] = [
        ("fn", "1"),
        ("fn x", "x"),
        ("fn _", "1"),
        ("fn x, y", "x + y"),
        ("fn x, ...r", "r"),
        ("fn ...r", "r"),
        ("fn ()", "1"),
        ("fn (a, b)", "a + b"),
        ("fn name(a)", "a"),
        ("fn name()", "1"),
    ];

    /// Asserts each lambda in [`LAMBDAS`], put in `layout`, parses with its `->` on
    /// the next line as with its `->` on its line.
    fn assert_arrow_continues(layout: impl Fn(&str) -> String) {
        for (head, body) in LAMBDAS {
            let multiline = layout(&format!(
                r"{head}
                -> {body}"
            ));
            let one_line = layout(&format!("{head} -> {body}"));
            assert_same_as_one_line(&multiline, &one_line);
        }
    }

    #[test]
    fn at_top_level() {
        assert_arrow_continues(|lambda| {
            format!(
                r"
                def f = {lambda}
                f
                "
            )
        });
    }

    #[test]
    fn inside_parentheses() {
        assert_arrow_continues(|lambda| {
            format!(
                r"
                def f = ({lambda})
                f
                "
            )
        });
    }

    #[test]
    fn as_an_argument() {
        assert_arrow_continues(|lambda| format!("xs @ transform({lambda})"));
    }

    #[test]
    fn inside_a_block() {
        assert_arrow_continues(|lambda| {
            format!(
                r"
                def g = fn -> {{
                    def f = {lambda}
                    f
                }}
                "
            )
        });
    }

    #[test]
    fn inside_a_block_inside_parentheses() {
        assert_arrow_continues(|lambda| {
            format!(
                r"
                (fn -> {{
                    def f = {lambda}
                    f
                }})
                "
            )
        });
    }

    #[test]
    fn for_a_defn() {
        let multiline = r"
            defn f(a)
                -> a
            export defn g(a, ...r)
                -> r
        ";
        let one_line = r"
            defn f(a) -> a
            export defn g(a, ...r) -> r
        ";
        assert_same_as_one_line(multiline, one_line);
    }

    #[test]
    fn for_a_defn_in_a_block() {
        let multiline = r"
            def h = fn -> {
                defn f(a)
                    -> a
                f
            }
        ";
        let one_line = r"
            def h = fn -> {
                defn f(a) -> a
                f
            }
        ";
        assert_same_as_one_line(multiline, one_line);
    }

    // With no parameters, `fn` then `->` on the next line reads as the two joined.
    #[test]
    fn after_a_bare_fn() {
        let source = r"
            def f = fn
                -> 1
        ";
        assert_same_as_one_line(source, "def f = fn -> 1");
    }

    // As with a leading `.`, blank lines and comment lines may come between.
    #[test]
    fn after_blank_lines_and_comments() {
        let multiline = r"
            def f = fn x # the parameter

                # the body
                -> x
        ";
        assert_same_as_one_line(multiline, "def f = fn x -> x");
    }

    #[test]
    fn for_each_lambda_of_a_curried_one() {
        let multiline = r"
            def add = fn x
                -> fn y
                -> x + y
        ";
        assert_same_as_one_line(multiline, "def add = fn x -> fn y -> x + y");
    }

    #[test]
    fn with_its_body_on_a_later_line_still() {
        let multiline = r"
            def f = fn x
                ->
                    x
        ";
        assert_same_as_one_line(multiline, "def f = fn x -> x");
    }

    // The lambda still ends at the end of its body's line.
    #[test]
    fn the_lambda_still_ends_at_its_body_line() {
        let source = r"
            def f = fn x
                -> x
            f(1)
        ";
        assert_eq!(parse(source).statements.len(), 2);
    }

    // Only `->` continues: a line break elsewhere in a lambda's head is still an
    // error, outside brackets and in them.
    #[test]
    fn other_breaks_in_a_lambda_head_are_still_errors() {
        // (source, the error's message)
        let cases = [
            (
                r"
                def f = fn
                    x -> x
                ",
                "expected parameters or `->`, but found a line break",
            ),
            (
                r"
                (fn
                    x -> x)
                ",
                "expected parameters or `->`, but found a line break",
            ),
            (
                r"
                (fn x,
                    y -> x)
                ",
                "expected a parameter name, but found a line break",
            ),
            (
                r"
                (fn x
                    , y -> x)
                ",
                "expected `->`, but found a line break",
            ),
            (
                r"
                (fn ...
                    r -> r)
                ",
                "expected a name after `...`, but found a line break",
            ),
            (
                r"
                (fn name
                    (a) -> a)
                ",
                "expected `->`, but found a line break",
            ),
        ];
        for (source, message) in cases {
            assert_eq!(parse_err_message(source), message, "for:\n{source}");
        }
    }

    // A `->` cannot begin a statement, so one after a complete statement is an error,
    // not a lambda.
    #[test]
    fn an_arrow_after_a_complete_statement_is_an_error() {
        let source = r"
            def f = 1
            -> 2
        ";
        assert_eq!(
            parse_err_message(source),
            "expected an expression, but found `->`"
        );
    }
}

// ============================================================
// A `:` at the start of a line
// ============================================================

mod colon_on_a_later_line {
    use super::*;

    const HELP: &str = "a `:` must stay on the line of what comes before it";

    /// Asserts `source` fails at a `:` on a later line than what comes before it,
    /// `before`, with the error labeling the line break right after `before`.
    fn assert_rejected(source: &str, before: &str) {
        let err = parse_program("test.frst", source).expect_err(source);
        assert_eq!(
            err.message(),
            "expected `:`, but found a line break",
            "for:\n{source}"
        );
        assert_eq!(err.help(), Some(HELP), "for:\n{source}");
        let [label] = err.labels() else {
            panic!("expected one label, got {:?} for:\n{source}", err.labels());
        };
        let (start, end) = (label.span.start, label.span.end);
        assert_eq!(&source[start..end], "\n", "for:\n{source}");
        assert!(
            source[..start].ends_with(before),
            "expected the line break after {before:?}, but it is after {:?} in:\n{source}",
            &source[..start],
        );
        assert_eq!(label.text, "unexpected", "for:\n{source}");
    }

    #[test]
    fn after_an_if_condition() {
        let at_top_level = r"
            if c
                : 1
        ";
        let in_parentheses = r"
            (if c
                : 1)
        ";
        let after_a_condition_across_lines = r"
            (if a
                and b
                : 1)
        ";
        assert_rejected(at_top_level, "if c");
        assert_rejected(in_parentheses, "(if c");
        assert_rejected(after_a_condition_across_lines, "and b");
    }

    #[test]
    fn after_an_elif_condition() {
        let at_top_level = r"
            if a: 1
            elif b
                : 2
        ";
        let in_parentheses = r"
            (if a: 1
                elif b
                : 2)
        ";
        assert_rejected(at_top_level, "elif b");
        assert_rejected(in_parentheses, "elif b");
    }

    #[test]
    fn after_else() {
        let at_top_level = r"
            if a: 1 else
                : 2
        ";
        let in_parentheses = r"
            (if a: 1 else
                : 2)
        ";
        let on_its_own_line_in_brackets = r"
            [
                if a: 1
                else
                : 2
            ]
        ";
        assert_rejected(at_top_level, "else");
        assert_rejected(in_parentheses, "else");
        assert_rejected(on_its_own_line_in_brackets, "else");
    }

    #[test]
    fn after_a_guard_if() {
        let guard = r"
            match x {
                n if
                    : n > 0 => n,
                _ => 0,
            }
        ";
        let after_a_type_test = r"
            match x {
                n is Int if
                    : n > 0 => n,
                _ => 0,
            }
        ";
        assert_rejected(guard, "n if");
        assert_rejected(after_a_type_test, "Int if");
    }

    #[test]
    fn after_init() {
        let at_top_level = r"
            reduce xs init
                : 0 with f
        ";
        let in_parentheses = r"
            (reduce xs init
                : 0 with f)
        ";
        assert_rejected(at_top_level, "init");
        assert_rejected(in_parentheses, "init");
    }

    #[test]
    fn after_a_map_literal_key() {
        let named = r"
            def m = {
                a
                    : 1
            }
        ";
        let computed = r"
            def m = {
                [k]
                    : 1
            }
        ";
        let named_after_another_entry = r"
            def m = {b: 2, a
                : 1}
        ";
        let in_a_lambda_body = r"
            fn -> {a
                : 1}
        ";
        assert_rejected(named, "a");
        assert_rejected(computed, "[k]");
        assert_rejected(named_after_another_entry, "a");
        assert_rejected(in_a_lambda_body, "{a");
    }

    #[test]
    fn after_a_map_pattern_key() {
        let named = r"
            def {
                a
                    : b
            } = m
        ";
        let computed = r"
            def {
                [k]
                    : b
            } = m
        ";
        let named_in_an_arm = r"
            match m {
                {a
                    : b} => b,
            }
        ";
        let computed_in_an_arm = r"
            match m {
                {[k]
                    : b} => b,
            }
        ";
        assert_rejected(named, "a");
        assert_rejected(computed, "[k]");
        assert_rejected(named_in_an_arm, "{a");
        assert_rejected(computed_in_an_arm, "{[k]");
    }

    // The line break labeled is the first after what comes before the `:`.
    #[test]
    fn after_blank_lines_and_a_comment() {
        let source = r"
            (if c # a note

                : 1)
        ";
        assert_rejected(source, "# a note");
    }

    // A block's line breaks are significant, so this `if` is in no bracket.
    #[test]
    fn in_a_block_inside_parentheses() {
        let source = r"
            (fn -> {
                if c
                    : 1
            })
        ";
        assert_rejected(source, "if c");
    }

    #[test]
    fn a_line_break_after_a_colon_is_still_accepted() {
        let multiline = r"
            def a = if c:
                1
            elif d:
                2
            else:
                3
            def b = (if c:
                1
                elif d:
                2
                else:
                3)
            def t = reduce xs init:
                0 with f
            def m = {
                k:
                    1,
                [k]:
                    2,
            }
            def {
                p:
                    q,
                [k]:
                    r,
            } = m
            match x {
                n if:
                    n > 0 => n,
                _ => 0,
            }
        ";
        let one_line = r"
            def a = if c: 1 elif d: 2 else: 3
            def b = (if c: 1 elif d: 2 else: 3)
            def t = reduce xs init: 0 with f
            def m = {k: 1, [k]: 2}
            def {p: q, [k]: r} = m
            match x { n if: n > 0 => n, _ => 0 }
        ";
        assert_same_as_one_line(multiline, one_line);
    }

    #[test]
    fn a_line_break_before_elif_else_or_init_is_still_accepted() {
        let multiline = r"
            def a = if c: 1
            elif d: 2
            else: 3
            def b = (if c: 1
                elif d: 2
                else: 3)
            def t = reduce xs
                init: 0
                with f
            def u = (reduce xs
                init: 0
                with f)
        ";
        let one_line = r"
            def a = if c: 1 elif d: 2 else: 3
            def b = (if c: 1 elif d: 2 else: 3)
            def t = reduce xs init: 0 with f
            def u = (reduce xs init: 0 with f)
        ";
        assert_same_as_one_line(multiline, one_line);
    }
}
