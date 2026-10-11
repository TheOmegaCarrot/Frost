//! Bundles: Maps of natives over one shared state.

use std::sync::Arc;

use crate::native::{Args, IntoNativeResult};
use crate::{MapKey, NativeCtx, Params, Value};

/// A Map of natives over one state that never changes, such as a compiled
/// pattern with its matching functions, built one function at a time.
///
/// Each function receives a [`BundleCtx`], through which it reads the state
/// and calls back into Frost, and its arguments as [`Args`]. The bundle owns
/// the state, or shares it with the host through an [`Arc`]. For a state that
/// changes, see [`MutableBundle`](crate::native::MutableBundle).
///
/// ```
/// use frostlang::native::{Bundle, FrostArg};
/// use frostlang::{Params, Value};
///
/// const NO_PARAMS: Params = Params::new(&[]);
/// const AMOUNT: Params = Params::new(&[<f64 as FrostArg>::PARAM.named("amount")]);
///
/// let scale = Bundle::new(2.5_f64)
///     .function("scale.factor", NO_PARAMS, |ctx, _| Ok(*ctx.state()))
///     .function("scale.apply", AMOUNT, |ctx, mut args| {
///         let amount: f64 = args.take()?;
///         Ok(amount * ctx.state())
///     })
///     .build();
/// ```
#[must_use = "a bundle does nothing until built"]
pub struct Bundle<S> {
    state: Arc<S>,
    functions: Vec<(&'static str, Value)>,
}

impl<S: Send + Sync + 'static> Bundle<S> {
    /// A bundle owning `state`.
    pub fn new(state: S) -> Self {
        Self::from_shared(Arc::new(state))
    }

    /// A bundle sharing `state` with whoever else holds it, such as the host.
    pub fn from_shared(state: Arc<S>) -> Self {
        Self {
            state,
            functions: Vec::new(),
        }
    }

    /// Adds the function `name`, taking arguments per `params`.
    ///
    /// Its key in the bundle is `name` after the last `.`, so
    /// `"compiled_regex.matches"` is keyed `matches`.
    ///
    /// # Panics
    ///
    /// If the bundle already has a function with that key.
    pub fn function<F, R>(self, name: &'static str, params: Params, function: F) -> Self
    where
        F: Fn(&mut BundleCtx<'_, S>, Args<'_>) -> R + Send + Sync + 'static,
        R: IntoNativeResult,
    {
        self.function_over_state(name, params, move |native, state, args| {
            function(&mut BundleCtx { native, state }, args)
        })
    }

    /// Adds the function `name`, which receives the state apart from its
    /// [`NativeCtx`], so that a bundle kind can give it its own context.
    pub(super) fn function_over_state<F, R>(
        mut self,
        name: &'static str,
        params: Params,
        function: F,
    ) -> Self
    where
        F: Fn(NativeCtx<'_>, &S, Args<'_>) -> R + Send + Sync + 'static,
        R: IntoNativeResult,
    {
        let key = name.rsplit_once('.').map_or(name, |(_, key)| key);
        assert!(
            self.functions.iter().all(|(existing, _)| *existing != key),
            "the bundle already has a function keyed {key}"
        );
        let state = Arc::clone(&self.state);
        let checked = params.clone();
        let native = Value::checked_native(name, checked, move |native, args| {
            let args = Args::new(name, params.clone(), args);
            function(native, &state, args).into_result(name)
        });
        self.functions.push((key, native));
        self
    }

    /// The bundle: a Map of its functions by key.
    pub fn build(self) -> Value {
        self.functions
            .into_iter()
            .map(|(key, function)| (MapKey::from(key), function))
            .collect()
    }
}

/// What a [`Bundle`] function runs with: its [`NativeCtx`] and the bundle's
/// state.
///
/// The state stays readable while the function calls back into Frost:
///
/// ```
/// use frostlang::native::{Bundle, Function};
/// use frostlang::{FrostType, Param, Params, Value};
///
/// const CALLBACK: Params = Params::new(&[Param::of(FrostType::FUNCTION)]);
///
/// let names = Bundle::new(vec![Value::from("Ada"), Value::from("Grace")])
///     .function("names.each", CALLBACK, |ctx, mut args| {
///         let callback: Function = args.take()?;
///         let names = ctx.state();
///         for name in names {
///             ctx.native().invoke(&callback, [name.clone()])?;
///         }
///         Ok(())
///     })
///     .build();
/// ```
pub struct BundleCtx<'a, S> {
    native: NativeCtx<'a>,
    state: &'a S,
}

impl<'a, S> BundleCtx<'a, S> {
    /// The bundle's state.
    pub fn state(&self) -> &'a S {
        self.state
    }

    /// The function's [`NativeCtx`], for calling back into Frost.
    pub fn native(&mut self) -> &mut NativeCtx<'a> {
        &mut self.native
    }
}
