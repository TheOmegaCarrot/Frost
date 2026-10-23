//! Tests for `frostlang::native::MutableBundle`: building a Map of natives over
//! one shared state, locking it, calling back into Frost between locks,
//! sharing it with the host, and poisoning.
//!
//! `Script` runs every optimization permutation against the same captured
//! value, so a test whose state changes captures a factory making a fresh
//! bundle per run, or keeps to operations that repeat identically.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use frostlang::native::{FrostArg, Function, MutableBundle, PoisonedState, StateLock};
use frostlang::{Arity, FrostType, MapKey, Param, Params, Value};

use crate::script::{Script, optimization_permutations};

const NO_PARAMS: Params = Params::new(&[]);
const AMOUNT: Params = Params::new(&[<i64 as FrostArg>::PARAM.named("amount")]);
const CALLBACK: Params = Params::new(&[Param::of(FrostType::FUNCTION).named("callback")]);

/// A counter bundle over `count`: `get`, `add(amount)`, and `apply(callback)`,
/// which sets the count to `callback(count)`.
fn counter(count: MutableBundle<i64>) -> Value {
    count
        .function("counter.get", NO_PARAMS, |ctx, _| Ok(*ctx.lock()?))
        .function("counter.add", AMOUNT, |ctx, mut args| {
            let amount: i64 = args.take()?;
            let mut count = ctx.lock()?;
            *count += amount;
            Ok(*count)
        })
        .function("counter.apply", CALLBACK, |ctx, mut args| {
            let callback: Function = args.take()?;
            let count = *ctx.lock()?;
            let applied = ctx.native().invoke(&callback, [Value::Int(count)])?;
            let applied = applied.as_int().ok_or("counter.apply needs an Int back")?;
            *ctx.lock()? = applied;
            Ok(applied)
        })
        .build()
}

/// `make_counter(start)`: a fresh counter per call, owning its count.
pub(crate) fn make_counter() -> Value {
    Value::native("make_counter", Arity::Exact(1), |_, args| {
        let start = args[0].as_int().expect("an Int start");
        Ok(counter(MutableBundle::new(start)))
    })
}

fn run(source: &str) -> Value {
    Script::new(source)
        .capture("make_counter", make_counter())
        .run()
}

fn raises(source: &str) -> String {
    Script::new(source)
        .capture("make_counter", make_counter())
        .raises()
}

// ============================================================
// Building
// ============================================================

#[test]
fn each_function_is_keyed_by_its_name_after_the_last_dot() {
    let bundle = MutableBundle::new(())
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
        raises("make_counter(0).add('one')"),
        "Function counter.add requires Int as argument 1 (amount), got String"
    );
}

#[test]
fn an_empty_bundle_is_an_empty_map() {
    assert_eq!(MutableBundle::new(()).build(), Value::map::<&str, 0>([]));
}

#[test]
#[should_panic(expected = "the bundle already has a function keyed get")]
fn a_second_function_with_the_same_key_panics() {
    let _ = MutableBundle::new(())
        .function("a.get", NO_PARAMS, |_, _| Ok(Value::Null))
        .function("b.get", NO_PARAMS, |_, _| Ok(Value::Null));
}

/// A group: the function `name`, returning Null, added to `bundle`.
fn group(name: &'static str) -> impl FnOnce(MutableBundle<()>) -> MutableBundle<()> {
    move |bundle| bundle.function(name, NO_PARAMS, |_, _| Ok(Value::Null))
}

/// The keys of `bundle`, in order.
fn keys(bundle: &Value) -> Vec<String> {
    let map = bundle.as_map().expect("a bundle is a Map");
    map.keys().map(ToString::to_string).collect()
}

#[test]
fn groups_are_added_with_with_and_with_if_its_condition_holds() {
    let bundle = |extra: bool| {
        MutableBundle::new(())
            .with(group("thing.always"))
            .with_if(extra, group("thing.sometimes"))
            .with_if(!extra, group("thing.otherwise"))
            .build()
    };
    assert_eq!(keys(&bundle(false)), ["always", "otherwise"]);
    assert_eq!(keys(&bundle(true)), ["always", "sometimes"]);
}

// ============================================================
// Calling
// ============================================================

#[test]
fn functions_share_one_state_across_calls() {
    let source = r"
        def c = make_counter(10)
        c.add(5)
        c.add(-2)
        [c.get(), c.add(1), c.get()]
    ";
    assert_eq!(run(source), Value::array([13, 14, 14]));
}

#[test]
fn bundles_do_not_share_state_with_each_other() {
    let source = r"
        def a = make_counter(0)
        def b = make_counter(100)
        a.add(1)
        [a.get(), b.get()]
    ";
    assert_eq!(run(source), Value::array([1, 100]));
}

#[test]
fn arguments_are_checked_by_params_and_taken_by_args() {
    assert_eq!(
        raises("make_counter(0).add()"),
        "Function counter.add expects 1 argument, but was called with 0"
    );
    assert_eq!(
        raises("make_counter(0).get(1)"),
        "Function counter.get expects 0 arguments, but was called with 1"
    );
}

#[test]
fn results_convert_through_into_native_result() {
    let unit = MutableBundle::new(())
        .function("thing.nothing", NO_PARAMS, |_, _| Ok(()))
        .build();
    let result = Script::new("t.nothing()").capture("t", unit).run();
    assert_eq!(result, Value::Null, "() returns Null");
}

#[test]
fn a_function_calls_back_into_frost_between_locks() {
    let source = r"
        def c = make_counter(3)
        c.apply(fn n -> n * 10)
    ";
    assert_eq!(run(source), Value::Int(30));
}

#[test]
fn a_callback_may_reenter_the_same_bundle() {
    // The callback reads and changes the counter while `apply` is running,
    // which `apply` allows because it does not hold the state meanwhile.
    let source = r"
        def c = make_counter(3)
        def result = c.apply(fn n -> c.add(n) + 1)
        [result, c.get()]
    ";
    assert_eq!(run(source), Value::array([7, 7]));
}

#[test]
fn a_callback_error_propagates() {
    assert_eq!(
        raises("make_counter(0).apply(fn n -> n + 'x')"),
        raises("0 + 'x'"),
        "the callback's own error"
    );
}

#[test]
fn a_held_state_still_offers_the_native_context() {
    let named = MutableBundle::new(())
        .function("thing.whoami", NO_PARAMS, |ctx, _| {
            let state = ctx.lock()?;
            Ok(state.native().name().to_string())
        })
        .build();
    let result = Script::new("t.whoami()").capture("t", named).run();
    assert_eq!(result, Value::from("thing.whoami"));
}

// ============================================================
// Sharing with the host
// ============================================================

#[test]
fn the_host_sees_what_frost_changes() {
    let shared = Arc::new(StateLock::new(0_i64));
    let set = MutableBundle::from_shared(Arc::clone(&shared))
        .function("thing.set", AMOUNT, |ctx, mut args| {
            let value: i64 = args.take()?;
            *ctx.lock()? = value;
            Ok(())
        })
        .build();
    Script::new("t.set(7)").capture("t", set).run();
    assert_eq!(*shared.lock().expect("not poisoned"), 7);
}

#[test]
fn frost_sees_what_the_host_changes() {
    let shared = Arc::new(StateLock::new(0_i64));
    let bundle = counter(MutableBundle::from_shared(Arc::clone(&shared)));
    *shared.lock().expect("not poisoned") = 41;
    let result = Script::new("c.get()").capture("c", bundle).run();
    assert_eq!(result, Value::Int(41));
}

// ============================================================
// Poisoning
// ============================================================

/// A bundle whose `panic_holding` panics while holding the state, and whose
/// `panic_free` panics without holding it.
fn panicking(state: Arc<StateLock<i64>>) -> Value {
    MutableBundle::from_shared(state)
        .function("thing.get", NO_PARAMS, |ctx, _| Ok(*ctx.lock()?))
        .function("thing.panic_holding", NO_PARAMS, |ctx, _| {
            let _held = ctx.lock()?;
            panic!("a bug while holding the state");
            #[expect(unreachable_code, reason = "the type the closure returns")]
            Ok(())
        })
        .function("thing.panic_free", NO_PARAMS, |_, _| {
            panic!("a bug while not holding the state");
            #[expect(unreachable_code, reason = "the type the closure returns")]
            Ok(())
        })
        .build()
}

/// Runs `source`, which must panic, under one optimization permutation.
fn run_panicking(source: &str, bundle: &Value) {
    let optimization = optimization_permutations()
        .next()
        .expect("there is always one permutation");
    let script = Script::new(source).capture("t", bundle.clone());
    let outcome = catch_unwind(AssertUnwindSafe(|| script.run_under(optimization)));
    assert!(outcome.is_err(), "{source:?} should panic");
}

#[test]
fn a_panic_while_holding_the_state_poisons_it() {
    let shared = Arc::new(StateLock::new(5_i64));
    let bundle = panicking(Arc::clone(&shared));
    run_panicking("t.panic_holding()", &bundle);

    let message = Script::new("t.get()").capture("t", bundle).raises();
    assert_eq!(
        message,
        "Function thing.get cannot run: a panic poisoned its state"
    );
    assert_eq!(
        shared.lock().err(),
        Some(PoisonedState),
        "the host's lock fails too"
    );
}

#[test]
fn a_panic_while_not_holding_the_state_leaves_it_usable() {
    let shared = Arc::new(StateLock::new(5_i64));
    let bundle = panicking(Arc::clone(&shared));
    run_panicking("t.panic_free()", &bundle);

    let result = Script::new("t.get()").capture("t", bundle).run();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn poisoned_state_displays_as_an_error() {
    assert_eq!(PoisonedState.to_string(), "a panic poisoned the state");
}
