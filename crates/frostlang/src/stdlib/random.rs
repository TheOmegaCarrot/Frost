//! `std.random`: pseudo-random engines, seeded or not, and the draws they make.
//!
//! An engine is a Map of draw functions sharing one generator, which each draw
//! advances. Generation is `fastrand`'s. An engine is seeded either from an Int,
//! reproducibly within a Frost version, or from the operating system's randomness.
//! Not for cryptography.

use std::hash::{BuildHasher, RandomState};

use crate::native::{Args, FrostArg, MutableBundle, MutableBundleCtx, Optional};
use crate::{
    FrostArray, FrostError, FrostFloat, FrostResult, FrostType, Param, Params, StdlibModule, Value,
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

/// The context of an engine's draw functions, over its generator.
type Ctx<'a> = MutableBundleCtx<'a, fastrand::Rng>;

/// An engine: the draw functions over one generator seeded with `seed`.
fn engine(seed: u64) -> Value {
    const BOUNDS: Params = Params::new(&[
        Param::of(FrostType::NUMERIC).named("low"),
        Param::of(FrostType::NUMERIC).named("high"),
    ]);
    const INT_BOUNDS: Params = Params::new(&[
        <i64 as FrostArg>::PARAM.named("low"),
        <i64 as FrostArg>::PARAM.named("high"),
    ]);
    const PROBABILITY: Params = Params::new(&[Param::of(FrostType::NUMERIC)
        .named("probability")
        .optional()]);
    const ARRAY: Params = Params::new(&[<FrostArray as FrostArg>::PARAM]);
    const ARRAY_AND_COUNT: Params = Params::new(&[
        <FrostArray as FrostArg>::PARAM,
        <i64 as FrostArg>::PARAM.named("n"),
    ]);
    MutableBundle::new(fastrand::Rng::with_seed(seed))
        .function("rng.int", INT_BOUNDS, int)
        .function("rng.float", BOUNDS, float)
        .function("rng.bool", PROBABILITY, boolean)
        .function("rng.choice", ARRAY, choice)
        .function("rng.sample", ARRAY_AND_COUNT, sample)
        .function("rng.shuffle", ARRAY, shuffle)
        .build()
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

fn int(ctx: &mut Ctx<'_>, mut args: Args<'_>) -> FrostResult {
    let low: i64 = args.take()?;
    let high: i64 = args.take()?;
    if low > high {
        return Err(bounds_error("rng.int", &low.into(), &high.into()));
    }
    Ok(Value::Int(ctx.lock()?.i64(low..=high)))
}

fn float(ctx: &mut Ctx<'_>, mut args: Args<'_>) -> FrostResult {
    let low_arg: Value = args.take()?;
    let high_arg: Value = args.take()?;
    let [low, high] = [&low_arg, &high_arg].map(float_arg);
    if low > high {
        return Err(bounds_error("rng.float", &low_arg, &high_arg));
    }
    if low == high {
        return Ok(Value::Float(
            FrostFloat::new(low).expect("a Numeric argument is finite"),
        ));
    }
    let t = ctx.lock()?.f64_inclusive();
    // Weighting both ends, rather than `low + (high - low) * t`, cannot
    // overflow, and gives exactly `low` and `high` at the ends. Rounding can
    // still step just past an end, so the result is clamped.
    let drawn = ((1.0 - t) * low + t * high).clamp(low, high);
    Ok(Value::Float(
        FrostFloat::new(drawn).expect("a point between two finite floats is finite"),
    ))
}

fn boolean(ctx: &mut Ctx<'_>, mut args: Args<'_>) -> FrostResult {
    let Optional(probability) = args.take::<Optional<Value>>()?;
    let Some(probability) = probability else {
        return Ok(Value::Bool(ctx.lock()?.bool()));
    };
    let p = float_arg(&probability);
    if !(0.0..=1.0).contains(&p) {
        return Err(FrostError::from_string(format!(
            "Function rng.bool requires argument 1 (probability) to be from 0 to 1, got {}",
            probability.to_frost_string()
        )));
    }
    // A draw from [0, 1) is below 1 always and below 0 never.
    Ok(Value::Bool(ctx.lock()?.f64() < p))
}

fn choice(ctx: &mut Ctx<'_>, mut args: Args<'_>) -> FrostResult {
    let array: FrostArray = args.take()?;
    if array.is_empty() {
        return Err(FrostError::from_static(
            "Function rng.choice requires a non-empty Array",
        ));
    }
    let index = ctx.lock()?.usize(..array.len());
    Ok(array[index].clone())
}

fn sample(ctx: &mut Ctx<'_>, mut args: Args<'_>) -> FrostResult {
    let array: FrostArray = args.take()?;
    let n: i64 = args.take()?;
    let count = usize::try_from(n).ok().filter(|&n| n <= array.len());
    let Some(count) = count else {
        return Err(FrostError::from_string(format!(
            "Function rng.sample requires argument 2 (n) to be from 0 to the Array's \
             length, {}, got {n}",
            array.len()
        )));
    };
    let sampled = ctx.lock()?.choose_multiple(array.iter().cloned(), count);
    Ok(sampled.into())
}

fn shuffle(ctx: &mut Ctx<'_>, mut args: Args<'_>) -> FrostResult {
    let array: FrostArray = args.take()?;
    let mut elements = array.into_vec();
    ctx.lock()?.shuffle(&mut elements);
    Ok(elements.into())
}
