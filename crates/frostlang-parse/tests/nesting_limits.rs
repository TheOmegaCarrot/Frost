//! [`ParseLimits`]: how each nesting shape counts against the recursion and AST depth
//! limits, where the parse stops, and what the error says.
//!
//! Each shape is pinned at its boundary: the deepest nesting the limit accepts, and one
//! level more, which it rejects.

use std::num::NonZeroU64;

use crate::helpers::{RECURSION_MESSAGE, nest};

use frostlang_parse::{ParseLimits, parse_program, parse_program_with_limits};

const AST_DEPTH_MESSAGE: &str = "expression too deep";

/// A recursion limit of `max`, and no AST depth limit.
fn recursion_limit(max: u64) -> ParseLimits {
    ParseLimits::UNLIMITED.with_max_recursion_depth(NonZeroU64::new(max))
}

/// An AST depth limit of `max`, and no recursion limit.
fn ast_depth_limit(max: u64) -> ParseLimits {
    ParseLimits::UNLIMITED.with_max_ast_depth(NonZeroU64::new(max))
}

/// The message `src` fails to parse with under `limits`, or `None` if it parses.
fn failure(src: &str, limits: ParseLimits) -> Option<String> {
    parse_program_with_limits("test.frst", src, limits)
        .err()
        .map(|err| err.message().to_owned())
}

/// Asserts that `shape(deepest)` parses under `limits` and `shape(deepest + 1)` fails with
/// a message starting with `message`.
#[track_caller]
fn assert_boundary(
    name: &str,
    shape: impl Fn(usize) -> String,
    deepest: usize,
    limits: ParseLimits,
    message: &str,
) {
    let at = shape(deepest);
    assert_eq!(
        failure(&at, limits),
        None,
        "{name}: {deepest} levels should be within the limit:\n{at}"
    );
    let past = shape(deepest + 1);
    let failed = failure(&past, limits).unwrap_or_else(|| {
        panic!(
            "{name}: {} levels should pass the limit:\n{past}",
            deepest + 1
        )
    });
    assert!(
        failed.starts_with(message),
        "{name}: expected a failure starting {message:?}, got {failed:?}"
    );
}

/// A source nested as many levels as it is given.
type Shape = fn(usize) -> String;

// -- Recursion --

#[test]
fn each_level_of_nesting_counts_against_the_recursion_limit() {
    const MAX: u64 = 20;
    // (shape, a source nested `n` levels, the deepest `n` the limit accepts)
    // The top-level expression is a level itself, so most shapes reach `MAX - 1`.
    let cases: [(&str, Shape, usize); 14] = [
        ("parentheses", |n| nest("(", "1", ")", n), 19),
        ("Array literal", |n| nest("[", "1", "]", n), 19),
        ("Map literal", |n| nest("{a: ", "1", "}", n), 19),
        ("call argument", |n| nest("f(", "1", ")", n), 19),
        ("prefix `-`", |n| format!("{}1", "-".repeat(n)), 19),
        ("prefix `not`", |n| format!("{}true", "not ".repeat(n)), 19),
        ("`if` branch", |n| format!("{}1", "if c: ".repeat(n)), 19),
        // The `if`, then each `elif` within the one before, then the `else` branch.
        (
            "`elif`",
            |n| format!("if c: 0 {}else: 0", "elif c: 0 ".repeat(n)),
            18,
        ),
        ("lambda body", |n| format!("{}1", "fn -> ".repeat(n)), 19),
        ("`do` block", |n| nest("do { ", "1", " }", n), 19),
        ("`match` arm", |n| nest("match c { _ => ", "1", " }", n), 19),
        // The `match`, then its arm's pattern.
        (
            "Array pattern in a `match`",
            |n| format!("match c {{ {} => 0 }}", nest("[", "a", "]", n)),
            18,
        ),
        (
            "Map pattern in a `def`",
            |n| format!("def {} = c", nest("{a: ", "b", "}", n)),
            19,
        ),
        (
            "format String interpolation",
            |n| nest("$'${", "1", "}'", n),
            19,
        ),
    ];
    for (name, shape, deepest) in cases {
        assert_boundary(
            name,
            shape,
            deepest,
            recursion_limit(MAX),
            RECURSION_MESSAGE,
        );
    }
}

#[test]
fn a_chain_is_built_without_recursing() {
    // Each link is parsed in a loop; only its right operand recurses, one level.
    let limits = recursion_limit(2);
    let cases = [
        ("`+`", format!("1{}", " + 1".repeat(1000))),
        ("`and`", format!("true{}", " and true".repeat(1000))),
        ("`.`", format!("m{}", ".a".repeat(1000))),
        ("index", format!("m{}", "[0]".repeat(1000))),
        ("call", format!("f{}", "()".repeat(1000))),
        ("`@`", format!("1{}", " @ f()".repeat(1000))),
    ];
    for (name, src) in cases {
        assert_eq!(failure(&src, limits), None, "a chain of {name}");
    }
}

// -- AST depth --

#[test]
fn each_link_of_a_chain_counts_against_the_ast_depth_limit() {
    const MAX: u64 = 20;
    // Each link holds the chain before it, so the first operand is the deepest node.
    let cases: [(&str, Shape, usize); 7] = [
        ("`+`", |n| format!("1{}", " + 1".repeat(n)), 19),
        ("`and`", |n| format!("true{}", " and true".repeat(n)), 19),
        ("`.`", |n| format!("m{}", ".a".repeat(n)), 19),
        ("index", |n| format!("m{}", "[0]".repeat(n)), 19),
        ("call", |n| format!("f{}", "()".repeat(n)), 19),
        ("`@`", |n| format!("1{}", " @ f()".repeat(n)), 19),
        // An `@` callee's path is a chain too, within the call it makes.
        (
            "`@` callee path",
            |n| format!("1 @ m{}()", ".a".repeat(n)),
            18,
        ),
    ];
    for (name, shape, deepest) in cases {
        assert_boundary(
            name,
            shape,
            deepest,
            ast_depth_limit(MAX),
            AST_DEPTH_MESSAGE,
        );
    }
}

#[test]
fn each_level_of_nesting_counts_against_the_ast_depth_limit() {
    const MAX: u64 = 20;
    let cases: [(&str, Shape, usize); 6] = [
        ("Array literal", |n| nest("[", "1", "]", n), 19),
        ("prefix `-`", |n| format!("{}1", "-".repeat(n)), 19),
        (
            "`elif`",
            |n| format!("if c: 0 {}else: 0", "elif c: 0 ".repeat(n)),
            18,
        ),
        (
            "Array pattern in a `def`",
            |n| format!("def {} = c", nest("[", "a", "]", n)),
            19,
        ),
        // Each level is an Array pattern holding an alternative, which holds the next:
        // two levels each.
        (
            "alternatives",
            |n| format!("def {} = c", nest("[a | ", "a", "]", n)),
            9,
        ),
        (
            "format String interpolation",
            |n| nest("$'${", "1", "}'", n),
            19,
        ),
    ];
    for (name, shape, deepest) in cases {
        assert_boundary(
            name,
            shape,
            deepest,
            ast_depth_limit(MAX),
            AST_DEPTH_MESSAGE,
        );
    }
}

#[test]
fn parentheses_add_no_ast_depth() {
    let src = nest("(", "1", ")", 100);
    assert_eq!(failure(&src, ast_depth_limit(1)), None);
    // A parenthesized operand of a chain is no deeper than a bare one.
    let src = format!("{}1", "(1) + ".repeat(5));
    assert_eq!(failure(&src, ast_depth_limit(6)), None);
}

#[test]
fn a_chain_on_a_deep_operand_counts_both() {
    // The Array literals reach depth 6 (5 Arrays and the 1), and each of 4 links
    // moves them one level deeper: 10.
    let operand_first = format!("{} + 1 + 1 + 1 + 1", nest("[", "1", "]", 5));
    let operand_last = format!("1 + 1 + 1 + 1 + {}", nest("[", "1", "]", 5));
    // A chain inside the Arrays: the chain's slot is depth 6, and its first operand 10.
    let chain_inside = nest("[", "1 + 1 + 1 + 1 + 1", "]", 5);
    for src in [operand_first, chain_inside] {
        assert_eq!(failure(&src, ast_depth_limit(10)), None, "{src}");
        assert!(failure(&src, ast_depth_limit(9)).is_some(), "{src}");
    }
    // A deep right operand is not pushed down by the links before it: the last link
    // holds it at depth 2, so the 1 in its innermost Array is at depth 7.
    assert_eq!(failure(&operand_last, ast_depth_limit(7)), None);
    assert!(failure(&operand_last, ast_depth_limit(6)).is_some());
}

#[test]
fn sibling_depths_do_not_add_up() {
    // The outer Array is depth 1. Its first element's innermost 1 is at depth 2 + 15,
    // and its second element, a chain of 15 links, puts its first operand at depth 2 + 15.
    let src = format!("[{}, 1{}]", nest("[", "1", "]", 15), " + 1".repeat(15));
    assert_eq!(failure(&src, ast_depth_limit(17)), None);
    assert!(failure(&src, ast_depth_limit(16)).is_some());
}

#[test]
fn an_interpolation_counts_from_where_its_format_string_sits() {
    // The outer Array is depth 1, the format String 2, the interpolated Array 3, and
    // its 1 depth 4. Recursion follows the same levels.
    let src = "[$'${[1]}']";
    assert_eq!(failure(src, recursion_limit(4)), None);
    assert!(failure(src, recursion_limit(3)).is_some());
    assert_eq!(failure(src, ast_depth_limit(4)), None);
    assert!(failure(src, ast_depth_limit(3)).is_some());

    // What an interpolation builds counts toward the chain holding its format String:
    // its innermost 1 is at depth 4, and the `+` moves it to 5.
    let src = "$'${[[1]]}' + 1";
    assert_eq!(failure(src, ast_depth_limit(5)), None);
    assert!(failure(src, ast_depth_limit(4)).is_some());
}

// -- Both --

#[test]
fn each_limit_applies_alone() {
    let deep_brackets = nest("[", "1", "]", 30);
    let long_chain = format!("1{}", " + 1".repeat(30));
    let limits = recursion_limit(10).with_max_ast_depth(None);
    assert!(failure(&deep_brackets, limits).is_some());
    assert_eq!(failure(&long_chain, limits), None);
    let limits = ast_depth_limit(10).with_max_recursion_depth(None);
    assert!(failure(&long_chain, limits).is_some());
    assert_eq!(failure(&nest("(", "1", ")", 30), limits), None);
}

#[test]
fn unlimited_parses_deep_source() {
    let cases = [
        nest("(", "1", ")", 100),
        nest("[", "1", "]", 100),
        format!("1{}", " + 1".repeat(1000)),
    ];
    for src in cases {
        assert!(
            parse_program_with_limits("test.frst", &src, ParseLimits::UNLIMITED).is_ok(),
            "{src}"
        );
    }
}

#[test]
fn a_lambda_body_read_both_ways_reports_the_limit() {
    // `{[` may open a Map or a block holding an Array, so the Map is tried first, and
    // a block if that fails. Past a limit, the error is the limit's, whichever it is.
    let block = format!("fn -> {{{}}}", nest("[", "1", "]", 20));
    let map = format!("fn -> {{[k]: {}}}", nest("[", "1", "]", 20));
    for src in [block, map] {
        for limits in [recursion_limit(10), ast_depth_limit(10)] {
            let failed = failure(&src, limits).expect("past the limit");
            assert!(
                failed.starts_with(RECURSION_MESSAGE) || failed.starts_with(AST_DEPTH_MESSAGE),
                "{src}: {failed}"
            );
        }
    }
}

// -- Errors --

#[test]
fn passing_the_recursion_limit_labels_where_it_is_passed() {
    let src = "[[[[1]]]]";
    let err = parse_program_with_limits("test.frst", src, recursion_limit(3)).expect_err(src);
    assert_eq!(err.message(), "nested too deeply: the limit is 3 levels");
    let [label] = err.labels() else {
        panic!("expected one label, got {:?}", err.labels());
    };
    // The fourth `[`.
    assert_eq!((label.span.start, label.span.end), (3, 4));
    assert_eq!(label.text, "past the limit here");
    assert_eq!(err.help(), Some("split it up with `def`"));
}

#[test]
fn passing_the_ast_depth_limit_labels_the_link_that_passes_it() {
    let src = "1 + 1 + 1 + 1";
    let err = parse_program_with_limits("test.frst", src, ast_depth_limit(3)).expect_err(src);
    assert_eq!(err.message(), "expression too deep: the limit is 3 levels");
    let [label] = err.labels() else {
        panic!("expected one label, got {:?}", err.labels());
    };
    // The third `+`.
    assert_eq!((label.span.start, label.span.end), (10, 11));
    assert_eq!(label.text, "past the limit here");
    assert_eq!(
        err.help(),
        Some(
            "split it up with `def`; each link of a chain such as `a + b + c` or `a.b.c` is one level deeper"
        )
    );
}

#[test]
fn passing_a_limit_at_the_end_of_input_labels_the_end() {
    // The recursion limit is passed by the expression `[` holds, which never comes.
    let src = "[";
    let err = parse_program_with_limits("test.frst", src, recursion_limit(1)).expect_err(src);
    assert_eq!(err.message(), "nested too deeply: the limit is 1 level");
    let [label] = err.labels() else {
        panic!("expected one label, got {:?}", err.labels());
    };
    assert_eq!((label.span.start, label.span.end), (1, 1));
}

// -- Deeply nested format Strings --
// The lexer reads a whole format String, nested ones included, as one token before the
// parser counts any level, so how deeply they nest must not matter until it does.

const FORMAT_DEPTH: usize = 20_000;

#[test]
fn deeply_nested_format_strings_are_rejected_at_the_limit() {
    let src = nest("$'${", "1", "}'", FORMAT_DEPTH);
    let failed = failure(&src, ParseLimits::new()).expect("past the default limit");
    assert!(failed.starts_with(RECURSION_MESSAGE), "{failed}");
}

#[test]
fn nested_format_strings_that_never_close_are_unclosed_within_the_limit() {
    // The error is the innermost format String's, which is what does not close.
    let src = format!("{}1", "$'${".repeat(10));
    assert_eq!(
        failure(&src, recursion_limit(20)).as_deref(),
        Some("unclosed format String")
    );
}

#[test]
fn nested_format_strings_that_never_close_are_too_deep_past_the_limit() {
    // Past the limit, the String would be too deep even if it closed, and its depth
    // comes first in the source.
    let src = format!("{}1", "$'${".repeat(FORMAT_DEPTH));
    let err = parse_program_with_limits("test.frst", &src, recursion_limit(20))
        .expect_err("past the limit");
    assert_eq!(err.message(), "nested too deeply: the limit is 20 levels");
    let [label] = err.labels() else {
        panic!("expected one label, got {:?}", err.labels());
    };
    // The 21st format String's `$'`.
    let opener = 20 * "$'${".len();
    assert_eq!((label.span.start, label.span.end), (opener, opener + 2));
}

// -- Defaults --

#[test]
fn parse_program_applies_the_default_limits() {
    let defaults = ParseLimits::new();
    assert_eq!(ParseLimits::default(), defaults);
    let max = defaults
        .max_recursion_depth
        .expect("the default limits recursion")
        .get() as usize;
    // On a test thread, whose stack is Rust's default size, which the defaults suit.
    let within = nest("[", "1", "]", max - 1);
    assert!(parse_program("test.frst", &within).is_ok());
    let past = nest("[", "1", "]", max);
    let err = parse_program("test.frst", &past).expect_err("past the default limit");
    assert!(
        err.message().starts_with(RECURSION_MESSAGE),
        "{}",
        err.message()
    );

    let max = defaults
        .max_ast_depth
        .expect("the default limits AST depth")
        .get() as usize;
    let within = format!("1{}", " + 1".repeat(max - 1));
    assert!(parse_program("test.frst", &within).is_ok());
    let past = format!("1{}", " + 1".repeat(max));
    let err = parse_program("test.frst", &past).expect_err("past the default limit");
    assert!(
        err.message().starts_with(AST_DEPTH_MESSAGE),
        "{}",
        err.message()
    );
}
