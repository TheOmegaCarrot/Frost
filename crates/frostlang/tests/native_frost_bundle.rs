//! `#[frost_bundle]` and `#[frost_mutable_bundle]`: groups of Frost functions
//! over shared state, from the functions of an impl block.
//!
//! The first section checks generated bundles against equivalents built by
//! hand in `native_bundle` and `native_mutable_bundle`, so that the macros are
//! shown to generate the same functions. The rest covers what only the macros
//! decide: each way a function takes the state, keys and names, groups, and
//! `#[cfg]`s.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use frostlang::native::{
    ArgSite, Bundle, BundleCtx, FromArg, MutableBundle, MutableBundleCtx, PoisonedState, StateLock,
};
use frostlang::{
    Arity, FrostError, FrostType, MapKey, NativeCtx, TypeSet, Value, frost_bundle,
    frost_mutable_bundle,
};

use crate::script::{Script, optimization_permutations};
use crate::{native_bundle, native_mutable_bundle};

/// Asserts that a run of each of `sources`, with `name` bound to the
/// generated value, ends exactly as with the hand-built one.
fn assert_alike(name: &str, generated: &Value, hand_built: &Value, sources: &[&str]) {
    for source in sources {
        let outcome = |value: &Value| Script::new(source).capture(name, value.clone()).outcome();
        assert_eq!(
            outcome(generated),
            outcome(hand_built),
            "{source:?} ends alike with either bundle"
        );
    }
}

fn run(name: &str, value: Value, source: &str) -> Value {
    Script::new(source).capture(name, value).run()
}

fn raises(name: &str, value: Value, source: &str) -> String {
    Script::new(source).capture(name, value).raises()
}

/// The keys of `bundle`, a Map, in order.
fn keys(bundle: &Value) -> Vec<String> {
    bundle
        .as_map()
        .expect("a bundle is a Map")
        .keys()
        .map(|key| match key {
            MapKey::String(key) => key.to_string(),
            other => panic!("a bundle's keys are Strings, got {other:?}"),
        })
        .collect()
}

/// Any Function.
struct Callback(Value);

impl FromArg for Callback {
    const TYPES: TypeSet = FrostType::FUNCTION;

    fn from_arg(value: Value, _: &ArgSite<'_>) -> Result<Self, FrostError> {
        Ok(Self(value))
    }
}

// ============================================================
// Alike to hand-built bundles
// ============================================================

/// As `native_bundle::table`.
struct Table(Vec<i64>);

#[frost_bundle(prefix = "table")]
impl Table {
    fn get(&self, index: usize) -> Option<i64> {
        self.0.get(index).copied()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    /// Reads the state while calling back into Frost.
    fn apply(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        callback: Callback,
    ) -> Result<Value, FrostError> {
        self.0
            .iter()
            .map(|&entry| ctx.invoke(&callback.0, [Value::Int(entry)]))
            .collect()
    }
}

fn table(entries: Vec<i64>) -> Value {
    Table::frost_functions(Bundle::new(Table(entries))).build()
}

#[test]
fn a_bundle_over_state_that_never_changes_is_alike() {
    assert_alike(
        "t",
        &table(vec![10, 20, 30]),
        &native_bundle::table(vec![10, 20, 30]),
        &[
            "[t.len(), t.get(0), t.get(2), t.get(3)]",
            "t.apply(fn n -> n + 1)",
            "t.apply(fn n -> [n, t.len(), t.get(0)])",
            "t.apply(fn n -> n + 'x')",
            "t.get('one')",
            "t.get()",
            "t.len(1)",
            "t.apply(1)",
        ],
    );
}

/// As `native_mutable_bundle`'s counter.
struct Counter(i64);

#[frost_mutable_bundle(prefix = "counter")]
impl Counter {
    fn get(&self) -> i64 {
        self.0
    }

    fn add(&mut self, amount: i64) -> i64 {
        self.0 += amount;
        self.0
    }

    /// Locks the state itself, calling back into Frost between locks.
    fn apply(
        #[frost(ctx)] ctx: &mut MutableBundleCtx<'_, Self>,
        callback: Callback,
    ) -> Result<i64, FrostError> {
        let count = ctx.lock()?.0;
        let applied = ctx.native().invoke(&callback.0, [Value::Int(count)])?;
        let applied = applied.as_int().ok_or("counter.apply needs an Int back")?;
        ctx.lock()?.0 = applied;
        Ok(applied)
    }
}

/// `make_counter(start)`, as `native_mutable_bundle::make_counter`.
fn make_counter() -> Value {
    Value::native("make_counter", Arity::Exact(1), |_, args| {
        let start = args[0].as_int().expect("an Int start");
        Ok(Counter::frost_functions(MutableBundle::new(Counter(start))).build())
    })
}

#[test]
fn a_bundle_over_state_that_changes_is_alike() {
    assert_alike(
        "make_counter",
        &make_counter(),
        &native_mutable_bundle::make_counter(),
        &[
            r"
                def c = make_counter(10)
                c.add(5)
                c.add(-2)
                [c.get(), c.add(1), c.get()]
            ",
            r"
                def c = make_counter(3)
                def result = c.apply(fn n -> c.add(n) + 1)
                [result, c.get()]
            ",
            "make_counter(0).apply(fn n -> 'x')",
            "make_counter(0).apply(fn n -> n + 'x')",
            "make_counter(0).add('one')",
            "make_counter(0).add()",
            "make_counter(0).get(1)",
            "make_counter(0).apply(1)",
        ],
    );
}

// ============================================================
// Taking the state
// ============================================================

/// Each way a function of a bundle over state that never changes takes it.
struct Greeting(String);

#[frost_bundle(prefix = "greeting")]
impl Greeting {
    fn greet(&self, name: String) -> String {
        format!("{}, {name}", self.0)
    }

    fn whoami(&self, #[frost(ctx)] ctx: &mut NativeCtx<'_>) -> String {
        format!("{} from {}", self.0, ctx.name())
    }

    fn through_context(#[frost(ctx)] ctx: &mut BundleCtx<'_, Self>) -> String {
        format!("{} through {}", ctx.state().0, ctx.native().name())
    }

    fn stateless(text: String) -> String {
        text.to_uppercase()
    }
}

#[test]
fn a_function_takes_the_state_that_never_changes_each_way() {
    let greeting = Greeting::frost_functions(Bundle::new(Greeting(String::from("Hello"))));
    let greeting = greeting.build();
    let run = |source| run("g", greeting.clone(), source);
    assert_eq!(run("g.greet('Ada')"), Value::from("Hello, Ada"));
    assert_eq!(run("g.whoami()"), Value::from("Hello from greeting.whoami"));
    assert_eq!(
        run("g.through_context()"),
        Value::from("Hello through greeting.through_context")
    );
    assert_eq!(run("g.stateless('hi')"), Value::from("HI"));
}

/// Each way a function of a bundle over state that changes takes it.
struct Tally(Vec<String>);

#[frost_mutable_bundle(prefix = "tally")]
impl Tally {
    fn count(&self) -> usize {
        self.0.len()
    }

    fn record(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> usize {
        self.0.push(ctx.name().to_string());
        self.0.len()
    }

    fn record_each(
        #[frost(ctx)] ctx: &mut MutableBundleCtx<'_, Self>,
        callback: Callback,
    ) -> Result<Value, FrostError> {
        let produced = ctx.native().invoke(&callback.0, [])?;
        ctx.lock()?.0.push(produced.to_frost_string());
        Ok(produced)
    }

    fn stateless(n: i64) -> i64 {
        n * 2
    }
}

fn tally() -> Value {
    Tally::frost_functions(MutableBundle::new(Tally(Vec::new()))).build()
}

#[test]
fn a_function_takes_the_state_that_changes_each_way() {
    // One permutation, since the state is shared across runs.
    let optimization = optimization_permutations()
        .next()
        .expect("there is always one permutation");
    let source = r"
        t.record()
        t.record_each(fn -> 'made')
        [t.count(), t.stateless(21)]
    ";
    let result = Script::new(source)
        .capture("t", tally())
        .run_under(optimization);
    assert_eq!(result, Value::array([2, 42]));
}

#[test]
fn a_function_through_its_context_may_reenter_the_bundle() {
    let optimization = optimization_permutations()
        .next()
        .expect("there is always one permutation");
    // The callback records while `record_each` runs, which it may, as it does
    // not hold the state meanwhile.
    let source = r"
        t.record_each(fn -> t.record())
        t.count()
    ";
    let result = Script::new(source)
        .capture("t", tally())
        .run_under(optimization);
    assert_eq!(result, Value::Int(2));
}

// ============================================================
// Keys and names
// ============================================================

#[test]
fn each_function_is_keyed_by_its_name_and_named_with_the_prefix() {
    let greeting = Greeting::frost_functions(Bundle::new(Greeting(String::new()))).build();
    assert_eq!(
        keys(&greeting),
        ["greet", "stateless", "through_context", "whoami"]
    );
    assert_eq!(
        raises("g", greeting, "g.greet(1)"),
        "Function greeting.greet requires String as argument 1 (name), got Int"
    );
}

struct Raw;

#[frost_bundle(prefix = "raw")]
impl Raw {
    fn r#type(&self) -> &'static str {
        "typed"
    }
}

#[test]
fn a_raw_function_name_is_keyed_without_its_prefix() {
    let raw = Raw::frost_functions(Bundle::new(Raw)).build();
    assert_eq!(keys(&raw), ["type"]);
    assert_eq!(run("r", raw, "r.type()"), Value::from("typed"));
}

struct Unnamed;

#[frost_bundle(prefix = "unnamed")]
impl Unnamed {
    fn repeat(&self, #[frost(unnamed)] text: String, times: usize) -> String {
        text.repeat(times)
    }
}

#[test]
fn an_unnamed_parameter_is_reported_by_position_alone() {
    let unnamed = Unnamed::frost_functions(Bundle::new(Unnamed)).build();
    assert_eq!(
        raises("u", unnamed.clone(), "u.repeat(1, 2)"),
        "Function unnamed.repeat requires String as argument 1, got Int"
    );
    assert_eq!(
        raises("u", unnamed, "u.repeat('a', 'b')"),
        "Function unnamed.repeat requires Int as argument 2 (times), got String"
    );
}

// ============================================================
// Groups
// ============================================================

struct Light(bool);

#[frost_mutable_bundle(prefix = "light")]
impl Light {
    fn is_on(&self) -> bool {
        self.0
    }
}

#[frost_mutable_bundle(prefix = "light", group = switch_functions)]
impl Light {
    fn switch(&mut self) -> bool {
        self.0 = !self.0;
        self.0
    }
}

fn light(switchable: bool) -> Value {
    let mut light = Light::frost_functions(MutableBundle::new(Light(false)));
    if switchable {
        light = Light::switch_functions(light);
    }
    light.build()
}

#[test]
fn groups_combine_into_a_bundle_with_optional_parts() {
    assert_eq!(keys(&light(false)), ["is_on"]);
    assert_eq!(keys(&light(true)), ["is_on", "switch"]);
    let optimization = optimization_permutations()
        .next()
        .expect("there is always one permutation");
    let result = Script::new("[l.switch(), l.is_on()]")
        .capture("l", light(true))
        .run_under(optimization);
    assert_eq!(result, Value::array([true, true]));
}

struct Configured;

#[frost_bundle(prefix = "configured")]
impl Configured {
    // Were its addition not configured out with it, the adder would call a
    // function that does not exist, and this file would not compile.
    #[cfg(any())]
    fn absent(&self) {}

    // Always configured in, as a test.
    #[cfg(test)]
    fn present(&self) -> i64 {
        1
    }
}

// A block configured out after the attribute: were its adder not configured
// out with it, the adder would call functions that do not exist.
#[frost_bundle(prefix = "configured", group = absent_functions)]
#[cfg(any())]
impl Configured {
    fn gone(&self) {}
}

// And before it, which rustc applies first, removing the attribute too.
#[cfg(any())]
#[frost_bundle(prefix = "configured", group = also_absent_functions)]
impl Configured {
    fn also_gone(&self) {}
}

// A block configured in after the attribute: always, as a test.
#[frost_bundle(prefix = "configured", group = extra_functions)]
#[cfg(test)]
impl Configured {
    fn extra(&self) -> i64 {
        2
    }
}

#[test]
fn each_function_is_added_exactly_when_it_is_configured() {
    let configured = Configured::frost_functions(Bundle::new(Configured)).build();
    assert_eq!(keys(&configured), ["present"]);
}

#[test]
fn each_group_exists_exactly_when_its_block_is_configured() {
    let configured = Configured::frost_functions(Bundle::new(Configured));
    let configured = Configured::extra_functions(configured).build();
    assert_eq!(keys(&configured), ["extra", "present"]);
}

// ============================================================
// Sharing and poisoning
// ============================================================

#[test]
fn a_shared_state_is_the_host_s_own() {
    let shared = Arc::new(StateLock::new(Counter(41)));
    let counter = Counter::frost_functions(MutableBundle::from_shared(Arc::clone(&shared))).build();
    let optimization = optimization_permutations()
        .next()
        .expect("there is always one permutation");
    let result = Script::new("c.add(1)")
        .capture("c", counter)
        .run_under(optimization);
    assert_eq!(result, Value::Int(42));
    assert_eq!(shared.lock().expect("not poisoned").0, 42);
}

struct Fragile(i64);

#[frost_mutable_bundle(prefix = "fragile")]
impl Fragile {
    fn get(&self) -> i64 {
        self.0
    }

    fn break_it(&mut self) {
        self.0 = -1;
        panic!("a bug while holding the state");
    }
}

#[test]
fn a_panic_in_a_function_holding_the_state_poisons_it() {
    let shared = Arc::new(StateLock::new(Fragile(5)));
    let fragile = Fragile::frost_functions(MutableBundle::from_shared(Arc::clone(&shared))).build();
    let optimization = optimization_permutations()
        .next()
        .expect("there is always one permutation");
    let script = Script::new("f.break_it()").capture("f", fragile.clone());
    let outcome = catch_unwind(AssertUnwindSafe(|| script.run_under(optimization)));
    assert!(outcome.is_err(), "break_it should panic");
    assert_eq!(
        raises("f", fragile, "f.get()"),
        "Function fragile.get cannot run: a panic poisoned its state"
    );
    assert_eq!(shared.lock().err(), Some(PoisonedState));
}
