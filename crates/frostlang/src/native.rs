//! Glue for writing native functions with Rust types: taking each argument as
//! a Rust type, and returning a Rust type as a value.
//!
//! An argument type describes its own parameter through [`FrostArg`], so a
//! native's [`Params`](crate::Params) can be built from its argument types.
//! The natives themselves are built as usual, with
//! [`Value::checked_native`](crate::Value::checked_native), which checks each
//! argument's type. An [`Args`] then takes each argument as its Rust type,
//! and [`IntoNativeResult`] converts the result:
//!
//! ```
//! use frostlang::native::{Args, FrostArg, IntoNativeResult, Optional};
//! use frostlang::{Params, Value};
//!
//! const PARAMS: Params = Params::new(&[
//!     <String as FrostArg>::PARAM.named("text"),
//!     <Optional<usize> as FrostArg>::PARAM.named("times"),
//! ]);
//!
//! let repeat = Value::checked_native("repeat", PARAMS, |ctx, args| {
//!     let mut args = Args::new(ctx.name(), PARAMS, args);
//!     let text: String = args.take()?;
//!     let Optional(times) = args.take()?;
//!     text.repeat(times.unwrap_or(1)).into_result(ctx.name())
//! });
//! ```
//!
//! Errors are worded alike for every native: a wrong type by
//! [`check_args`](crate::NativeFunction::check_args), and an argument of the
//! right type but unacceptable content by [`ArgSite::requires`], as when
//! `-1` is given for a `u64`.

mod arg;
mod bundle;
mod category;
mod caught;
mod def;
// White-box tests for `#[frost_fn]` used within this crate.
#[cfg(test)]
mod frost_fn_tests;
mod mutable_bundle;
mod opaque;
mod result;

// Defined in the core, as `from_value` and `to_value` use it too; re-exported
// here beside the other argument types.
pub use crate::Nullable;
pub use arg::{ArgSite, Args, De, FromArg, FrostArg, Optional, Rest};
pub use bundle::{Bundle, BundleCtx};
pub use category::{Flat, Function, Nonnull, Numeric, Primitive, Structured};
pub use caught::caught;
pub use def::NativeDef;
pub use mutable_bundle::{MutableBundle, MutableBundleCtx, PoisonedState, StateGuard, StateLock};
pub use opaque::Opaque;
pub use result::{IntoNativeResult, Ser};
