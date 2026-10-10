//! The whole path a host takes to embed Frost: compile source, run it, and read
//! its result and exports.

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{Value, Vm};

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
