//! `std.random`: pseudo-random engines, seeded or not, and the draws they make.
//!
//! An engine is a Map of draw functions sharing one generator, which each draw
//! advances. Generation is `fastrand`'s. An engine is seeded either from an Int,
//! reproducibly within a Frost version, or from the operating system's randomness.
//! Not for cryptography.

use std::hash::{BuildHasher, RandomState};

use crate::native::{MutableBundle, Numeric, Optional};
use crate::{
    FrostArray, FrostError, FrostFloat, FrostResult, FrostType, Param, Params, StdlibModule, Value,
    frost_mutable_bundle,
};

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

/// An engine: the draw functions over one generator seeded with `seed`.
fn engine(seed: u64) -> Value {
    let engine = Engine(fastrand::Rng::with_seed(seed));
    Engine::frost_functions(MutableBundle::new(engine)).build()
}

/// An engine's generator.
struct Engine(fastrand::Rng);

/// The error for bounds whose low end, `low`, is above the high end, `high`.
fn bounds_error(function: &str, low: Value, high: Value) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires argument 1 (low) to be at most argument 2 (high), \
         got {} and {}",
        low.to_frost_string(),
        high.to_frost_string()
    ))
}

#[frost_mutable_bundle(prefix = "rng")]
impl Engine {
    fn int(&mut self, low: i64, high: i64) -> FrostResult {
        if low > high {
            return Err(bounds_error("rng.int", low.into(), high.into()));
        }
        Ok(Value::Int(self.0.i64(low..=high)))
    }

    fn float(&mut self, low: Numeric, high: Numeric) -> FrostResult {
        let (from, to) = (low.to_f64(), high.to_f64());
        if from > to {
            return Err(bounds_error("rng.float", low.into(), high.into()));
        }
        if from == to {
            return Ok(Value::Float(
                FrostFloat::new(from).expect("a Numeric argument is finite"),
            ));
        }
        let t = self.0.f64_inclusive();
        // Weighting both ends, rather than `from + (to - from) * t`, cannot
        // overflow, and gives exactly `from` and `to` at the ends. Rounding can
        // still step just past an end, so the result is clamped.
        let drawn = ((1.0 - t) * from + t * to).clamp(from, to);
        Ok(Value::Float(
            FrostFloat::new(drawn).expect("a point between two finite floats is finite"),
        ))
    }

    fn bool(&mut self, probability: Optional<Numeric>) -> FrostResult {
        let Optional(Some(probability)) = probability else {
            return Ok(Value::Bool(self.0.bool()));
        };
        let p = probability.to_f64();
        if !(0.0..=1.0).contains(&p) {
            return Err(FrostError::from_string(format!(
                "Function rng.bool requires argument 1 (probability) to be from 0 to 1, got {}",
                Value::from(probability).to_frost_string()
            )));
        }
        // A draw from [0, 1) is below 1 always and below 0 never.
        Ok(Value::Bool(self.0.f64() < p))
    }

    fn choice(&mut self, #[frost(unnamed)] array: FrostArray) -> FrostResult {
        if array.is_empty() {
            return Err(FrostError::from_static(
                "Function rng.choice requires a non-empty Array",
            ));
        }
        let index = self.0.usize(..array.len());
        Ok(array[index].clone())
    }

    fn sample(&mut self, #[frost(unnamed)] array: FrostArray, n: i64) -> FrostResult {
        let count = usize::try_from(n).ok().filter(|&n| n <= array.len());
        let Some(count) = count else {
            return Err(FrostError::from_string(format!(
                "Function rng.sample requires argument 2 (n) to be from 0 to the Array's \
                 length, {}, got {n}",
                array.len()
            )));
        };
        Ok(self.0.choose_multiple(array.iter().cloned(), count).into())
    }

    fn shuffle(&mut self, #[frost(unnamed)] array: FrostArray) -> FrostArray {
        let mut elements = array.into_vec();
        self.0.shuffle(&mut elements);
        elements.into()
    }
}
