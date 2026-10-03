//! `std.random`, from Frost source.
//!
//! Most cases draw from engines a script seeds itself (`random.seed(n)`), which
//! every optimization permutation builds and draws from identically. The default
//! engine, `random.rng`, belongs to the module instance, so its draws advance
//! from one run to the next: cases about it build their own module and run once.

mod source;

use std::sync::Arc;

use frost_runtime::stdlib::RandomConfig;
use frost_runtime::{Importer, ImporterBuilder, Stdlib, Value, stdlib};
use source::{Script, UNOPTIMIZED};

/// An importer providing only `std.random`, configured by `config`.
fn importer(config: RandomConfig) -> Arc<Importer> {
    let stdlib = Stdlib::new()
        .with_module(stdlib::random(config))
        .expect("a lone module is accepted");
    ImporterBuilder::new().with_stdlib(stdlib).build()
}

/// `expression`, with `std.random` bound as `random`.
fn source(expression: &str) -> String {
    format!(
        r"
        def random = import('std.random')
        {expression}
        "
    )
}

/// `expression`, run with `std.random` bound as `random`.
fn script(expression: &str) -> Script {
    Script::new(&source(expression)).importer(importer(RandomConfig::default()))
}

/// `expression`, run once with `importer`'s `std.random` bound as `random`.
fn run_once(expression: &str, importer: &Arc<Importer>) -> Value {
    Script::new(&source(expression))
        .importer(Arc::clone(importer))
        .run_under(UNOPTIMIZED)
}

/// Assert each `expression` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (expression, expected) in cases {
        assert_eq!(
            script(expression).run(),
            script(expected).run(),
            "{expression:?} is {expected}"
        );
    }
}

/// Assert each `expression` raises exactly `message`.
fn assert_raises(cases: &[(&str, &str)]) {
    for (expression, message) in cases {
        assert_eq!(script(expression).raises(), *message, "{expression:?}");
    }
}

/// Assert `call` (such as `random.seed(1).int`) raises the arity error of
/// `function` for each count in `counts`, where `expects` is how the error
/// states its arity.
fn assert_arity(call: &str, function: &str, expects: &str, counts: &[usize]) {
    for &argc in counts {
        let expression = format!("{call}({})", vec!["null"; argc].join(", "));
        assert_raises(&[(
            &expression,
            &format!("Function {function} expects {expects} arguments, but was called with {argc}"),
        )]);
    }
}

/// A Frost expression for `count` draws from a fresh engine seeded with 1, each
/// made by `draw`, a Frost expression over the engine `r`.
fn draws(draw: &str, count: usize) -> String {
    format!(
        "do {{
            def r = random.seed(1)
            map range({count}) with fn _ -> {draw}
        }}"
    )
}

// --- The module ---

#[test]
fn the_module_holds_a_default_engine_and_seed() {
    let methods = "['bool', 'choice', 'float', 'int', 'sample', 'shuffle']";
    assert_values(&[
        ("sorted(keys(random))", "['rng', 'seed']"),
        ("sorted(keys(random.rng))", methods),
        ("sorted(keys(random.seed(1)))", methods),
    ]);
}

#[test]
fn the_module_is_contained() {
    let stdlib = Stdlib::contained(RandomConfig { rng_seed: Some(9) });
    let contained = ImporterBuilder::new().with_stdlib(stdlib).build();
    let result = Script::new("import('std.random').rng.int(4, 4)")
        .importer(contained)
        .run();
    assert_eq!(result, Value::Int(4));
}

// --- Seeded engines ---

#[test]
fn engines_of_one_seed_draw_alike() {
    let source = r"
        def a = random.seed(2024)
        def b = random.seed(2024)
        defn draws(r) -> [
            r.int(1, 100),
            r.float(0, 1),
            r.bool(),
            r.bool(0.3),
            r.choice(range(10)),
            r.sample(range(10), 3),
            r.shuffle(range(10)),
            r.int(-5, 5),
        ]
        draws(a) == draws(b)
    ";
    assert_values(&[(source, "true")]);
}

#[test]
fn any_int_is_a_seed() {
    for seed in ["0", "-1", "math_maxint", "-9223372036854775807 - 1"] {
        let seed = seed.replace("math_maxint", "9223372036854775807");
        assert_values(&[(
            &format!("random.seed({seed}).int(1, 6) == random.seed({seed}).int(1, 6)"),
            "true",
        )]);
    }
}

#[test]
fn engines_of_different_seeds_draw_differently() {
    assert_values(&[(
        "random.seed(1).int(0, 1000000000000) == random.seed(2).int(0, 1000000000000)",
        "false",
    )]);
}

#[test]
fn engines_do_not_share_state() {
    let source = r"
        def a = random.seed(5)
        def b = random.seed(5)
        a.int(0, 1000000)
        a.int(0, 1000000)
        b.int(0, 1000000) == random.seed(5).int(0, 1000000)
    ";
    assert_values(&[(source, "true")]);
}

#[test]
fn every_binding_of_an_engine_shares_its_state() {
    let source = r"
        def a = random.seed(5)
        def b = a
        def first = a.int(0, 1000000)
        def second = b.int(0, 1000000)
        def fresh = random.seed(5)
        [first == fresh.int(0, 1000000), second == fresh.int(0, 1000000)]
    ";
    assert_values(&[(source, "[true, true]")]);
}

// --- int ---

#[test]
fn int_draws_within_inclusive_bounds() {
    let in_bounds = draws("r.int(-3, 3)", 1000);
    assert_values(&[
        (
            &format!("all({in_bounds}, fn x -> is_int(x) and x >= -3 and x <= 3)"),
            "true",
        ),
        // Both ends, and everything between, come up.
        (
            &format!("sorted(keys(count_by({in_bounds}, id)))"),
            "range(-3, 4)",
        ),
        (
            &format!("all({}, fn x -> x == 5)", draws("r.int(5, 5)", 50)),
            "true",
        ),
        (
            "is_int(random.seed(1).int(-9223372036854775807 - 1, 9223372036854775807))",
            "true",
        ),
    ]);
    assert_raises(&[(
        "random.seed(1).int(5, 1)",
        "Function rng.int requires argument 1 (low) to be at most argument 2 (high), got 5 and 1",
    )]);
}

// --- float ---

#[test]
fn float_draws_within_inclusive_bounds() {
    assert_values(&[
        (
            &format!(
                "all({}, fn x -> is_float(x) and x >= -1.5 and x <= 2.5)",
                draws("r.float(-1.5, 2.5)", 1000)
            ),
            "true",
        ),
        // Int bounds are taken as Floats.
        (
            &format!("all({}, is_float)", draws("r.float(0, 1)", 50)),
            "true",
        ),
        (
            &format!("all({}, fn x -> x == 2.0)", draws("r.float(2, 2)", 50)),
            "true",
        ),
        // The widest bounds do not overflow.
        (
            "is_float(random.seed(1).float(-1.7976931348623157e308, 1.7976931348623157e308))",
            "true",
        ),
    ]);
    assert_raises(&[(
        "random.seed(1).float(1, 0.5)",
        "Function rng.float requires argument 1 (low) to be at most argument 2 (high), \
         got 1 and 0.5",
    )]);
}

// --- bool ---

#[test]
fn bool_draws_with_a_probability() {
    assert_values(&[
        (&format!("all({}, is_bool)", draws("r.bool()", 50)), "true"),
        (&format!("none({}, id)", draws("r.bool(0)", 200)), "true"),
        (&format!("all({}, id)", draws("r.bool(1)", 200)), "true"),
        (&format!("all({}, id)", draws("r.bool(1.0)", 200)), "true"),
        // A fair coin and a biased one both come up both ways.
        (
            &format!("len(count_by({}, id))", draws("r.bool()", 200)),
            "2",
        ),
        (
            &format!("len(count_by({}, id))", draws("r.bool(0.2)", 200)),
            "2",
        ),
    ]);
    for probability in ["-0.1", "1.5", "2"] {
        assert_raises(&[(
            &format!("random.seed(1).bool({probability})"),
            &format!(
                "Function rng.bool requires argument 1 (probability) to be from 0 to 1, got {probability}"
            ),
        )]);
    }
}

// --- choice, sample, shuffle ---

#[test]
fn choice_draws_an_element() {
    assert_values(&[
        (
            &format!(
                "all({}, fn x -> includes(['a', 'b', 'c'], x))",
                draws("r.choice(['a', 'b', 'c'])", 200)
            ),
            "true",
        ),
        (
            &format!(
                "sorted(keys(count_by({}, id)))",
                draws("r.choice(['a', 'b', 'c'])", 200)
            ),
            "['a', 'b', 'c']",
        ),
        ("random.seed(1).choice([[1]])", "[1]"),
    ]);
    assert_raises(&[(
        "random.seed(1).choice([])",
        "Function rng.choice requires a non-empty Array",
    )]);
}

#[test]
fn sample_draws_distinct_elements() {
    let source = r"
        def s = random.seed(1).sample(range(10), 4)
        [len(s), all(s, fn x -> includes(range(10), x)), len(count_by(s, id))]
    ";
    assert_values(&[
        (source, "[4, true, 4]"),
        ("random.seed(1).sample(range(10), 0)", "[]"),
        ("sorted(random.seed(1).sample(range(10), 10))", "range(10)"),
        // Elements are drawn by position: equal elements are each drawable.
        ("random.seed(1).sample([7, 7, 7], 2)", "[7, 7]"),
    ]);
    for n in ["-1", "11"] {
        assert_raises(&[(
            &format!("random.seed(1).sample(range(10), {n})"),
            &format!(
                "Function rng.sample requires argument 2 (n) to be from 0 to the Array's length, \
                 10, got {n}"
            ),
        )]);
    }
}

#[test]
fn shuffle_reorders_without_changing_its_input() {
    let source = r"
        def a = range(20)
        def s = random.seed(1).shuffle(a)
        [sorted(s) == a, s == a, a == range(20)]
    ";
    assert_values(&[
        (source, "[true, false, true]"),
        ("random.seed(1).shuffle([])", "[]"),
        ("random.seed(1).shuffle(['x'])", "['x']"),
    ]);
}

// --- The default engine, `random.rng` ---

#[test]
fn a_configured_seed_draws_as_random_seed_does() {
    let importer = importer(RandomConfig { rng_seed: Some(42) });
    let source = r"
        def s = random.seed(42)
        [
            random.rng.int(0, 1000000) == s.int(0, 1000000),
            random.rng.shuffle(range(10)) == s.shuffle(range(10)),
        ]
    ";
    assert_eq!(
        run_once(source, &importer),
        Value::array([Value::Bool(true), Value::Bool(true)])
    );
}

#[test]
fn a_configured_seed_makes_the_default_engine_reproducible() {
    let draw = "random.rng.int(0, 1000000000000)";
    let first = importer(RandomConfig { rng_seed: Some(7) });
    let second = importer(RandomConfig { rng_seed: Some(7) });
    assert_eq!(run_once(draw, &first), run_once(draw, &second));
}

#[test]
fn the_default_engine_draws_on_from_one_script_to_the_next() {
    // Two scripts sharing one module instance share its default engine.
    let importer = importer(RandomConfig { rng_seed: Some(3) });
    let first = run_once("random.rng.int(0, 1000000)", &importer);
    let second = run_once("random.rng.int(0, 1000000)", &importer);
    let expected = run_once(
        r"
        def s = random.seed(3)
        [s.int(0, 1000000), s.int(0, 1000000)]
        ",
        &importer,
    );
    assert_eq!(Value::array([first, second]), expected);
}

#[test]
fn an_unconfigured_default_engine_is_seeded_differently_each_time() {
    let draw = "random.rng.int(0, 9223372036854775807)";
    let first = run_once(draw, &importer(RandomConfig::default()));
    let second = run_once(draw, &importer(RandomConfig::default()));
    assert_ne!(first, second, "two OS-seeded engines drew alike");
}

#[test]
fn the_default_engine_draws_within_bounds() {
    // Whatever its seed, every run agrees on these.
    let source = r"
        def x = random.rng.int(1, 6)
        x >= 1 and x <= 6
    ";
    assert_values(&[(source, "true")]);
}

// --- Arguments ---

#[test]
fn every_function_checks_its_arguments() {
    let engine = "random.seed(1)";
    assert_raises(&[
        (
            &format!("{engine}.int('1', 2)"),
            "Function rng.int requires Int as argument 1 (low), got String",
        ),
        (
            &format!("{engine}.int(1, 2.0)"),
            "Function rng.int requires Int as argument 2 (high), got Float",
        ),
        (
            &format!("{engine}.float('a', 1)"),
            "Function rng.float requires Numeric as argument 1 (low), got String",
        ),
        (
            &format!("{engine}.float(0, null)"),
            "Function rng.float requires Numeric as argument 2 (high), got Null",
        ),
        (
            &format!("{engine}.bool('x')"),
            "Function rng.bool requires Numeric as argument 1 (probability), got String",
        ),
        (
            &format!("{engine}.choice('abc')"),
            "Function rng.choice requires Array as argument 1, got String",
        ),
        (
            &format!("{engine}.sample({{}}, 1)"),
            "Function rng.sample requires Array as argument 1, got Map",
        ),
        (
            &format!("{engine}.sample([1], '1')"),
            "Function rng.sample requires Int as argument 2 (n), got String",
        ),
        (
            &format!("{engine}.shuffle('abc')"),
            "Function rng.shuffle requires Array as argument 1, got String",
        ),
        (
            "random.seed(1.5)",
            "Function random.seed requires Int as argument 1 (seed), got Float",
        ),
    ]);
    assert_arity(&format!("{engine}.int"), "rng.int", "2", &[0, 1, 3]);
    assert_arity(&format!("{engine}.float"), "rng.float", "2", &[0, 1, 3]);
    assert_arity(
        &format!("{engine}.bool"),
        "rng.bool",
        "between 0 and 1",
        &[2],
    );
    assert_arity(&format!("{engine}.choice"), "rng.choice", "1", &[0, 2]);
    assert_arity(&format!("{engine}.sample"), "rng.sample", "2", &[0, 1, 3]);
    assert_arity(&format!("{engine}.shuffle"), "rng.shuffle", "1", &[0, 2]);
    assert_arity("random.seed", "random.seed", "1", &[0, 2]);
}
