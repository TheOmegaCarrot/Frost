//! Mutable bundles: Maps of natives over one shared state behind a lock.
//!
//! A mutable bundle is a [`Bundle`] over a [`StateLock`], whose functions
//! receive a context that only locks the state in ways that cannot deadlock.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::native::{Args, Bundle, IntoNativeResult};
use crate::{FrostError, NativeCtx, Params, Value};

/// A Map of natives over one state that changes, such as a stream with its
/// reads and writes, built one function at a time.
///
/// Each function receives a [`MutableBundleCtx`], through which it locks the
/// state or calls back into Frost, and its arguments as [`Args`]. The bundle
/// owns the state, or shares it with the host through a [`StateLock`]. For a
/// state that never changes, a [`Bundle`] needs no lock.
///
/// ```
/// use frostlang::native::{FrostArg, MutableBundle};
/// use frostlang::{Params, Value};
///
/// const NO_PARAMS: Params = Params::new(&[]);
/// const AMOUNT: Params = Params::new(&[<i64 as FrostArg>::PARAM.named("amount")]);
///
/// let counter = MutableBundle::new(0_i64)
///     .function("counter.get", NO_PARAMS, |ctx, _| Ok(*ctx.lock()?))
///     .function("counter.add", AMOUNT, |ctx, mut args| {
///         let amount: i64 = args.take()?;
///         let mut count = ctx.lock()?;
///         *count += amount;
///         Ok(*count)
///     })
///     .build();
/// ```
#[must_use = "a bundle does nothing until built"]
pub struct MutableBundle<S>(Bundle<StateLock<S>>);

impl<S: Send + 'static> MutableBundle<S> {
    /// A bundle owning `state`.
    pub fn new(state: S) -> Self {
        Self(Bundle::new(StateLock::new(state)))
    }

    /// A bundle sharing `state` with whoever else holds it, such as the host.
    pub fn from_shared(state: Arc<StateLock<S>>) -> Self {
        Self(Bundle::from_shared(state))
    }

    /// Adds the function `name`, taking arguments per `params`.
    ///
    /// Its key in the bundle is `name` after the last `.`, so `"reader.seek"`
    /// is keyed `seek`.
    ///
    /// # Panics
    ///
    /// If the bundle already has a function with that key.
    pub fn function<F, R>(self, name: &'static str, params: Params, function: F) -> Self
    where
        F: Fn(&mut MutableBundleCtx<'_, S>, Args<'_>) -> R + Send + Sync + 'static,
        R: IntoNativeResult,
    {
        Self(
            self.0
                .function_over_state(name, params, move |native, state, args| {
                    function(&mut MutableBundleCtx { native, state }, args)
                }),
        )
    }

    /// The bundle: a Map of its functions by key.
    pub fn build(self) -> Value {
        self.0.build()
    }
}

/// A mutable bundle's state behind its lock, for sharing it between the bundle
/// and the host. See [`MutableBundle::from_shared`].
///
/// A panic while the state is locked poisons it, since the panic may have left
/// it half changed: every later lock fails.
pub struct StateLock<S>(Mutex<S>);

impl<S> StateLock<S> {
    /// The lock over `state`.
    pub fn new(state: S) -> Self {
        Self(Mutex::new(state))
    }

    /// Locks the state, waiting while another holder has it.
    pub fn lock(&self) -> Result<MutexGuard<'_, S>, PoisonedState> {
        self.0.lock().map_err(|_| PoisonedState)
    }
}

impl<S: fmt::Debug> fmt::Debug for StateLock<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StateLock").field(&self.0).finish()
    }
}

/// The error locking a [`StateLock`] that a panic poisoned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoisonedState;

impl fmt::Display for PoisonedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a panic poisoned the state")
    }
}

impl std::error::Error for PoisonedState {}

/// What a [`MutableBundle`] function runs with: its [`NativeCtx`] and the
/// bundle's state.
///
/// The function locks the state with [`lock`](Self::lock), and calls back into
/// Frost through [`native`](Self::native), but never both at once: holding the
/// state while calling back would deadlock should the call reach the same
/// bundle, so the borrow checker forbids it. Between locks, a function may
/// freely do either:
///
/// ```
/// use frostlang::native::{Function, MutableBundle};
/// use frostlang::{FrostType, Param, Params, Value};
///
/// const CALLBACK: Params = Params::new(&[Param::of(FrostType::FUNCTION)]);
///
/// let cell = MutableBundle::new(Value::Null)
///     .function("cell.update", CALLBACK, |ctx, mut args| {
///         let callback: Function = args.take()?;
///         let old = ctx.lock()?.clone();
///         let new = ctx.native().invoke(&callback, [old])?;
///         *ctx.lock()? = new.clone();
///         Ok(new)
///     })
///     .build();
/// ```
///
/// Calling back into Frost while holding the state does not compile:
///
/// ```compile_fail
/// # use frostlang::native::{Function, MutableBundle};
/// # use frostlang::{FrostType, Param, Params, Value};
/// # const CALLBACK: Params = Params::new(&[Param::of(FrostType::FUNCTION)]);
/// MutableBundle::new(Value::Null).function("cell.call", CALLBACK, |ctx, mut args| {
///     let callback: Function = args.take()?;
///     let state = ctx.lock()?;
///     let result = ctx.native().invoke(&callback, [state.clone()]);
///     drop(state);
///     result
/// });
/// ```
///
/// Nor does locking the state twice at once:
///
/// ```compile_fail
/// # use frostlang::native::MutableBundle;
/// # use frostlang::{Params, Value};
/// # const NO_PARAMS: Params = Params::new(&[]);
/// MutableBundle::new(Value::Null).function("cell.twice", NO_PARAMS, |ctx, _| {
///     let first = ctx.lock()?;
///     let second = ctx.lock()?;
///     Ok(Value::array([first.clone(), second.clone()]))
/// });
/// ```
pub struct MutableBundleCtx<'a, S> {
    native: NativeCtx<'a>,
    state: &'a StateLock<S>,
}

impl<'a, S> MutableBundleCtx<'a, S> {
    /// Locks the state, waiting while another holder has it.
    ///
    /// Fails if a panic poisoned the state, naming this function.
    pub fn lock(&mut self) -> Result<StateGuard<'_, S>, FrostError> {
        let state = self.state.lock().map_err(|PoisonedState| {
            FrostError::from_string(format!(
                "Function {} cannot run: a panic poisoned its state",
                self.native.name()
            ))
        })?;
        Ok(StateGuard {
            state,
            native: &self.native,
        })
    }

    /// The function's [`NativeCtx`], for calling back into Frost.
    pub fn native(&mut self) -> &mut NativeCtx<'a> {
        &mut self.native
    }
}

/// A [`MutableBundle`] function's hold on the bundle's state, from
/// [`MutableBundleCtx::lock`]. It dereferences to the state, and releases it
/// when dropped.
pub struct StateGuard<'a, S> {
    state: MutexGuard<'a, S>,
    native: &'a NativeCtx<'a>,
}

impl<'a, S> StateGuard<'a, S> {
    /// The function's [`NativeCtx`], for every capability but calling back
    /// into Frost.
    pub fn native(&self) -> &'a NativeCtx<'a> {
        self.native
    }
}

impl<S> Deref for StateGuard<'_, S> {
    type Target = S;

    fn deref(&self) -> &S {
        &self.state
    }
}

impl<S> DerefMut for StateGuard<'_, S> {
    fn deref_mut(&mut self) -> &mut S {
        &mut self.state
    }
}
