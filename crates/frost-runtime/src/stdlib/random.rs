//! `std.random`: pseudo-random engines, seeded or not, and the draws they make.
//!
//! An engine is a Map of draw functions sharing one generator, which each draw
//! advances. Generation is `fastrand`'s. An engine is seeded either from an Int,
//! reproducibly within a Frost version, or from the operating system's randomness.
//! Not for cryptography.

use std::hash::{BuildHasher, RandomState};
use std::sync::{Arc, Mutex};

use crate::{FrostError, FrostFloat, FrostType, Param, Params, StdlibModule, Value};

/// The configuration of [`random`].
#[derive(Clone, Copy, Debug, Default)]
pub struct RandomConfig {
    /// The seed of the module's default engine, `random.rng`.
    /// With `None`, it is seeded from the operating system's randomness,
    /// and so differs every time the module is built.
    ///
    /// With `Some(n)`, `random.rng` draws exactly as a script's `random.seed(n)`
    /// engine would, which makes scripts drawing from it reproducible, such as
    /// for tests or replays.
    pub rng_seed: Option<i64>,
}

/// The `std.random` module: pseudo-random engines, which draw Ints, Floats, and
/// Bools, choose and sample elements, and shuffle Arrays. `random.rng` is a
/// default engine, seeded as `config` says; `random.seed(n)` makes another.
///
/// The engine `random.rng` is built with the module, so every script importing
/// one instance of the module draws from it in turn. A seeded engine's draws are
/// reproducible within a Frost version, not across versions. Not for cryptography.
pub fn random(config: RandomConfig) -> StdlibModule {
    let rng_seed = config.rng_seed.map_or_else(os_seed, seed_bits);
    StdlibModule::new(
        "random",
        Value::map([("rng", engine(rng_seed)), ("seed", seed())]),
    )
}

/// A seed from the operating system's randomness: std documents that a new
/// `RandomState` starts from random keys, which then hash to a random value.
fn os_seed() -> u64 {
    RandomState::new().hash_one(())
}

/// A script's Int seed, as the generator takes it: the same 64 bits.
fn seed_bits(seed: i64) -> u64 {
    u64::from_ne_bytes(seed.to_ne_bytes())
}

fn seed() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::INT).named("seed")]);
    Value::checked_native("random.seed", PARAMS, |_, args| {
        let seed = args[0].as_int().expect("type-checked as an Int");
        Ok(engine(seed_bits(seed)))
    })
}

// --- Engines ---

/// A generator shared by the draw functions of one engine.
type Shared = Arc<Mutex<fastrand::Rng>>;

/// An engine: the draw functions over one generator seeded with `seed`.
fn engine(seed: u64) -> Value {
    let rng: Shared = Arc::new(Mutex::new(fastrand::Rng::with_seed(seed)));
    Value::map([
        ("int", int(&rng)),
        ("float", float(&rng)),
        ("bool", boolean(&rng)),
        ("choice", choice(&rng)),
        ("sample", sample(&rng)),
        ("shuffle", shuffle(&rng)),
    ])
}

/// Runs `draw` with the engine's generator.
fn draw<T>(rng: &Shared, draw: impl FnOnce(&mut fastrand::Rng) -> T) -> T {
    // A draw cannot panic midway, so the generator is never left poisoned.
    draw(&mut rng.lock().expect("a draw never panics"))
}

/// The error for bounds whose low end, `low`, is above the high end, `high`.
fn bounds_error(function: &str, low: &Value, high: &Value) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires argument 1 (low) to be at most argument 2 (high), \
         got {} and {}",
        low.to_frost_string(),
        high.to_frost_string()
    ))
}

/// A type-checked Numeric argument, as a float.
fn float_arg(arg: &Value) -> f64 {
    match arg {
        Value::Int(i) => *i as f64,
        Value::Float(f) => f.get(),
        other => unreachable!("type-checked as Numeric, got {}", other.type_name()),
    }
}

fn int(rng: &Shared) -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::INT).named("low"),
        Param::of(FrostType::INT).named("high"),
    ]);
    let rng = Arc::clone(rng);
    Value::checked_native("rng.int", PARAMS, move |_, args| {
        let [low, high] = [&args[0], &args[1]].map(|arg| arg.as_int().expect("an Int"));
        if low > high {
            return Err(bounds_error("rng.int", &args[0], &args[1]));
        }
        Ok(Value::Int(draw(&rng, |rng| rng.i64(low..=high))))
    })
}

fn float(rng: &Shared) -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::NUMERIC).named("low"),
        Param::of(FrostType::NUMERIC).named("high"),
    ]);
    let rng = Arc::clone(rng);
    Value::checked_native("rng.float", PARAMS, move |_, args| {
        let [low, high] = [&args[0], &args[1]].map(float_arg);
        if low > high {
            return Err(bounds_error("rng.float", &args[0], &args[1]));
        }
        let t = draw(&rng, fastrand::Rng::f64_inclusive);
        // Weighting both ends, rather than `low + (high - low) * t`, cannot
        // overflow, and gives exactly `low` and `high` at the ends.
        let drawn = (1.0 - t) * low + t * high;
        Ok(Value::Float(
            FrostFloat::new(drawn).expect("a point between two finite floats is finite"),
        ))
    })
}

fn boolean(rng: &Shared) -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::NUMERIC)
        .named("probability")
        .optional()]);
    let rng = Arc::clone(rng);
    Value::checked_native("rng.bool", PARAMS, move |_, args| {
        let Some(probability) = args.first() else {
            return Ok(Value::Bool(draw(&rng, fastrand::Rng::bool)));
        };
        let p = float_arg(probability);
        if !(0.0..=1.0).contains(&p) {
            return Err(FrostError::from_string(format!(
                "Function rng.bool requires argument 1 (probability) to be from 0 to 1, got {}",
                probability.to_frost_string()
            )));
        }
        // A draw from [0, 1) is below 1 always and below 0 never.
        Ok(Value::Bool(draw(&rng, fastrand::Rng::f64) < p))
    })
}

fn choice(rng: &Shared) -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::ARRAY)]);
    let rng = Arc::clone(rng);
    Value::checked_native("rng.choice", PARAMS, move |_, args| {
        let array = args[0].as_array().expect("type-checked as an Array");
        if array.is_empty() {
            return Err(FrostError::from_static(
                "Function rng.choice requires a non-empty Array",
            ));
        }
        let index = draw(&rng, |rng| rng.usize(..array.len()));
        Ok(array[index].clone())
    })
}

fn sample(rng: &Shared) -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::ARRAY),
        Param::of(FrostType::INT).named("n"),
    ]);
    let rng = Arc::clone(rng);
    Value::checked_native("rng.sample", PARAMS, move |_, args| {
        let array = args[0].as_array().expect("type-checked as an Array");
        let n = args[1].as_int().expect("type-checked as an Int");
        let count = usize::try_from(n).ok().filter(|&n| n <= array.len());
        let Some(count) = count else {
            return Err(FrostError::from_string(format!(
                "Function rng.sample requires argument 2 (n) to be from 0 to the Array's \
                 length, {}, got {n}",
                array.len()
            )));
        };
        let sampled = draw(&rng, |rng| {
            rng.choose_multiple(array.iter().cloned(), count)
        });
        Ok(sampled.into())
    })
}

fn shuffle(rng: &Shared) -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::ARRAY)]);
    let rng = Arc::clone(rng);
    Value::checked_native("rng.shuffle", PARAMS, move |_, args| {
        let Value::Array(array) = args[0].take() else {
            unreachable!("type-checked as an Array")
        };
        let mut elements = array.into_vec();
        draw(&rng, |rng| rng.shuffle(&mut elements));
        Ok(elements.into())
    })
}
