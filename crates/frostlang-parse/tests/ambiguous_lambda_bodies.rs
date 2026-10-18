//! Lambda bodies opening `{[`, which may be a Map with a computed key or a block whose
//! first statement is an Array: the parser tries the Map, and the block if that fails.
//! Nested, each level is reached under both readings of the level around it, so trying
//! each level's Map reading anew every time would take time exponential in the nesting.
//!
//! Each shape here is nested to the default limits, and must parse, or fail, within a
//! deadline far beyond what it needs.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::helpers::{RECURSION_MESSAGE, is_int, nest};

use frostlang_parse::ast::{Expr, FormatSegment, Spanned, Statement};
use frostlang_parse::{ParseError, parse_program};

/// How long a parse may take before it is taken to be stuck.
const DEADLINE: Duration = Duration::from_secs(30);

/// How many levels the default limits must allow. A parse doubling its work at each
/// level would take far longer than [`DEADLINE`] to reach this many.
const AT_LEAST: usize = 30;

/// How many levels [`up_to_the_limit`] tries before giving up on reaching the limit.
const AT_MOST: usize = 1000;

/// Parses `src` under the default limits on its own thread, failing the test if the
/// parse does not end within [`DEADLINE`].
fn parse_within_deadline(src: String) -> Result<Spanned<Expr>, ParseError> {
    let (sender, receiver) = mpsc::channel();
    let bytes = src.len();
    thread::spawn(move || {
        let parsed = parse_program("test.frst", &src).map(|program| {
            let [statement] = <[_; 1]>::try_from(program.statements).expect("one statement");
            match statement.node {
                Statement::Expr(expr) => expr,
                other => panic!("expected an expression, got {other:?}"),
            }
        });
        // Past the deadline, no one is left to receive it.
        let _ = sender.send(parsed);
    });
    match receiver.recv_timeout(DEADLINE) {
        Ok(parsed) => parsed,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("a parse of {bytes} bytes did not end within {DEADLINE:?}")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the parse panicked"),
    }
}

/// `levels` lambdas, each returning an Array holding the next, and the innermost `[1]`.
fn arrays(levels: usize) -> String {
    nest("fn -> {[", "1", "]}", levels)
}

/// [`arrays`], with each lambda in a format String's interpolation.
fn interpolated_arrays(levels: usize) -> String {
    nest("$'${fn -> {[", "1", "]}}'", levels)
}

/// [`arrays`], but with the innermost Array's element left unfinished.
fn unfinished(levels: usize) -> String {
    nest("fn -> {[", "1 +", "]}", levels)
}

/// [`interpolated_arrays`], but with the innermost Array's element left unfinished.
fn interpolated_unfinished(levels: usize) -> String {
    nest("$'${fn -> {[", "1 +", "]}}'", levels)
}

/// The one element of the Array that is the block body of the lambda `expr`.
#[track_caller]
fn sole_element(expr: &Spanned<Expr>) -> &Spanned<Expr> {
    let Expr::Lambda {
        body, return_expr, ..
    } = &expr.node
    else {
        panic!("expected a lambda, got {expr:?}");
    };
    assert!(body.is_empty(), "expected a lone Array, got {body:?}");
    match &return_expr.node {
        Expr::Array(elements) if elements.len() == 1 => &elements[0],
        other => panic!("expected an Array of one element, got {other:?}"),
    }
}

/// What the format String `expr` is: its only part, an interpolation.
#[track_caller]
fn sole_interpolation(expr: &Spanned<Expr>) -> &Spanned<Expr> {
    match &expr.node {
        Expr::FormatString(segments) => match segments.as_slice() {
            [FormatSegment::Interpolation(inner)] => inner,
            other => panic!("expected one interpolation, got {other:?}"),
        },
        other => panic!("expected a format String, got {other:?}"),
    }
}

/// Parses `shape` nested ever deeper until the default recursion limit stops it, and
/// returns the parse of every level within it, the shallowest first.
fn up_to_the_limit(shape: fn(usize) -> String) -> Vec<Result<Spanned<Expr>, ParseError>> {
    let mut parses = Vec::new();
    for levels in 1..=AT_MOST {
        let parsed = parse_within_deadline(shape(levels));
        if let Err(error) = &parsed
            && error.message().starts_with(RECURSION_MESSAGE)
        {
            assert!(
                parses.len() >= AT_LEAST,
                "the limit allows only {} levels",
                parses.len()
            );
            return parses;
        }
        parses.push(parsed);
    }
    panic!("{AT_MOST} levels never failed with {RECURSION_MESSAGE:?}")
}

/// Asserts that `shape` fails at every level within the limit, at the innermost `]`,
/// where the unfinished `+` expects an operand.
#[track_caller]
fn assert_fails_where_unfinished(shape: fn(usize) -> String) {
    for (levels, parsed) in (1..).zip(up_to_the_limit(shape)) {
        let error = parsed.expect_err("an unfinished `+` fails");
        assert_eq!(
            error.message(),
            "expected an expression, but found `]`",
            "{levels} levels"
        );
        let innermost_close = shape(levels).find(']').expect("a `]`");
        assert!(
            error
                .labels()
                .iter()
                .any(|label| label.span.start == innermost_close),
            "{levels} levels: expected a label on the innermost `]`, got {:?}",
            error.labels()
        );
    }
}

#[test]
fn nested_arrays_parse_as_blocks_to_the_limit() {
    for (levels, parsed) in (1..).zip(up_to_the_limit(arrays)) {
        let parsed = parsed.unwrap_or_else(|error| panic!("{levels} levels: {error}"));
        let mut expr = &parsed;
        for _ in 0..levels {
            expr = sole_element(expr);
        }
        assert!(is_int(expr, 1), "{levels} levels: innermost {expr:?}");
    }
}

#[test]
fn nested_arrays_in_interpolations_parse_as_blocks_to_the_limit() {
    for (levels, parsed) in (1..).zip(up_to_the_limit(interpolated_arrays)) {
        let parsed = parsed.unwrap_or_else(|error| panic!("{levels} levels: {error}"));
        let mut expr = &parsed;
        for _ in 0..levels {
            expr = sole_element(sole_interpolation(expr));
        }
        assert!(is_int(expr, 1), "{levels} levels: innermost {expr:?}");
    }
}

#[test]
fn nested_invalid_bodies_fail_where_they_are_unfinished_to_the_limit() {
    assert_fails_where_unfinished(unfinished);
}

#[test]
fn nested_invalid_bodies_in_interpolations_fail_where_they_are_unfinished_to_the_limit() {
    assert_fails_where_unfinished(interpolated_unfinished);
}
