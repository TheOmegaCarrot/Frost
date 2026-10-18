//! The whole path a host takes to embed Frost: compile source, run it, and read
//! its result and exports.

use std::collections::BTreeMap;

use frostlang::compile::{CompilerOptions, compile_in_scope, compile_program};
use frostlang::{ProgramParts, Value, Vm};

#[test]
fn a_host_closes_one_compiled_program_over_different_captures() {
    // Compiled once, a program is cloned and closed once per environment.
    let program = compile_in_scope("scaled.frst", "x * 10", CompilerOptions::new(), &["x"])
        .expect("the source compiles")
        .code;
    let run_with = |x: i64| {
        let captures = BTreeMap::from([(String::from("x"), Value::Int(x))]);
        let closure = program
            .clone()
            .close(captures)
            .expect("the capture is supplied");
        Vm::factory()
            .build(closure)
            .run()
            .expect("the program runs")
            .tail()
            .clone()
    };
    assert_eq!(run_with(1), Value::Int(10), "closed over x = 1");
    assert_eq!(run_with(2), Value::Int(20), "closed over x = 2");
}

#[test]
fn a_host_compiles_and_runs_source() {
    let source = r"
        defn double(x) -> x * 2
        export def answer = double(21)
        answer + 1
    ";
    let closure = compile_program("embedding.frst", source, CompilerOptions::new())
        .expect("the source compiles")
        .code
        .into_closure()
        .expect("the program needs no host captures");
    let result = Vm::factory()
        .build(closure)
        .run()
        .expect("the program runs");

    assert_eq!(result.tail(), &Value::Int(43), "the program's tail value");
    let exports: Vec<_> = result.exports().collect();
    assert_eq!(
        exports,
        [("answer", &Value::Int(42))],
        "the program's exports"
    );
}

#[test]
fn a_host_keeps_uniquely_owned_state_between_runs() {
    // The script exports a handler; the host runs it once per event on a recycled
    // Vm, passing the state in and taking the new state back out by value.
    let source = r"
        export defn step(state) -> state + [len(state)]
    ";
    let closure = compile_program("embedding.frst", source, CompilerOptions::new())
        .expect("the source compiles")
        .code
        .into_closure()
        .expect("the program needs no host captures");
    let ProgramParts {
        mut exports,
        mut idle_vm,
        ..
    } = Vm::factory()
        .build(closure)
        .run()
        .expect("the program runs")
        .into_parts();
    let Some((_, Value::Closure(step))) = exports.pop() else {
        panic!("the program exports `step` as a closure");
    };

    let mut state = Vec::new();
    for _ in 0..3 {
        let parts = idle_vm
            .build(step.clone())
            .run_with_args([Value::from(state)])
            .expect("`step` runs")
            .into_parts();
        idle_vm = parts.idle_vm;
        state = parts
            .tail
            .try_into_array()
            .expect("`step` returns an Array")
            .try_into_vec()
            .expect("the returned state is uniquely owned");
    }
    assert_eq!(
        state,
        [Value::Int(0), Value::Int(1), Value::Int(2)],
        "each run appended its step to the state"
    );
}
