//! Tests for `frostlang::native::Bundle`: building a Map of natives over one
//! state that never changes, reading it while calling back into Frost, and
//! sharing it with the host.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use frostlang::native::{Bundle, FrostArg};
use frostlang::{FrostType, MapKey, Param, Params, Value};

use crate::script::{Script, optimization_permutations};

const NO_PARAMS: Params = Params::new(&[]);
const INDEX: Params = Params::new(&[<usize as FrostArg>::PARAM.named("index")]);
const CALLBACK: Params = Params::new(&[Param::of(FrostType::FUNCTION).named("callback")]);

/// A table bundle over `entries`: `get(index)`, `len()`, and `apply(callback)`,
/// which returns an Array of `callback(entry)` for each entry.
fn table(entries: Vec<i64>) -> Value {
    Bundle::new(entries)
        .function("table.get", INDEX, |ctx, mut args| {
            let index: usize = args.take()?;
            Ok(ctx.state().get(index).copied())
        })
        .function("table.len", NO_PARAMS, |ctx, _| Ok(ctx.state().len()))
        .function("table.apply", CALLBACK, |ctx, mut args| {
            let callback: Value = args.take()?;
            let entries = ctx.state();
            entries
                .iter()
                .map(|&entry| ctx.native().invoke(&callback, [Value::Int(entry)]))
                .collect::<Result<Value, _>>()
        })
        .build()
}

fn run(source: &str) -> Value {
    Script::new(source)
        .capture("t", table(vec![10, 20, 30]))
        .run()
}

fn raises(source: &str) -> String {
    Script::new(source)
        .capture("t", table(vec![10, 20, 30]))
        .raises()
}

// ============================================================
// Building
// ============================================================

#[test]
fn each_function_is_keyed_by_its_name_after_the_last_dot() {
    let bundle = Bundle::new(())
        .function("thing.part.read", NO_PARAMS, |_, _| Ok(Value::Null))
        .function("undotted", NO_PARAMS, |_, _| Ok(Value::Null))
        .build();
    let map = bundle.as_map().expect("a bundle is a Map");
    let keys: Vec<&MapKey> = map.keys().collect();
    assert_eq!(keys, [&MapKey::from("read"), &MapKey::from("undotted")]);
}

#[test]
fn each_function_keeps_its_full_name() {
    // The full name is what errors show.
    assert_eq!(
        raises("t.get('one')"),
        "Function table.get requires Int as argument 1 (index), got String"
    );
}

#[test]
fn an_empty_bundle_is_an_empty_map() {
    assert_eq!(Bundle::new(()).build(), Value::map::<&str, 0>([]));
}

#[test]
#[should_panic(expected = "the bundle already has a function keyed get")]
fn a_second_function_with_the_same_key_panics() {
    let _ = Bundle::new(())
        .function("a.get", NO_PARAMS, |_, _| Ok(Value::Null))
        .function("b.get", NO_PARAMS, |_, _| Ok(Value::Null));
}

#[test]
fn functions_can_be_added_conditionally() {
    let with = |extra: bool| {
        let mut bundle =
            Bundle::new(()).function("thing.always", NO_PARAMS, |_, _| Ok(Value::Null));
        if extra {
            bundle = bundle.function("thing.sometimes", NO_PARAMS, |_, _| Ok(Value::Null));
        }
        bundle.build().as_map().expect("a bundle is a Map").len()
    };
    assert_eq!(with(false), 1);
    assert_eq!(with(true), 2);
}

// ============================================================
// Calling
// ============================================================

#[test]
fn functions_share_one_state() {
    assert_eq!(
        run("[t.len(), t.get(0), t.get(2)]"),
        Value::array([3, 10, 30])
    );
}

#[test]
fn bundles_do_not_share_state_with_each_other() {
    let result = Script::new("[a.get(0), b.get(0)]")
        .capture("a", table(vec![1]))
        .capture("b", table(vec![2]))
        .run();
    assert_eq!(result, Value::array([1, 2]));
}

#[test]
fn arguments_are_checked_by_params_and_taken_by_args() {
    assert_eq!(
        raises("t.get()"),
        "Function table.get expects 1 arguments, but was called with 0"
    );
    assert_eq!(
        raises("t.len(1)"),
        "Function table.len expects 0 arguments, but was called with 1"
    );
    assert_eq!(
        raises("t.apply(1)"),
        "Function table.apply requires Function as argument 1 (callback), got Int"
    );
}

#[test]
fn results_convert_through_into_native_result() {
    assert_eq!(run("t.get(3)"), Value::Null, "None returns Null");

    let unit = Bundle::new(())
        .function("thing.nothing", NO_PARAMS, |_, _| Ok(()))
        .build();
    let result = Script::new("u.nothing()").capture("u", unit).run();
    assert_eq!(result, Value::Null, "() returns Null");
}

#[test]
fn a_function_reads_the_state_while_calling_back_into_frost() {
    assert_eq!(run("t.apply(fn n -> n + 1)"), Value::array([11, 21, 31]));
}

#[test]
fn a_callback_may_reenter_the_same_bundle() {
    // `apply` reads the state throughout, while each callback reads it too.
    let source = r"
        t.apply(fn n -> [n, t.len(), t.get(0)])
    ";
    assert_eq!(
        run(source),
        Value::array([
            Value::array([10, 3, 10]),
            Value::array([20, 3, 10]),
            Value::array([30, 3, 10]),
        ])
    );
}

#[test]
fn a_callback_error_propagates() {
    assert_eq!(
        raises("t.apply(fn n -> n + 'x')"),
        raises("10 + 'x'"),
        "the callback's own error"
    );
}

#[test]
fn the_native_context_is_reachable_while_reading_the_state() {
    let named = Bundle::new(String::from("state"))
        .function("thing.whoami", NO_PARAMS, |ctx, _| {
            let state = ctx.state();
            Ok(format!("{} of {}", state, ctx.native().name()))
        })
        .build();
    let result = Script::new("t.whoami()").capture("t", named).run();
    assert_eq!(result, Value::from("state of thing.whoami"));
}

// ============================================================
// Sharing with the host
// ============================================================

#[test]
fn the_bundle_and_the_host_share_one_state() {
    // The state never changes, but what it shares through interior
    // mutability shows that the bundle holds the host's own state, not a copy.
    let shared = Arc::new(AtomicI64::new(0));
    let bundle = Bundle::from_shared(Arc::clone(&shared))
        .function("thing.get", NO_PARAMS, |ctx, _| {
            Ok(ctx.state().load(Ordering::Relaxed))
        })
        .function("thing.set", INDEX, |ctx, mut args| {
            let value: i64 = args.take()?;
            ctx.state().store(value, Ordering::Relaxed);
            Ok(())
        })
        .build();

    shared.store(41, Ordering::Relaxed);
    let script = Script::new("t.get()").capture("t", bundle.clone());
    assert_eq!(script.run(), Value::Int(41), "Frost sees the host's change");

    Script::new("t.set(7)").capture("t", bundle).run();
    assert_eq!(
        shared.load(Ordering::Relaxed),
        7,
        "the host sees Frost's change"
    );
}

// ============================================================
// Panics
// ============================================================

#[test]
fn a_panic_leaves_the_bundle_usable() {
    // With no lock, there is nothing to poison.
    let bundle = Bundle::new(5_i64)
        .function("thing.get", NO_PARAMS, |ctx, _| Ok(*ctx.state()))
        .function("thing.panic", NO_PARAMS, |ctx, _| {
            let _read = ctx.state();
            panic!("a bug while reading the state");
            #[expect(unreachable_code, reason = "the type the closure returns")]
            Ok(())
        })
        .build();

    let optimization = optimization_permutations()
        .next()
        .expect("there is always one permutation");
    let script = Script::new("t.panic()").capture("t", bundle.clone());
    let outcome = catch_unwind(AssertUnwindSafe(|| script.run_under(optimization)));
    assert!(outcome.is_err(), "t.panic() should panic");

    let result = Script::new("t.get()").capture("t", bundle).run();
    assert_eq!(result, Value::Int(5));
}
