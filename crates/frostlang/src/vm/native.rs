//! Native (Rust-implemented) Frost functions: the [`NativeFn`] bound,
//! [`NativeFunction`], the [`NativeCtx`] handle back into the VM,
//! and the [`Value::native`] / [`Value::checked_native`] constructors.

use std::borrow::Cow;
use std::sync::Arc;

use crate::native::{ArgSite, Function};
use crate::{FrostError, FrostResult, Value};

use super::Vm;
use super::function::Arity;
use super::params::{Binder, Params};

/// A native function's handle back into the running [`Vm`], passed to every native call.
///
/// Its methods that can run Frost code, such as [`invoke`](Self::invoke), take
/// `&mut self`; every other takes `&self`. So a shared `&NativeCtx` offers every
/// capability but calling back into Frost, which is how a
/// [`MutableBundleCtx`](crate::native::MutableBundleCtx) forbids calling back while holding
/// its state.
// Keep to this rule when adding a method: a `&self` method that runs Frost code
// would let a bundle function deadlock on its own state.
#[derive(Debug)]
pub struct NativeCtx<'a> {
    pub(crate) vm: &'a mut Vm,
    pub(super) function: &'a NativeFunction,
}

impl<'a> NativeCtx<'a> {
    /// Calls a Frost function from inside a native function.
    ///
    /// Returns the callee's result, or the error it raised. The Vm is restored before
    /// an `Err` is returned, so the caller may catch it and continue.
    ///
    /// A native takes a function argument as a [`Function`]; a host converts a
    /// [`Value`] to one with `Function::try_from`.
    pub fn invoke(
        &mut self,
        function: &Function,
        args: impl IntoIterator<Item = Value>,
    ) -> FrostResult {
        // Once the run is fatally aborting, refuse to do any more Frost work; this is
        // what stops a native that caught the abort and tried to re-enter the Vm.
        if let Some(err) = self.vm.abort_error() {
            return Err(err);
        }
        // A native calling back into Frost is itself a call: it burns fuel.
        self.vm.expend_fuel()?;

        let function = function.as_value();
        match function {
            Value::NativeFunction(native) => {
                let mut buf = self.vm.native_arg_pool.pop().unwrap_or_default();
                buf.extend(args);
                self.vm.run_native(native, buf)
            }

            Value::Closure(closure) => {
                let base = self.vm.stack.len();
                self.vm.stack.push(function.clone());
                self.vm.stack.extend(args);
                let argc = self.vm.stack.len() - base - 1;

                // This native boundary is where an unwinding error stops.
                // On any error path below, restore the Vm to its pre-call shape before returning Err
                // (truncate the operand stack back to `base`, discard the Frost frames left above our entry floor),
                // so that a catching native (e.g. `try_call`) resumes on a clean Vm.
                if let Err(err) =
                    Vm::check_arity(closure.function.arity, argc, &closure.function.name)
                {
                    // The callee never ran: only the function value and its args
                    // (no frame) sit above `base`.
                    self.vm.stack.truncate(base);
                    return Err(err);
                }

                let frame_floor = self.vm.stack_frames.len();
                if let Err(err) = self.vm.push_closure_frame(base, None) {
                    // No frame was pushed (the depth check runs first), so just clear
                    // the function value and args left above `base`.
                    self.vm.stack.truncate(base);
                    return Err(err);
                }

                if let Err(err) = self.vm.execute_function() {
                    let err = self.vm.unwind_frames(frame_floor, err);
                    self.vm.stack.truncate(base);
                    return Err(err);
                }

                // Mirror the normal VM return path's guard: the body must leave
                // exactly its result at the frame base, with no leaked or stolen
                // caller operand.
                debug_assert_eq!(
                    self.vm.stack.len(),
                    base + 1,
                    "an invoked closure must leave exactly its result at the frame base"
                );

                let result = self
                    .vm
                    .stack
                    .pop()
                    .expect("IMPOSSIBLE: closure left no result");
                self.vm.pop_vm_frame();
                Ok(result)
            }
            other => unreachable!("a Function holds a function, not {}", other.type_name()),
        }
    }

    /// Calls a Frost function as [`invoke`](Self::invoke) does, with arguments by
    /// reference.
    pub fn invoke_ref<'v>(
        &mut self,
        function: &Function,
        args: impl IntoIterator<Item = &'v Value>,
    ) -> FrostResult {
        self.invoke(function, args.into_iter().cloned())
    }

    /// Type-check the running native's args against `params`, attributing the error
    /// to this native by name. Forwards to [`NativeFunction::check_args`].
    pub fn check_args(&self, args: &[Value], params: &Params) -> Result<(), FrostError> {
        self.function.check_args(args, params)
    }

    /// The running native's name.
    ///
    /// It borrows the native, not this context, so it outlives a move of the
    /// context.
    pub fn name(&self) -> &'a str {
        &self.function.name
    }
}

/// The bound every native function must satisfy.
///
/// The `&mut [Value]` arguments are the callee's to consume: steal one with
/// [`Value::take`] (leaving `Null` behind) rather than cloning when you need to own it.
/// The caller discards the buffer once the call returns, so any values left in it are dropped.
pub trait NativeFn: Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static {
    /// Invoke the native with `ctx` and `args`.
    fn invoke(&self, ctx: NativeCtx<'_>, args: &mut [Value]) -> FrostResult {
        self(ctx, args)
    }
}
impl<F> NativeFn for F where
    F: Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static
{
}

/// A Rust-implemented Frost function, as held by [`Value::NativeFunction`].
pub struct NativeFunction {
    pub(super) arity: Arity,
    pub(super) function: Box<dyn NativeFn>,
    pub(super) name: Cow<'static, str>,
}

impl std::fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeFunction")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .finish_non_exhaustive()
    }
}

impl NativeFunction {
    /// Build a native function with an explicit [`Arity`]; `function` validates its own args.
    ///
    /// Most callers want [`Value::native`] instead: it wraps the result in a [`Value`].
    /// Reach for this only when you specifically need a bare [`NativeFunction`].
    pub fn new<F>(name: impl Into<Cow<'static, str>>, arity: Arity, function: F) -> Self
    where
        F: NativeFn,
    {
        Self {
            arity,
            name: name.into(),
            function: Box::new(function),
        }
    }

    /// Build a native whose [`Arity`] comes from `params`, and whose arguments
    /// are type-checked against `params` before `body` runs, so `body` may trust
    /// its argument types.
    ///
    /// Most callers want [`Value::checked_native`] instead: it wraps the result in a [`Value`].
    /// Reach for this only when you specifically need a bare [`NativeFunction`].
    pub fn checked<F>(name: impl Into<Cow<'static, str>>, params: Params, body: F) -> Self
    where
        F: NativeFn,
    {
        Self {
            arity: params.arity(),
            name: name.into(),
            function: Box::new(move |ctx, args| {
                ctx.check_args(args, &params)?;
                body(ctx, args)
            }),
        }
    }

    /// Type-check `args` against `params`, reporting a mismatch as
    /// `Function {name} requires {types} as argument {N}{ (name)}, got {Type}`.
    /// A guard to call before trusting argument types in a native body. Arity is
    /// assumed already validated (the VM checks a native's `Arity` before its body
    /// runs), so only the present arguments' types are checked.
    ///
    /// Which parameter each argument is checked against follows the rules on
    /// [`Params`]: every argument a rest parameter takes is checked against
    /// its types, and an omitted optional parameter checks nothing.
    pub fn check_args(&self, args: &[Value], params: &Params) -> Result<(), FrostError> {
        let mut binder = Binder::new(params, args.len());
        for param in params.as_slice() {
            for i in binder.bind(param) {
                let Some(arg) = args.get(i) else {
                    return Ok(());
                };
                if !param.accepts(arg) {
                    return Err(
                        ArgSite::argument(&self.name, i, param.name).wrong_type(param.types(), arg)
                    );
                }
            }
        }
        Ok(())
    }

    /// The function's name, as reported in errors and backtraces.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The number of arguments the function accepts.
    pub fn arity(&self) -> Arity {
        self.arity
    }
}

impl Value {
    /// Build a Frost function value from a Rust closure.
    ///
    /// When called from Frost, `function` runs with a [`NativeCtx`] (for calling back
    /// into the interpreter) and the call's arguments; whatever it returns becomes the
    /// result. `arity` declares how many arguments the function accepts; calling it
    /// with the wrong number produces a Frost error. The arguments are yours to
    /// consume: steal one with [`Value::take`] rather than cloning (see [`NativeFn`]).
    ///
    /// Reach for this when the function validates its own arguments: it accepts
    /// flexible types, or which types are valid depends on more than one argument at
    /// once. To have argument types checked for you, use [`Value::checked_native`].
    pub fn native<F>(name: impl Into<Cow<'static, str>>, arity: Arity, function: F) -> Value
    where
        F: NativeFn,
    {
        Value::NativeFunction(Arc::new(NativeFunction::new(name, arity, function)))
    }

    /// Build a Frost function value whose arguments are type-checked for you.
    ///
    /// `params` describes each parameter's accepted types and whether it is optional
    /// (see [`Params`]). The function's arity comes from the spec, and every argument
    /// is validated before `body` runs, so `body` can assume its arguments already
    /// match the spec. A bad argument raises a Frost error instead.
    ///
    /// This is the usual way to expose a Rust function to Frost. Use [`Value::native`]
    /// when the valid types can't be described per parameter and the function must
    /// check them itself.
    pub fn checked_native<F>(name: impl Into<Cow<'static, str>>, params: Params, body: F) -> Value
    where
        F: NativeFn,
    {
        Value::NativeFunction(Arc::new(NativeFunction::checked(name, params, body)))
    }
}
