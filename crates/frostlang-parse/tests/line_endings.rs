//! A CRLF line break is a line break exactly like LF: a script parses to the
//! same tree, and fails with the same error, whichever line ending it was saved with.
//! A lone CR is not a line break.

mod helpers;

use frostlang_parse::ast::*;
use frostlang_parse::parse_program;
use helpers::*;

/// `source` with every LF line break turned into CRLF.
fn crlf(source: &str) -> String {
    source.replace('\n', "\r\n")
}

/// The text of a String literal expression.
fn str_text(expr: &Spanned<Expr>) -> &str {
    match &expr.node {
        Expr::Literal(Literal::String(s)) => s,
        other => panic!("expected String literal, got {other:?}"),
    }
}

/// Programs that span lines in every way the grammar allows.
const MULTILINE_PROGRAMS: &[&str] = &[
    // Statements, blank lines, and comments.
    r"
        # a comment
        def a = 1

        def b = 2 # trailing
        a + b
    ",
    // Blocks and bodies on the next line.
    r"
        defn f(x) ->
            x + 1
        def g = fn x -> {
            def y = x
            y
        }
        do {
            f(1)
            g(2)
        }
    ",
    // Delimited lists across lines.
    r"
        def m = {
            a: 1,
            b,
        }
        def xs = [
            1,
            2,
        ]
        f(
            xs,
            m,
        )
    ",
    // Continuation lines.
    r"
        xs
            .field
            @ f()
            @ g(1)
    ",
    // Control flow.
    r"
        if a: 1
        elif b: 2
        else: 3
        match x {
            1 => 'one',
            [a, ...rest] => a,
            {k} as m => m,
            _ => null,
        }
        map xs with fn x ->
            x * 2
    ",
    // Multiline Strings: indentation, a blank line, escapes, and the empty String.
    r#"
        def s = """
            first
              indented

            last\t\\
            """
        def empty = """
            """
        def single = '''
            quoted
            '''
    "#,
];

#[test]
fn crlf_programs_parse_like_lf_programs() {
    for source in MULTILINE_PROGRAMS {
        assert_eq!(
            parse(&crlf(source)),
            parse(source),
            "CRLF and LF differ for:\n{source}"
        );
    }
}

#[test]
fn crlf_in_a_multiline_string_becomes_lf() {
    let source = r#"
        """
            hello
            world
            """
    "#;
    let expr = parse_expr(&crlf(source));
    assert_eq!(str_text(&expr), "hello\nworld");
}

/// Programs that fail because of where a line break falls.
const LINE_BREAK_ERRORS: &[&str] = &[
    // A line break ends a statement mid-expression.
    r"
        def x = 1 +
        2
    ",
    // Single-line String forms cannot span lines.
    r"
        'a
        b'
    ",
    r#"
        "a
        b"
    "#,
    "'a\\\nb'",
    r"
        $'a
        b'
    ",
    r"
        R'(a
        b)'
    ",
    // A multiline String's lines are checked against its closing delimiter.
    r#"
        """hello
        """
    "#,
    r#"
        """
        hello"""
    "#,
    r#"
        """
        oops
          """
    "#,
];

#[test]
fn crlf_programs_fail_like_lf_programs() {
    for source in LINE_BREAK_ERRORS {
        let lf = parse_program("test.frst", source).expect_err(source);
        let crlf_source = crlf(source);
        let crlf = parse_program("test.frst", &crlf_source).expect_err(&crlf_source);
        assert_eq!(
            crlf.message(),
            lf.message(),
            "CRLF and LF fail differently for {source:?}"
        );
    }
}

// A CR not followed by LF is ordinary text inside a quoted String.
#[test]
fn lone_cr_in_a_quoted_string_is_kept() {
    assert_eq!(str_text(&parse_expr("'a\rb'")), "a\rb");
    assert_eq!(str_text(&parse_expr("\"a\rb\"")), "a\rb");
}

// A CR not followed by LF is ordinary text inside a multiline String.
#[test]
fn lone_cr_in_a_multiline_string_is_kept() {
    let expr = parse_expr("\"\"\"\na\rb\n\"\"\"");
    assert_eq!(str_text(&expr), "a\rb");
}

// A CR right before a CRLF line break is text, and the CRLF is the line break.
#[test]
fn cr_before_a_crlf_line_break_is_kept() {
    let expr = parse_expr("\"\"\"\r\na\r\r\nb\r\n\"\"\"");
    assert_eq!(str_text(&expr), "a\r\nb");
}

// -- Escapes in multiline Strings --
// A line loses its indentation before its escapes are decoded, so an escape at a
// line's start or end decodes the same under either line ending.

#[test]
fn escapes_at_line_starts_and_ends_decode_under_either_line_ending() {
    let source = r"
        '''
            \tstart
            end\\
            letter\u{41}
              \tdeeper
            '''
    ";
    let expected = "\tstart\nend\\\nletterA\n  \tdeeper";
    for source in [source.to_owned(), crlf(source)] {
        assert_eq!(str_text(&parse_expr(&source)), expected, "{source:?}");
    }
}

// An escape is text, not indentation, even when it decodes to a tab.
#[test]
fn an_escape_inside_the_indentation_is_under_indented() {
    let source = r"
        '''
            ok
          \t  x
            '''
    ";
    for source in [source.to_owned(), crlf(source)] {
        let err = parse_program("test.frst", &source).expect_err(&source);
        assert_eq!(
            err.message(),
            "multiline String content is indented less than the closing delimiter",
            "{source:?}"
        );
    }
}

#[test]
fn a_lone_cr_beside_escapes_in_an_indented_multiline_string_is_kept() {
    let source = "'''\n    a\rb\\t\n    \\\\\rc\n    '''";
    for source in [source.to_owned(), crlf(source)] {
        assert_eq!(
            str_text(&parse_expr(&source)),
            "a\rb\t\n\\\rc",
            "{source:?}"
        );
    }
}
