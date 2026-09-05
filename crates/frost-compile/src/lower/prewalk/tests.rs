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

use super::free_names;
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

/// Free names of `source`'s lambda, in discovery (evaluation) order.
fn ordered(source: &str) -> Vec<String> {
    free_names(&lambda_of(source))
}

/// Free names of `source`'s lambda, as a sorted set.
fn free(source: &str) -> Vec<String> {
    let mut names = ordered(source);
    names.sort();
    names
}

fn assert_free(source: &str, expected: &[&str]) {
    assert_eq!(free(source), expected, "free names of `{source}`");
}

// -- Parameters, globals, basics --

#[test]
fn parameters_are_not_captured() {
    assert_free("fn a -> a + b", &["b"]);
    assert_free("fn a, b -> a + b + c + d", &["c", "d"]);
    assert_free("fn x -> x + x", &[]);
}

#[test]
fn a_global_name_used_in_a_lambda_is_a_free_name() {
    // The pre-walk does not filter globals; codegen resolves a global-named free
    // name to LoadGlobal (unless an enclosing scope shadows it).
    assert_free("fn s -> to_upper(s)", &["to_upper"]);
    assert_free("fn -> to_upper(x)", &["to_upper", "x"]);
    // A parameter of the same name shadows the global: not free.
    assert_free("fn to_upper -> to_upper", &[]);
}

#[test]
fn a_nested_lambda_over_a_shadowed_global_does_not_escape() {
    // In isolation `to_upper` is free (codegen would make it the builtin)...
    assert_free("fn -> to_upper", &["to_upper"]);
    // ...but under an enclosing binding of that name, the enclosing lambda
    // supplies it, so nothing is free at the outer level. This is the case that
    // a global filter inside the walk would get wrong.
    assert_free("fn to_upper -> fn -> to_upper", &[]);
    assert_free("fn -> { def to_upper = 7; fn -> to_upper }", &[]);
}

#[test]
fn a_global_is_free_again_once_a_shadow_leaves_scope() {
    // The `do` shadows `to_upper`, then exits; the later use is free again
    // (codegen resolves it to the builtin).
    assert_free("fn s -> { do { def to_upper = 1; 0 }; to_upper(s) }", &["to_upper"]);
}

#[test]
fn a_literal_only_body_captures_nothing() {
    assert_free("fn -> 42", &[]);
    assert_free("fn -> \"text\"", &[]);
}

// -- Operators and simple expression forms --

#[test]
fn unary_and_binary_operators() {
    assert_free("fn -> -x", &["x"]);
    assert_free("fn -> not x", &["x"]);
    assert_free("fn -> a + b * c", &["a", "b", "c"]);
    assert_free("fn -> p and q or r", &["p", "q", "r"]);
}

#[test]
fn if_walks_both_branches_and_the_condition() {
    assert_free("fn -> if c: t else: e", &["c", "e", "t"]);
    assert_free("fn -> if c: t", &["c", "t"]);
}

#[test]
fn calls_capture_callee_and_arguments() {
    assert_free("fn -> f(x, y)", &["f", "x", "y"]);
    assert_free("fn a -> f(a, g(a))", &["f", "g"]);
}

#[test]
fn indexing() {
    assert_free("fn -> a[b]", &["a", "b"]);
    // A hard-index key is a literal field name, not a variable.
    assert_free("fn -> x.field", &["x"]);
    assert_free("fn -> x.a.b.c", &["x"]);
    assert_free("fn m -> m.k[i]", &["i"]);
}

#[test]
fn collection_literals() {
    assert_free("fn -> [a, b, c]", &["a", "b", "c"]);
    // Computed key is a usage; an identifier key is a literal.
    assert_free("fn -> {[k]: v}", &["k", "v"]);
    assert_free("fn v -> {foo: v}", &[]);
    assert_free("fn -> {foo: v}", &["v"]);
}

#[test]
fn format_strings() {
    assert_free("fn -> $'${a} and ${b}'", &["a", "b"]);
    assert_free("fn -> $'no interpolation'", &[]);
    assert_free("fn a -> $'${a}${z}'", &["z"]);
}

#[test]
fn pipeline_expressions() {
    assert_free("fn -> filter xs with pred", &["pred", "xs"]);
    assert_free("fn -> map xs with f", &["f", "xs"]);
    assert_free("fn -> reduce xs with op", &["op", "xs"]);
    assert_free("fn -> reduce xs init: seed with op", &["op", "seed", "xs"]);
    assert_free("fn -> foreach xs with f", &["f", "xs"]);
}

// -- do blocks --

#[test]
fn do_block_absorbs_its_own_definitions() {
    assert_free("fn -> do { def x = 1; x + y }", &["y"]);
}

#[test]
fn a_name_out_of_scope_after_a_do_is_free() {
    assert_free("fn -> { do { def x = 1; x }; x }", &["x"]);
}

#[test]
fn nested_do_blocks_stack_and_unwind() {
    assert_free("fn -> do { def x = 1; do { def y = 2; x + y + z } }", &["z"]);
    // Each do's locals leave scope at its own `}`.
    assert_free("fn -> { do { def x = 1; x }; do { def y = 2; y } }", &[]);
}

#[test]
fn a_do_local_shadows_an_outer_local_then_restores() {
    assert_free("fn -> { def x = 1; do { def x = 2; x }; x }", &[]);
}

// -- Statement ordering and shadowing --

#[test]
fn use_before_definition_is_captured() {
    assert_free("fn -> { y; def y = 1; y }", &["y"]);
    assert_free("fn -> { a; def a = a; a }", &["a"]);
}

#[test]
fn later_statements_see_earlier_definitions() {
    assert_free("fn -> { def a = x; def b = a; a + b }", &["x"]);
}

// -- Destructuring in def --

#[test]
fn array_destructuring() {
    assert_free("fn -> { def [a, b] = pair; a + b + c }", &["c", "pair"]);
    assert_free("fn -> { def [a, [b, c]] = x; a + b + c }", &["x"]);
    assert_free("fn -> { def [a, ...rest] = x; a + rest }", &["x"]);
}

#[test]
fn map_destructuring() {
    assert_free("fn -> { def {foo: a} = m; a }", &["m"]);
    assert_free("fn -> { def {[k]: a} = m; a }", &["k", "m"]);
    assert_free("fn -> { def {foo: a} as whole = m; a + whole }", &["m"]);
}

#[test]
fn a_discard_binds_nothing_but_evaluates_the_rhs() {
    assert_free("fn -> { def _ = x; y }", &["x", "y"]);
}

// -- match --

#[test]
fn match_target_and_arms() {
    assert_free("fn -> match t { n is Int => n + x, _ => y }", &["t", "x", "y"]);
    assert_free("fn -> match t { n is String => n, _ => z }", &["t", "z"]);
}

#[test]
fn match_bindings_scope_to_their_arm() {
    // `n` is bound in the first arm only; it is not in scope for the second.
    assert_free("fn -> match t { n => n, _ => n }", &["n", "t"]);
}

#[test]
fn match_guard_sees_pattern_bindings() {
    assert_free("fn -> match t { n if: n > lim => n, _ => d }", &["d", "lim", "t"]);
}

#[test]
fn match_value_pattern_uses_a_name() {
    assert_free("fn -> match t { (v) => 1, _ => 2 }", &["t", "v"]);
    // A literal value pattern references nothing.
    assert_free("fn -> match t { 42 => x, _ => y }", &["t", "x", "y"]);
}

#[test]
fn match_array_pattern() {
    assert_free("fn -> match t { [a, b] => a + b, _ => z }", &["t", "z"]);
    assert_free("fn -> match t { [a, ...rest] => a + rest, _ => z }", &["t", "z"]);
}

#[test]
fn match_map_pattern() {
    assert_free("fn -> match t { {foo: a} => a, _ => z }", &["t", "z"]);
    assert_free("fn -> match t { {[k]: a} => a, _ => z }", &["k", "t", "z"]);
    assert_free("fn -> match t { {foo: a} as w => a + w, _ => z }", &["t", "z"]);
}

#[test]
fn match_alternatives_bind_the_same_names() {
    assert_free("fn -> match t { [a] | {foo: a} => a, _ => z }", &["t", "z"]);
}

#[test]
fn a_pattern_binding_is_visible_to_later_pattern_elements() {
    // `(a)` compares against the `a` bound by the first element: no capture.
    assert_free("fn a -> match a { [a, (a)] => true, _ => false }", &[]);
    // A computed key sees a binding from an earlier entry of the same pattern.
    assert_free("fn m -> match m { {kind, [kind]: node} => node, _ => 0 }", &[]);
}

#[test]
fn a_destructure_binding_is_visible_to_later_computed_keys() {
    // `[kind]` resolves the `kind` bound by the earlier entry: no capture.
    assert_free(
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
    assert_free("fn -> { def {[a]: a} = m; a }", &["a", "m"]);
}

#[test]
fn a_pattern_element_does_not_see_a_later_binding() {
    // Reverse of the forward case: the key `[kind]` precedes the `kind`
    // binding, so it captures.
    assert_free("fn -> match m { {[kind]: node, kind} => node, _ => 0 }", &["kind", "m"]);
}

#[test]
fn a_lambda_in_a_match_arm_captures_the_arm_binding() {
    // The inner lambda captures `xs` (the arm binding); only `t` reaches the
    // outer lambda.
    assert_free("fn -> match t { xs => map xs with fn y -> y + xs, _ => 0 }", &["t"]);
}

#[test]
fn use_before_definition_inside_a_do_escapes() {
    // A `do` is its own scoping node; the leading `x` precedes its def.
    assert_free("fn -> do { x; def x = 5; x }", &["x"]);
    assert_free("fn -> do { def x = xo; x }", &["xo"]);
}

#[test]
fn a_guard_sees_an_alternative_binding() {
    // `a` in the guard is bound by whichever alternative matched.
    assert_free("fn -> match t { [a] | {foo: a} if: a > lim => a, _ => 0 }", &["lim", "t"]);
}

#[test]
fn a_pattern_element_binding_is_visible_without_a_shadowing_param() {
    // Isolates the rule: `a` is not a parameter here, so `(a)` can only be
    // seeing the first element's binding.
    assert_free("fn -> match t { [a, (a)] => true, _ => false }", &["t"]);
}

#[test]
fn a_whole_map_binding_is_introduced_after_the_subpatterns() {
    // `[whole]` precedes the `as whole` binding, so it captures.
    assert_free("fn -> match m { {[whole]: node} as whole => node, _ => 0 }", &["m", "whole"]);
}

#[test]
fn a_named_but_unused_local_does_not_capture_though_its_rhs_does() {
    assert_free("fn -> { def x = free; 5 }", &["free"]);
}

#[test]
fn abbreviated_lambda_arity_is_the_highest_placeholder() {
    // `$3` alone makes a three-parameter lambda; `$1` and `$2` are also params.
    assert_free("$($3 + q)", &["q"]);
}

// -- Nested lambdas and capture propagation --

#[test]
fn inner_lambda_forces_outer_to_capture() {
    assert_free("fn a -> fn b -> x + a + b", &["x"]);
}

#[test]
fn inner_captures_outer_param_without_outer_capturing() {
    assert_free("fn a -> fn b -> a + b", &[]);
}

#[test]
fn capture_propagates_through_three_levels() {
    // c, b, a are each a lambda's own param; only w reaches the outermost.
    assert_free("fn a -> fn b -> fn c -> a + b + c + w", &["w"]);
}

#[test]
fn a_lambda_inside_a_do_captures_free_names_only() {
    // x is a function local (in scope for the inner lambda); w is free.
    assert_free("fn -> do { def x = 1; fn a -> x + a + w }", &["w"]);
}

// -- Self-name (named lambdas) --

#[test]
fn self_name_is_an_internal_definition() {
    assert_free("fn foo(a) -> foo(a)", &[]);
    assert_free("fn foo(a) -> foo(a) + b", &["b"]);
    assert_free(
        "fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)",
        &[],
    );
}

// -- Variadic parameter --

#[test]
fn variadic_parameter_is_not_captured() {
    assert_free("fn a, ...rest -> a + rest + b", &["b"]);
}

// -- Abbreviated lambdas --

#[test]
fn abbreviated_lambda_parameters() {
    assert_free("$($1 + $2)", &[]);
    assert_free("$($1 + y)", &["y"]);
    // `$` aliases `$1`.
    assert_free("$($ + z)", &["z"]);
    // A gap: `$2` used without `$1` still makes both positional params.
    assert_free("$($2 + q)", &["q"]);
    // The rest parameter `$$`.
    assert_free("$(g($$))", &["g"]);
}

#[test]
fn abbreviated_lambda_nested_in_a_lambda() {
    // The inner `$($1 + a + w)` has params `$1`; `a` is the outer param, `w` free.
    assert_free("fn a -> map xs with $($1 + a + w)", &["w", "xs"]);
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

#[test]
fn evaluation_order_of_compound_forms() {
    assert_eq!(ordered("fn -> if c: t else: e"), ["c", "t", "e"]);
    assert_eq!(ordered("fn -> [a, b, c]"), ["a", "b", "c"]);
    assert_eq!(ordered("fn -> {[k]: v}"), ["k", "v"]);
    // Reduce: structure, operation, then the optional init seed.
    assert_eq!(ordered("fn -> reduce xs init: seed with op"), ["xs", "op", "seed"]);
    // Match: discriminant, then each arm's pattern, guard, result.
    assert_eq!(
        ordered("fn -> match t { (p) if: gd => r, _ => e }"),
        ["t", "p", "gd", "r", "e"]
    );
}

// -- More corner cases --

#[test]
fn zero_arg_abbreviated_thunk() {
    assert_free("$(w)", &["w"]);
    assert_free("$(42)", &[]);
}

#[test]
fn lambda_in_a_match_guard_captures_the_arm_binding() {
    assert_free("fn -> match t { n if: (fn -> n > lim)() => n, _ => 0 }", &["lim", "t"]);
}

#[test]
fn do_block_inside_a_pattern_computed_key() {
    assert_free("fn a -> match m { {[do { def k = a; k }]: v} => v, _ => 0 }", &["m"]);
}

#[test]
fn closure_created_before_the_def_it_needs() {
    // `inner` closes over `x` before `x` is defined: `x` is a capture.
    assert_free("fn -> { def inner = fn -> x; def x = 1; inner }", &["x"]);
}

#[test]
fn self_name_used_from_a_nested_lambda() {
    assert_free("fn foo() -> fn -> foo", &[]);
}

#[test]
fn abbreviated_placeholder_through_a_nested_regular_lambda() {
    // The inner `fn -> $1` captures `$1` from the enclosing abbreviated lambda.
    assert_free("$( (fn -> $1)() )", &[]);
}

#[test]
fn nested_abbreviated_lambdas_keep_separate_placeholders() {
    // Each `$1` belongs to its own lambda; only `g` and `h` are free.
    assert_free("$( g($1, $(h($1))) )", &["g", "h"]);
}

#[test]
fn dedup_through_the_replay_path() {
    assert_eq!(ordered("fn -> [fn -> w, fn -> w, w]"), ["w"]);
}

#[test]
fn lambda_inside_a_format_interpolation() {
    assert_free("fn -> $'${(fn -> w)()}'", &["w"]);
}

#[test]
fn match_in_a_def_rhs() {
    // The arm scope must unwind before `x` is defined.
    assert_free("fn -> { def x = match t { n => n, _ => 0 }; x }", &["t"]);
}

#[test]
fn guard_only_usage_on_a_discard_arm() {
    assert_free("fn -> match t { _ if: g => 1, _ => 2 }", &["g", "t"]);
}

#[test]
fn threading_operator_discovers_callee_first() {
    // `a @ f(b)` desugars to `f(a, b)`; discovery is callee-first, so the order
    // diverges from source order.
    assert_free("fn -> a @ f(b)", &["a", "b", "f"]);
    assert_eq!(ordered("fn -> a @ f(b)"), ["f", "a", "b"]);
}
