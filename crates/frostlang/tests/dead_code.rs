//! Dead code: a store no later code reads is discarded instead
//! (`dead_store_eliminate`), and a value loaded only to be discarded is not
//! loaded (`discard_eliminate`).
//!
//! The harness runs each behavioral case under every optimization permutation,
//! so neither is checked ever to change what a program does. The code
//! assertions pin the options they are about.

use crate::script;

use frostlang::Value;
use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use script::{Script, UNOPTIMIZED, raises, run};

const DEAD_STORE: OptimizationOptions = UNOPTIMIZED.with(Optimization::DeadStoreEliminate, true);

const DISCARD: OptimizationOptions = UNOPTIMIZED.with(Optimization::DiscardEliminate, true);

const BOTH: OptimizationOptions = UNOPTIMIZED
    .with(Optimization::DeadStoreEliminate, true)
    .with(Optimization::DiscardEliminate, true);

/// Whether `code` stores to any local.
fn stores(code: &[Bytecode]) -> bool {
    code.iter().any(|op| matches!(op, Bytecode::DefLocal(_)))
}

/// How many `DefLocal`s `code` has.
fn store_count(code: &[Bytecode]) -> usize {
    code.iter()
        .filter(|op| matches!(op, Bytecode::DefLocal(_)))
        .count()
}

/// Whether `code` loads any local.
fn loads_a_local(code: &[Bytecode]) -> bool {
    code.iter().any(|op| matches!(op, Bytecode::LoadLocal(_)))
}

/// Whether `code` creates any closure.
fn creates_a_closure(code: &[Bytecode]) -> bool {
    code.iter()
        .any(|op| matches!(op, Bytecode::CreateClosure { .. }))
}

// Behavior, under every permutation.

#[test]
fn an_unread_binding_does_not_change_the_result() {
    let source = r#"
        def unused = "never read"
        def [a, b] = [1, 2]
        a
    "#;
    assert_eq!(run(source), Value::Int(1));
}

#[test]
fn an_unread_binding_whose_value_raises_still_raises() {
    let message = raises(
        r"
        def unused = 1 / 0
        2
    ",
    );
    assert!(message.contains("zero"), "{message}");
}

#[test]
fn an_unread_binding_whose_value_has_an_effect_still_has_it() {
    let script = Script::new(
        r"
        def unused = print(1)
        2
    ",
    );
    assert_eq!(script.printed(), vec!["1"]);
}

#[test]
fn an_unread_exported_binding_is_still_exported() {
    let finished = Script::new(
        r"
        export def shown = 1
        def hidden = 2
        3
    ",
    )
    .finish();
    assert_eq!(finished.tail, Value::Int(3));
    assert_eq!(
        finished.exports.into_iter().collect::<Vec<_>>(),
        vec![("shown".to_string(), Value::Int(1))]
    );
}

#[test]
fn an_unread_implicitly_exported_binding_is_still_exported() {
    let finished = Script::new(
        r"
        def shown = 1
        2
    ",
    )
    .implicit_export()
    .finish();
    assert_eq!(
        finished.exports.into_iter().collect::<Vec<_>>(),
        vec![("shown".to_string(), Value::Int(1))]
    );
}

#[test]
fn a_binding_read_on_only_one_branch_keeps_its_value() {
    let source = r"
        defn pick(flag) -> {
            def x = 10
            if flag: x else: 20
        }
        [pick(true), pick(false)]
    ";
    assert_eq!(
        run(source),
        Value::from(vec![Value::Int(10), Value::Int(20)])
    );
}

#[test]
fn a_shadowed_binding_is_independent_of_its_shadow() {
    let source = r"
        def x = 1
        do {
            def x = 2
            x
        }
    ";
    assert_eq!(run(source), Value::Int(2));
}

#[test]
fn unused_parameters_do_not_change_the_result() {
    let source = r"
        def first = fn x, y -> x
        def head = fn x, ...rest -> x
        def constant = fn x -> 7
        [first(1, 2), head(3, 4, 5), constant(6)]
    ";
    assert_eq!(
        run(source),
        Value::from(vec![Value::Int(1), Value::Int(3), Value::Int(7)])
    );
}

#[test]
fn unused_match_bindings_do_not_change_the_result() {
    let source = r"
        def classify = fn value -> match value {
            [a, b] | [b, a, _] => 'pair or triple',
            {name} => 'named',
            n is Int => 'int',
            _ => 'other'
        }
        map([[1, 2], [1, 2, 3], {name: 'x'}, 4, null], classify)
    ";
    assert_eq!(
        run(source),
        Value::from(vec![
            Value::from("pair or triple"),
            Value::from("pair or triple"),
            Value::from("named"),
            Value::from("int"),
            Value::from("other"),
        ])
    );
}

#[test]
fn a_binding_read_only_in_unreachable_code_compiles_and_runs() {
    // The arm after `_ is Int` is never reached by `1`, but its code is still
    // emitted, and its read of `f` still needs `f`'s slot.
    let source = r"
        defn f(x) -> x
        match 1 { 2 => 0, _ is Int => 3, (f(9)) => 0 }
    ";
    assert_eq!(run(source), Value::Int(3));
}

#[test]
fn a_discarded_statement_value_does_not_change_the_result() {
    let source = r"
        def x = 1
        x
        []
        {}
        fn -> 0
        x + 1
    ";
    assert_eq!(run(source), Value::Int(2));
}

#[test]
fn an_unused_closure_with_captures_does_not_change_the_result() {
    let source = r"
        defn make(x) -> {
            def y = x + 1
            fn -> x + y
            def unused = fn -> [x, y]
            y
        }
        make(1)
    ";
    assert_eq!(run(source), Value::Int(2));
}

#[test]
fn a_closure_kept_after_a_pruned_sibling_is_the_right_function() {
    // Pruning the unused closure shifts the kept one's function from second to
    // first among `make`'s nested functions; calling it must still run
    // `fn -> [x]`.
    let source = r"
        defn make(x) -> {
            fn -> x
            fn -> [x]
        }
        make(1)()
    ";
    assert_eq!(run(source), Value::array([1]));
}

// Code shape, under pinned options.

#[test]
fn a_discarded_closure_with_captures_is_pruned_with_its_capture_loads() {
    let make = Script::new(
        r"
        defn make(x, y) -> {
            fn -> x + y
            1
        }
    ",
    )
    .code(DISCARD)
    .nested(0);
    assert!(!creates_a_closure(&make.code), "{make:?}");
    assert_eq!(
        make.nested_count(),
        0,
        "the closure is no longer created, so not compiled in: {make:?}"
    );
    assert!(
        !loads_a_local(&make.code),
        "its captures are no longer loaded: {make:?}"
    );
}

#[test]
fn a_dead_closure_with_captures_frees_the_stores_it_read() {
    let emitted = Script::new(
        r"
        def x = 1
        def y = 2
        def f = fn -> x + y
        3
    ",
    )
    .code(BOTH);
    assert!(!creates_a_closure(&emitted.code), "{emitted:?}");
    assert_eq!(emitted.nested_count(), 0, "{emitted:?}");
    assert!(
        !stores(&emitted.code),
        "with the closure gone, nothing reads `x` or `y`: {emitted:?}"
    );
}

#[test]
fn a_closure_over_a_capture_is_pruned() {
    // The innermost function captures `a` from the middle one's own captures,
    // not from its parameters or bindings.
    let middle = Script::new(
        r"
        fn a -> fn -> {
            fn -> a
            1
        }
    ",
    )
    .code(DISCARD)
    .nested(0)
    .nested(0);
    assert!(!creates_a_closure(&middle.code), "{middle:?}");
    assert_eq!(middle.nested_count(), 0, "{middle:?}");
    assert!(
        !loads_a_local(&middle.code),
        "its capture is no longer loaded: {middle:?}"
    );
}

#[test]
fn only_the_unused_one_of_two_sibling_closures_is_pruned() {
    let make = Script::new(
        r"
        defn make(x) -> {
            fn -> x
            fn -> [x]
        }
        make
    ",
    )
    .code(DISCARD)
    .nested(0);
    assert_eq!(make.nested_count(), 1, "{make:?}");
    let kept = make.nested(0);
    assert!(
        kept.code.contains(&Bytecode::MakeArray(1)),
        "the kept function is the one building an Array: {kept:?}"
    );
}

#[test]
fn a_used_closure_with_captures_stays() {
    let make = Script::new(
        r"
        defn make(x) -> fn -> x
        make
    ",
    )
    .code(BOTH)
    .nested(0);
    assert!(creates_a_closure(&make.code), "{make:?}");
    assert_eq!(make.nested_count(), 1, "{make:?}");
}

#[test]
fn a_discarded_closure_with_captures_stays_without_discard_elimination() {
    let make = Script::new(
        r"
        defn make(x) -> {
            fn -> x
            1
        }
    ",
    )
    .code(DEAD_STORE)
    .nested(0);
    assert!(creates_a_closure(&make.code), "{make:?}");
    assert_eq!(make.nested_count(), 1, "{make:?}");
}

#[test]
fn a_dead_store_becomes_a_pop() {
    let emitted = Script::new(
        r"
        def x = 1
        2
    ",
    )
    .code(DEAD_STORE);
    assert_eq!(
        emitted.code,
        [
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::Pop,
            Bytecode::PushInt(2),
        ],
        "the value is still loaded, then discarded: {emitted:?}"
    );
}

#[test]
fn a_dead_store_and_its_load_both_go() {
    let emitted = Script::new(
        r"
        def x = 1
        2
    ",
    )
    .code(BOTH);
    assert_eq!(emitted.code, [Bytecode::Pop, Bytecode::PushInt(2)]);
}

#[test]
fn a_chain_of_dead_stores_all_go() {
    // Removing the load of `a` into `b` leaves `a` unread too.
    let emitted = Script::new(
        r"
        def a = 1
        def b = a
        def c = b
        2
    ",
    )
    .code(BOTH);
    assert_eq!(emitted.code, [Bytecode::Pop, Bytecode::PushInt(2)]);
}

#[test]
fn a_discarded_load_goes_without_dead_store_elimination() {
    let emitted = Script::new(
        r"
        def x = 1
        x
        2
    ",
    )
    .code(DISCARD);
    assert_eq!(
        emitted.code,
        [
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::DefLocal(0),
            Bytecode::PushInt(2),
        ],
        "the statement `x` is gone, but the store stays: {emitted:?}"
    );
}

#[test]
fn a_store_that_is_read_stays() {
    let emitted = Script::new(
        r"
        def x = 1
        x
    ",
    )
    .code(BOTH);
    assert!(stores(&emitted.code), "{emitted:?}");
}

#[test]
fn an_exported_store_stays() {
    let explicit = Script::new(
        r"
        export def x = 1
        2
    ",
    )
    .code(BOTH);
    assert!(stores(&explicit.code), "{explicit:?}");

    let implicit = Script::new(
        r"
        def x = 1
        2
    ",
    )
    .implicit_export()
    .code(BOTH);
    assert!(stores(&implicit.code), "{implicit:?}");
}

#[test]
fn a_store_read_on_one_branch_stays() {
    let emitted = Script::new(
        r"
        def x = 1
        if true: x else: 2
    ",
    )
    .code(BOTH);
    assert!(stores(&emitted.code), "{emitted:?}");
}

#[test]
fn a_shadowed_store_that_is_never_read_goes() {
    let emitted = Script::new(
        r"
        def x = 1
        do {
            def x = 2
            x
        }
    ",
    )
    .code(BOTH);
    assert_eq!(
        store_count(&emitted.code),
        1,
        "only the shadow, which is read, is stored: {emitted:?}"
    );
}

#[test]
fn a_computed_dead_value_is_still_computed() {
    let emitted = Script::new(
        r"
        def x = 1 / 0
        2
    ",
    )
    .code(BOTH);
    assert_eq!(
        emitted.code,
        [
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::PushInt(0),
            Bytecode::Divide,
            Bytecode::Pop,
            Bytecode::PushInt(2),
        ],
        "only the store goes: {emitted:?}"
    );
}

#[test]
fn a_dead_function_is_pruned() {
    let emitted = Script::new(
        r"
        def f = fn x -> x
        2
    ",
    )
    .code(BOTH);
    assert_eq!(emitted.code, [Bytecode::Pop, Bytecode::PushInt(2)]);
    assert_eq!(
        emitted.nested_count(),
        0,
        "the function is no longer created, so not compiled in: {emitted:?}"
    );
}

#[test]
fn an_unused_parameter_is_discarded() {
    let lambda = Script::new("fn x -> 1").code(BOTH).nested(0);
    assert_eq!(
        lambda.code,
        [Bytecode::Pop, Bytecode::Pop, Bytecode::PushInt(1)],
        "the argument and the function itself are popped: {lambda:?}"
    );
}

#[test]
fn a_pop_reached_by_a_jump_stays() {
    // The value the `if` leaves is popped where its branches join, so the pop
    // belongs to neither branch's load.
    let emitted = Script::new(
        r"
        if true: 1 else: 2
        3
    ",
    )
    .code(BOTH);
    assert_eq!(
        emitted.count(&Bytecode::Pop),
        2,
        "the top-level's own pop, and the statement's: {emitted:?}"
    );
    assert!(emitted.code.contains(&Bytecode::PushInt(1)), "{emitted:?}");
    assert!(emitted.code.contains(&Bytecode::PushInt(2)), "{emitted:?}");
}
