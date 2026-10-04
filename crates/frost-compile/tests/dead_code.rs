//! Dead code: a store no later code reads is discarded instead
//! (`dead_store_eliminate`), and a value loaded only to be discarded is not
//! loaded (`discard_eliminate`).
//!
//! The harness runs each behavioral case under every optimization permutation,
//! so neither is checked ever to change what a program does. The code
//! assertions pin the options they are about.

mod common;

use common::{Script, UNOPTIMIZED, raises, run};
use frost_compile::OptimizationOptions;
use frost_runtime::{Bytecode, Value};

const DEAD_STORE: OptimizationOptions = OptimizationOptions {
    dead_store_eliminate: true,
    ..UNOPTIMIZED
};

const DISCARD: OptimizationOptions = OptimizationOptions {
    discard_eliminate: true,
    ..UNOPTIMIZED
};

const BOTH: OptimizationOptions = OptimizationOptions {
    dead_store_eliminate: true,
    discard_eliminate: true,
    ..UNOPTIMIZED
};

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
        map [[1, 2], [1, 2, 3], {name: 'x'}, 4, null] with classify
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
    // The arm after `_` is never reached, but its code is still emitted, and
    // its read of `f` still needs `f`'s slot.
    let source = r"
        defn f(x) -> x
        match 1 { 2 => 0, _ => 3, (f(9)) => 0 }
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

// Code shape, under pinned options.

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
