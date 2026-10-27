//! What a host can learn of a Frost function it holds as a [`Closure`]:
//! its name and arity (as a `NativeFunction` reports its own) and its origin.
//! Also the name and arity a native reads of any Function it is given.

use crate::script;

use std::sync::Arc;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::native::Function;
use frostlang::{Arity, BacktraceFrame, Closure, Value, Vm, frost_fn};
use script::{Script, optimization_permutations};

/// The Closure the script `source` evaluates to under each optimization
/// permutation; it must evaluate to one.
/// A Closure is equal only to itself, so each run's Closure is checked on its
/// own.
#[track_caller]
fn closures(source: &str) -> Vec<Arc<Closure>> {
    optimization_permutations()
        .map(|options| match Script::new(source).run_under(options) {
            Value::Closure(closure) => closure,
            other => panic!("{source:?} should give a Closure, but gave {other:?}"),
        })
        .collect()
}

/// The frame `closure`'s own call adds to the backtrace when the host calls it
/// with `args`, which must make it raise.
#[track_caller]
fn own_frame(closure: &Arc<Closure>, args: Vec<Value>) -> BacktraceFrame {
    let failure = match Vm::factory().build(Arc::clone(closure)).run_with_args(args) {
        Ok(result) => panic!(
            "calling {} should raise, but gave {:?}",
            closure.name(),
            result.tail()
        ),
        Err(failure) => failure,
    };
    failure
        .error()
        .backtrace()
        .first()
        .cloned()
        .expect("a raising call leaves its own frame")
}

// --- Arity ---

#[test]
fn a_closure_reports_every_arity_a_lambda_can_have() {
    for (source, arity) in [
        ("fn -> 1", Arity::Exact(0)),
        ("fn x -> x", Arity::Exact(1)),
        ("fn x, y, z -> x", Arity::Exact(3)),
        ("fn ...rest -> rest", Arity::AtLeast(0)),
        ("fn x, y, ...rest -> rest", Arity::AtLeast(2)),
        ("$($ * 2)", Arity::Exact(1)),
        ("$($1 + $2)", Arity::Exact(2)),
        // A placeholder left unused still takes its argument.
        ("$($2)", Arity::Exact(2)),
        ("$(len($$))", Arity::AtLeast(0)),
        ("$($1 + len($$))", Arity::AtLeast(1)),
    ] {
        for closure in closures(source) {
            assert_eq!(closure.arity(), arity, "{source}");
        }
    }
}

#[test]
fn a_global_written_in_frost_reports_its_arity() {
    // `call` takes a function, and optionally its arguments.
    for closure in closures("call") {
        assert_eq!(closure.arity(), Arity::Between(1, 2), "call");
    }
}

// --- Name ---

#[test]
fn a_closure_is_named_as_its_own_backtrace_frame_is() {
    for (source, name) in [
        // Named by the script.
        ("fn twice(x) -> x * 2", "twice"),
        (
            r"
            defn twice(x) -> x * 2
            twice
            ",
            "twice",
        ),
        // Named by the `def` that makes it.
        (
            r"
            def twice = fn x -> x * 2
            twice
            ",
            "twice",
        ),
        (
            r"
            def twice = $($ * 2)
            twice
            ",
            "twice",
        ),
        (
            r"
            def twice is Function = fn x -> x * 2
            twice
            ",
            "twice",
        ),
        // A later binding of it renames nothing.
        (
            r"
            defn twice(x) -> x * 2
            def again = twice
            again
            ",
            "twice",
        ),
        // Neither.
        ("[fn x -> x * 2][0]", "<lambda>"),
        ("$($ * 2)", "<lambda>"),
        (
            r"
            def [twice] = [fn x -> x * 2]
            twice
            ",
            "<lambda>",
        ),
    ] {
        for closure in closures(source) {
            assert_eq!(closure.name(), name, "{source}");
            // `null * 2` raises inside the function itself.
            let frame = own_frame(&closure, vec![Value::Null]);
            assert_eq!(frame.name(), closure.name(), "{source}: its frame");
        }
    }
}

#[test]
fn captures_do_not_change_what_a_closure_reports() {
    // `offset` is a parameter of the enclosing function, so it is captured
    // under every optimization.
    let source = r"
        defn shifter(offset) -> fn shifted(x, ...more) -> x + offset
        shifter(10)
    ";
    for closure in closures(source) {
        assert_eq!(closure.name(), "shifted", "its name");
        assert_eq!(closure.arity(), Arity::AtLeast(1), "its arity");
        let frame = own_frame(&closure, vec![Value::from("not a number")]);
        assert_eq!(frame.name(), "shifted", "its frame");
    }
}

#[test]
fn a_programs_closure_is_named_main_and_takes_no_arguments() {
    let program = compile_program("main.frst", "1 + 2", CompilerOptions::new())
        .expect("the source compiles")
        .code;
    let closure = program.into_closure().expect("it captures nothing");
    assert_eq!(closure.name(), "<main>", "its name");
    assert_eq!(closure.arity(), Arity::Exact(0), "its arity");
}

// --- Origin ---

/// `twice`, from a script compiled as `scripts/game.frst`, stripped of its
/// debug info if `stripped`.
fn twice_from_game(stripped: bool) -> Arc<Closure> {
    let source = r"
        defn twice(x) -> x * 2
        twice
    ";
    let program = compile_program("scripts/game.frst", source, CompilerOptions::new())
        .expect("the source compiles")
        .code;
    let program = if stripped { program.strip() } else { program };
    let result = Vm::factory()
        .build(program.into_closure().expect("it captures nothing"))
        .run()
        .expect("the script runs");
    match result.tail() {
        Value::Closure(twice) => Arc::clone(twice),
        other => panic!("the script should give a Closure, but gave {other:?}"),
    }
}

#[test]
fn a_closure_reports_the_base_name_of_the_file_it_was_compiled_from() {
    for stripped in [false, true] {
        let twice = twice_from_game(stripped);
        assert_eq!(
            twice.origin(),
            Some("game.frst"),
            "the base name only, stripped: {stripped}"
        );
        let frame = own_frame(&twice, vec![Value::Null]);
        assert_eq!(
            frame.origin(),
            twice.origin(),
            "as its frame reports it, stripped: {stripped}"
        );
    }
}

#[test]
fn a_closure_compiled_from_no_file_has_no_origin() {
    // The generated globals are written in Frost, but compiled from no file.
    for closure in closures("curry(fn x, y -> x + y, 1)") {
        assert_eq!(closure.origin(), None, "the Closure curry returns");
    }
}

// --- Any Function, native or Frost ---

/// `describe(function)`: the function's name, then its arity as its kind and
/// bounds, as a native taking any Function sees them.
#[frost_fn]
fn describe(function: Function) -> Vec<Value> {
    let count = |n: usize| Value::Int(i64::try_from(n).expect("a count fits an Int"));
    let arity = match function.arity() {
        Arity::Exact(n) => vec![Value::from("Exact"), count(n)],
        Arity::Between(low, high) => vec![Value::from("Between"), count(low), count(high)],
        Arity::AtLeast(n) => vec![Value::from("AtLeast"), count(n)],
    };
    std::iter::once(Value::from(function.name()))
        .chain(arity)
        .collect()
}

#[test]
fn a_native_reads_the_name_and_arity_of_any_function() {
    for (argument, described) in [
        ("fn twice(x) -> x * 2", "['twice', 'Exact', 1]"),
        ("fn ...rest -> rest", "['<lambda>', 'AtLeast', 0]"),
        ("call", "['call', 'Between', 1, 2]"),
        ("len", "['len', 'Exact', 1]"),
    ] {
        let tail = Script::new(&format!("describe({argument})"))
            .capture("describe", describe_frost().build())
            .run();
        assert_eq!(tail, script::run(described), "{argument}");
    }
}
