//! The facade alone is enough to embed Frost: a host compiles source and runs
//! it through `frostlang` paths only, with no direct dependency on the crates
//! it gathers.

use frostlang::{CompilerOptions, OptimizationOptions, Value, Vm, compile_program};

#[test]
fn compiles_and_runs_source_through_the_facade() {
    let source = r"
        defn double(x) -> x * 2
        export def answer = double(21)
        answer + 1
    ";
    let options = CompilerOptions {
        optimization_options: OptimizationOptions::ALL,
        implicit_export: false,
    };
    let closure = compile_program("facade.frst", source, options)
        .expect("the source compiles")
        .code
        .into_closure()
        .expect("the program needs no host captures");
    let result = Vm::factory()
        .build(closure)
        .expect("the VM builds")
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
