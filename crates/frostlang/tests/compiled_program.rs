//! `CompiledProgram`, a program not yet asserted well-formed, and the `bytecode`
//! accessors that reach the `CompiledFunction` behind it or behind a `Closure`.

use std::sync::Arc;

use frostlang::bytecode::{
    CompiledFunction, closure_function, program_from_function, program_function,
};
use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{CompiledProgram, Value, Vm};

/// The function `source` compiles to.
fn compiled(source: &str) -> Arc<CompiledFunction> {
    let closure = compile_program("program.frst", source, CompilerOptions::new())
        .unwrap_or_else(|errors| panic!("{source:?} compiles:\n{}", errors.render_plain()))
        .code
        .into_closure()
        .expect("captures nothing");
    Arc::clone(closure_function(&closure))
}

/// The tail value of running `program`.
fn run(program: CompiledProgram) -> Value {
    let closure = program
        .assert_well_formed()
        .into_closure()
        .expect("captures nothing");
    Vm::factory()
        .build(closure)
        .run()
        .unwrap_or_else(|failure| panic!("runs: {}", failure.error()))
        .tail()
        .clone()
}

#[test]
fn a_program_made_from_a_function_runs_once_asserted_well_formed() {
    let program = program_from_function(compiled("6 * 7"));
    assert_eq!(run(program), Value::Int(42));
}

#[test]
fn a_program_holds_the_very_function_it_was_made_from() {
    let function = compiled("1");
    let program = program_from_function(Arc::clone(&function));
    assert!(
        Arc::ptr_eq(program_function(&program), &function),
        "`program_function` gives back the function, not a copy"
    );
}

#[test]
fn a_cloned_program_shares_its_function() {
    let program = program_from_function(compiled("1"));
    let clone = program.clone();
    assert!(Arc::ptr_eq(
        program_function(&program),
        program_function(&clone)
    ));
    assert_eq!(run(clone), Value::Int(1), "the clone runs on its own");
}

#[test]
fn a_closure_runs_the_function_its_program_was_made_from() {
    let function = compiled("2");
    let closure = program_from_function(Arc::clone(&function))
        .assert_well_formed()
        .into_closure()
        .expect("captures nothing");
    assert!(
        Arc::ptr_eq(closure_function(&closure), &function),
        "`closure_function` gives back the function, not a copy"
    );
}
