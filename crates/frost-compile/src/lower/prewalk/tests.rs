//! Tests for capture discovery.
//!
//! Capture analysis underpins the correctness of every closure the compiler
//! emits, so this is exhaustive: every expression form that has sub-expressions,
//! every destructure and match-pattern shape, nested and shadowing scopes, and
//! the exact evaluation order that fixes capture slots.
//!
//! Each case parses a lambda expression and asserts its free names. Most assert
//! the sorted set (what matters is *which* names are free); a few assert the
//! unsorted order, to pin the evaluation-order walk.

use super::find_captures;
use frost_parse::{ast::Statement, parse_program};

/// The lambda expression that is `source`'s single statement.
fn lambda_of(source: &str) -> frost_parse::ast::Expr {
    let program = parse_program("test.frst", source).expect("source should parse");
    let statement = program
        .statements
        .into_iter()
        .next()
        .expect("one statement");
    let Statement::Expr(expr) = statement.node else {
        panic!("expected an expression statement");
    };
    expr.node
}

/// Captures of `source`'s lambda, in discovery (evaluation) order.
fn ordered(source: &str) -> Vec<String> {
    find_captures(&lambda_of(source))
}

/// Captures of `source`'s lambda, as a sorted set.
fn captures(source: &str) -> Vec<String> {
    let mut names = ordered(source);
    names.sort();
    names
}

fn assert_captures(source: &str, expected: &[&str]) {
    assert_eq!(captures(source), expected, "captures of `{source}`");
}

// -- Parameters, globals, basics --

#[test]
fn parameters_are_not_captured() {
    assert_captures("fn a -> a + b", &["b"]);
    assert_captures("fn a, b -> a + b + c + d", &["c", "d"]);
    assert_captures("fn x -> x + x", &[]);
}

#[test]
fn globals_are_not_captured() {
    assert_captures("fn s -> to_upper(s)", &[]);
    assert_captures("fn -> to_upper(x)", &["x"]);
    // A local of the same name shadows the global (poor style, still legal).
    assert_captures("fn to_upper -> to_upper", &[]);
}

#[test]
fn a_literal_only_body_captures_nothing() {
    assert_captures("fn -> 42", &[]);
    assert_captures("fn -> \"text\"", &[]);
}

// -- Operators and simple expression forms --

#[test]
fn unary_and_binary_operators() {
    assert_captures("fn -> -x", &["x"]);
    assert_captures("fn -> not x", &["x"]);
    assert_captures("fn -> a + b * c", &["a", "b", "c"]);
    assert_captures("fn -> p and q or r", &["p", "q", "r"]);
}

#[test]
fn if_walks_both_branches_and_the_condition() {
    assert_captures("fn -> if c: t else: e", &["c", "e", "t"]);
    assert_captures("fn -> if c: t", &["c", "t"]);
}

#[test]
fn calls_capture_callee_and_arguments() {
    assert_captures("fn -> f(x, y)", &["f", "x", "y"]);
    assert_captures("fn a -> f(a, g(a))", &["f", "g"]);
}

#[test]
fn indexing() {
    assert_captures("fn -> a[b]", &["a", "b"]);
    // A hard-index key is a literal field name, not a variable.
    assert_captures("fn -> x.field", &["x"]);
    assert_captures("fn -> x.a.b.c", &["x"]);
    assert_captures("fn m -> m.k[i]", &["i"]);
}

#[test]
fn collection_literals() {
    assert_captures("fn -> [a, b, c]", &["a", "b", "c"]);
    // Computed key is a usage; an identifier key is a literal.
    assert_captures("fn -> {[k]: v}", &["k", "v"]);
    assert_captures("fn v -> {foo: v}", &[]);
    assert_captures("fn -> {foo: v}", &["v"]);
}

#[test]
fn format_strings() {
    assert_captures("fn -> $'${a} and ${b}'", &["a", "b"]);
    assert_captures("fn -> $'no interpolation'", &[]);
    assert_captures("fn a -> $'${a}${z}'", &["z"]);
}

#[test]
fn pipeline_expressions() {
    assert_captures("fn -> filter xs with pred", &["pred", "xs"]);
    assert_captures("fn -> map xs with f", &["f", "xs"]);
    assert_captures("fn -> reduce xs with op", &["op", "xs"]);
    assert_captures("fn -> reduce xs init: seed with op", &["op", "seed", "xs"]);
    assert_captures("fn -> foreach xs with f", &["f", "xs"]);
}

// -- do blocks --

#[test]
fn do_block_absorbs_its_own_definitions() {
    assert_captures("fn -> do { def x = 1; x + y }", &["y"]);
}

#[test]
fn a_name_out_of_scope_after_a_do_is_free() {
    assert_captures("fn -> { do { def x = 1; x }; x }", &["x"]);
}

#[test]
fn nested_do_blocks_stack_and_unwind() {
    assert_captures("fn -> do { def x = 1; do { def y = 2; x + y + z } }", &["z"]);
    // Each do's locals leave scope at its own `}`.
    assert_captures("fn -> { do { def x = 1; x }; do { def y = 2; y } }", &[]);
}

#[test]
fn a_do_local_shadows_an_outer_local_then_restores() {
    assert_captures("fn -> { def x = 1; do { def x = 2; x }; x }", &[]);
}

// -- Statement ordering and shadowing --

#[test]
fn use_before_definition_is_captured() {
    assert_captures("fn -> { y; def y = 1; y }", &["y"]);
    assert_captures("fn -> { a; def a = a; a }", &["a"]);
}

#[test]
fn later_statements_see_earlier_definitions() {
    assert_captures("fn -> { def a = x; def b = a; a + b }", &["x"]);
}

// -- Destructuring in def --

#[test]
fn array_destructuring() {
    assert_captures("fn -> { def [a, b] = pair; a + b + c }", &["c", "pair"]);
    assert_captures("fn -> { def [a, [b, c]] = x; a + b + c }", &["x"]);
    assert_captures("fn -> { def [a, ...rest] = x; a + rest }", &["x"]);
}

#[test]
fn map_destructuring() {
    assert_captures("fn -> { def {foo: a} = m; a }", &["m"]);
    assert_captures("fn -> { def {[k]: a} = m; a }", &["k", "m"]);
    assert_captures("fn -> { def {foo: a} as whole = m; a + whole }", &["m"]);
}

#[test]
fn a_discard_binds_nothing_but_evaluates_the_rhs() {
    assert_captures("fn -> { def _ = x; y }", &["x", "y"]);
}

// -- match --

#[test]
fn match_target_and_arms() {
    assert_captures("fn -> match t { n is Int => n + x, _ => y }", &["t", "x", "y"]);
    assert_captures("fn -> match t { n is String => n, _ => z }", &["t", "z"]);
}

#[test]
fn match_bindings_scope_to_their_arm() {
    // `n` is bound in the first arm only; it is not in scope for the second.
    assert_captures("fn -> match t { n => n, _ => n }", &["n", "t"]);
}

#[test]
fn match_guard_sees_pattern_bindings() {
    assert_captures("fn -> match t { n if: n > lim => n, _ => d }", &["d", "lim", "t"]);
}

#[test]
fn match_value_pattern_uses_a_name() {
    assert_captures("fn -> match t { (v) => 1, _ => 2 }", &["t", "v"]);
    // A literal value pattern references nothing.
    assert_captures("fn -> match t { 42 => x, _ => y }", &["t", "x", "y"]);
}

#[test]
fn match_array_pattern() {
    assert_captures("fn -> match t { [a, b] => a + b, _ => z }", &["t", "z"]);
    assert_captures("fn -> match t { [a, ...rest] => a + rest, _ => z }", &["t", "z"]);
}

#[test]
fn match_map_pattern() {
    assert_captures("fn -> match t { {foo: a} => a, _ => z }", &["t", "z"]);
    assert_captures("fn -> match t { {[k]: a} => a, _ => z }", &["k", "t", "z"]);
    assert_captures("fn -> match t { {foo: a} as w => a + w, _ => z }", &["t", "z"]);
}

#[test]
fn match_alternatives_bind_the_same_names() {
    assert_captures("fn -> match t { [a] | {foo: a} => a, _ => z }", &["t", "z"]);
}

#[test]
fn a_pattern_binding_is_visible_to_later_pattern_elements() {
    // `(a)` compares against the `a` bound by the first element: no capture.
    assert_captures("fn a -> match a { [a, (a)] => true, _ => false }", &[]);
    // A computed key sees a binding from an earlier entry of the same pattern.
    assert_captures("fn m -> match m { {kind, [kind]: node} => node, _ => 0 }", &[]);
}

#[test]
fn a_destructure_binding_is_visible_to_later_computed_keys() {
    // `[kind]` resolves the `kind` bound by the earlier entry: no capture.
    assert_captures(
        "fn -> { def {kind, [kind]: node} = {kind: 'k', other: 42}; node }",
        &[],
    );
}

// -- Corner cases verified against the C++ oracle --
//
// Each pins a distinct scope/ordering rule. Visibility within a pattern or
// destructure is strictly *preceding*: an element sees earlier bindings but not
// its own or later ones.

#[test]
fn alternatives_contribute_usages_from_every_branch() {
    // Value patterns in each branch are usages; both escape. The discriminant
    // is evaluated before any arm, so `t` is discovered first: the pattern match
    // logically depends on the discriminant's value.
    assert_eq!(
        ordered("fn -> match t { (a) | (b) => 1, _ => 0 }"),
        ["t", "a", "b"]
    );
}

#[test]
fn a_computed_key_does_not_see_its_own_entry_binding() {
    // `[a]` is evaluated before this entry binds `a`, so `a` is captured.
    assert_captures("fn -> { def {[a]: a} = m; a }", &["a", "m"]);
}

#[test]
fn a_pattern_element_does_not_see_a_later_binding() {
    // Reverse of the forward case: the key `[kind]` precedes the `kind`
    // binding, so it captures.
    assert_captures("fn -> match m { {[kind]: node, kind} => node, _ => 0 }", &["kind", "m"]);
}

#[test]
fn a_lambda_in_a_match_arm_captures_the_arm_binding() {
    // The inner lambda captures `xs` (the arm binding); only `t` reaches the
    // outer lambda.
    assert_captures("fn -> match t { xs => map xs with fn y -> y + xs, _ => 0 }", &["t"]);
}

#[test]
fn use_before_definition_inside_a_do_escapes() {
    // A `do` is its own scoping node; the leading `x` precedes its def.
    assert_captures("fn -> do { x; def x = 5; x }", &["x"]);
    assert_captures("fn -> do { def x = xo; x }", &["xo"]);
}

#[test]
fn a_guard_sees_an_alternative_binding() {
    // `a` in the guard is bound by whichever alternative matched.
    assert_captures("fn -> match t { [a] | {foo: a} if: a > lim => a, _ => 0 }", &["lim", "t"]);
}

#[test]
fn a_pattern_element_binding_is_visible_without_a_shadowing_param() {
    // Isolates the rule: `a` is not a parameter here, so `(a)` can only be
    // seeing the first element's binding.
    assert_captures("fn -> match t { [a, (a)] => true, _ => false }", &["t"]);
}

#[test]
fn a_whole_map_binding_is_introduced_after_the_subpatterns() {
    // `[whole]` precedes the `as whole` binding, so it captures.
    assert_captures("fn -> match m { {[whole]: node} as whole => node, _ => 0 }", &["m", "whole"]);
}

#[test]
fn a_named_but_unused_local_does_not_capture_though_its_rhs_does() {
    assert_captures("fn -> { def x = free; 5 }", &["free"]);
}

#[test]
fn abbreviated_lambda_arity_is_the_highest_placeholder() {
    // `$3` alone makes a three-parameter lambda; `$1` and `$2` are also params.
    assert_captures("$($3 + q)", &["q"]);
}

// -- Nested lambdas and capture propagation --

#[test]
fn inner_lambda_forces_outer_to_capture() {
    assert_captures("fn a -> fn b -> x + a + b", &["x"]);
}

#[test]
fn inner_captures_outer_param_without_outer_capturing() {
    assert_captures("fn a -> fn b -> a + b", &[]);
}

#[test]
fn capture_propagates_through_three_levels() {
    // c, b, a are each a lambda's own param; only w reaches the outermost.
    assert_captures("fn a -> fn b -> fn c -> a + b + c + w", &["w"]);
}

#[test]
fn a_lambda_inside_a_do_captures_free_names_only() {
    // x is a function local (in scope for the inner lambda); w is free.
    assert_captures("fn -> do { def x = 1; fn a -> x + a + w }", &["w"]);
}

// -- Self-name (named lambdas) --

#[test]
fn self_name_is_an_internal_definition() {
    assert_captures("fn foo(a) -> foo(a)", &[]);
    assert_captures("fn foo(a) -> foo(a) + b", &["b"]);
    assert_captures(
        "fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)",
        &[],
    );
}

// -- Variadic parameter --

#[test]
fn variadic_parameter_is_not_captured() {
    assert_captures("fn a, ...rest -> a + rest + b", &["b"]);
}

// -- Abbreviated lambdas --

#[test]
fn abbreviated_lambda_parameters() {
    assert_captures("$($1 + $2)", &[]);
    assert_captures("$($1 + y)", &["y"]);
    // `$` aliases `$1`.
    assert_captures("$($ + z)", &["z"]);
    // A gap: `$2` used without `$1` still makes both positional params.
    assert_captures("$($2 + q)", &["q"]);
    // The rest parameter `$$`.
    assert_captures("$(g($$))", &["g"]);
}

#[test]
fn abbreviated_lambda_nested_in_a_lambda() {
    // The inner `$($1 + a + w)` has params `$1`; `a` is the outer param, `w` free.
    assert_captures("fn a -> map xs with $($1 + a + w)", &["w", "xs"]);
}

// -- Evaluation order (unsorted) --

#[test]
fn captures_are_discovered_in_evaluation_order() {
    assert_eq!(ordered("fn -> a + b"), ["a", "b"]);
    assert_eq!(ordered("fn -> b + a"), ["b", "a"]);
    // Callee before arguments.
    assert_eq!(ordered("fn -> f(x, y)"), ["f", "x", "y"]);
    // The rhs of a def is evaluated (and its frees discovered) before the body.
    assert_eq!(ordered("fn -> { def local = seed; local + after }"), ["seed", "after"]);
}

#[test]
fn a_repeated_free_name_is_captured_once_at_first_use() {
    assert_eq!(ordered("fn -> a + b + a"), ["a", "b"]);
    assert_eq!(ordered("fn -> g(a, a, b)"), ["g", "a", "b"]);
}
