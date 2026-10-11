//! Frost: an embeddable, dynamically-typed functional programming language.
//!
//! A host compiles Frost source with the `compile` module, then runs the
//! program on a [`Vm`]. The [`Value`] type is how values pass between the host
//! and Frost.
//!
//! # Features
//!
//! - `compile` (default): the `compile` module, for compiling Frost source.
//!   Without it, a host runs only programs compiled elsewhere, loaded as images
//!   (see `image`).
//! - `graphical-diagnostics`: draw compile errors as source snippets with their
//!   labels, rather than narrating them as plain text. If you show Frost
//!   compiler errors to people, you probably want it. Implies `compile`.
//! - `image`: the `image` module, for saving compiled programs to bytes and
//!   loading them back.
//!
//! A library that extends Frost, such as one providing native functions, should
//! depend on this crate with `default-features = false`, so that its users
//! choose for themselves whether to include the compiler.

// So that the paths the binding macros generate, which start `::frostlang`,
// resolve within this crate too.
extern crate self as frostlang;

#[cfg(feature = "compile")]
pub mod compile;
mod core;
#[cfg(feature = "image")]
pub mod image;
pub mod native;
pub mod stdlib;
mod vm;

pub use core::{
    BacktraceFrame, FrostArray, FrostBytes, FrostError, FrostFloat, FrostMap, FrostOpaque,
    FrostResult, FrostString, FrostType, KEYWORDS, MapKey, OpaqueHandle, SpecialFloat, TypeSet,
    Value, ValueMap, WithBacktrace, from_value, is_identifier_like,
    is_identifier_like_and_not_keyword, is_reserved_keyword, to_value, value_map,
    write_escaped_string, write_quoted_string,
};

pub use vm::{
    AbortReason, Arity, CancelToken, ChildVmFactory, Closure, Extension, ExtensionError,
    HostComponent, HostComponentError, IdleVm, ImportCtx, ImportResolver, Importer,
    ImporterBuilder, InvalidComponentName, InvalidParams, MissingCaptures, ModuleId, NativeCtx,
    NativeFn, NativeFunction, Param, Params, PrintSink, ProgramResult, RunError, RunErrorKind,
    Stdlib, StdlibError, StdlibModule, StdoutSink, TrustedProgram, Vm, VmFactory,
    VmRuntimeConfiguration,
};

/// Makes a Frost function from a Rust function.
///
/// `#[frost_fn(name = ...)]` keeps the function as written, and adds a
/// function of the same visibility, named after it plus `_frost`, that returns
/// it as a Frost function [`Value`]. The name is any constant `&'static str`
/// expression; errors show it.
///
/// ```
/// use frostlang::{Value, frost_fn};
///
/// #[frost_fn(name = "text.repeat")]
/// fn repeat(text: String, times: usize) -> String {
///     text.repeat(times)
/// }
///
/// let repeat_for_frost: Value = repeat_frost();
/// // The function stays callable from Rust.
/// assert_eq!(repeat("ab".to_string(), 2), "abab");
/// ```
///
/// # Parameters
///
/// Each parameter's type gives its [`Param`] through
/// [`FrostArg`](native::FrostArg): the types it accepts, and whether it is
/// required, [`Optional`](native::Optional), or a [`Rest`](native::Rest). Its
/// name names it in errors. Parameters that break the rules of [`Params`] fail
/// to compile.
///
/// A first parameter marked `#[frost(ctx)]`, of type `&mut NativeCtx<'_>`,
/// receives the call's [`NativeCtx`], for calling back into Frost. Rust code
/// cannot create a `NativeCtx`, so a function that takes one can only be
/// called by Frost; leave it out of a function you also call from Rust.
///
/// ```
/// use frostlang::{FrostResult, NativeCtx, Value, frost_fn};
///
/// #[frost_fn(name = "apply_twice")]
/// fn apply_twice(
///     #[frost(ctx)] ctx: &mut NativeCtx<'_>,
///     function: Value,
///     value: Value,
/// ) -> FrostResult {
///     let once = ctx.invoke(&function, [value])?;
///     ctx.invoke(&function, [once])
/// }
///
/// let apply_twice_for_frost: Value = apply_twice_frost();
/// ```
///
/// # Result
///
/// The function returns any [`IntoNativeResult`](native::IntoNativeResult);
/// returning nothing returns Null.
///
/// # Restrictions
///
/// The function is a plain free function: not a method, and not generic
/// (including through `impl Trait` parameters), `async`, `unsafe`, or
/// `extern`. Each parameter but the context is a plain name, such as `text`.
pub use frostlang_macros::frost_fn;

/// Makes a Frost function from a Rust closure.
///
/// `frost_closure!(name, closure)` returns the Frost function [`Value`] named
/// `name`, which calls `closure`. It takes its parameters and gives its result
/// as [`frost_fn`] does, which describes them; this covers the differences.
///
/// `name` is any `&'static str` expression, evaluated once, when the function
/// is made, so a factory may name each function it makes:
///
/// ```
/// use frostlang::{Value, frost_closure};
///
/// fn adder(name: &'static str, amount: i64) -> Value {
///     frost_closure!(name, move |n: i64| n + amount)
/// }
///
/// let add_one_for_frost: Value = adder("add_one", 1);
/// ```
///
/// A parameter written without a type is a [`Value`], accepting any value.
/// The context parameter needs no type either, as it has only the one:
///
/// ```
/// use frostlang::{Value, frost_closure};
///
/// let apply_for_frost: Value = frost_closure!("apply", |#[frost(ctx)] ctx, function, value| {
///     ctx.invoke(&function, [value])
/// });
/// ```
///
/// The closure is called through a shared reference, from whichever thread
/// runs Frost, so it must be `Fn`, `Send`, `Sync`, and `'static`, typically by
/// capturing with `move`. It is not `async` or `const`, and has no `for<...>`
/// binder.
pub use frostlang_macros::frost_closure;

// What the binding macros' generated code names: its only path into this
// crate, so that public paths may move without breaking the macros.
#[doc(hidden)]
pub mod __private {
    pub use crate::native::{Args, FrostArg, IntoNativeResult};
    pub use crate::{NativeCtx, Params, Value};
}

/// The bytecode the [`Vm`] runs, as the compiler produces it.
///
/// This module serves tools that work with bytecode directly, such as
/// disassemblers or a compiler for another language. It is exempt from semver:
/// any release may change it. A host running Frost programs needs nothing here.
pub mod bytecode {
    pub use crate::vm::{
        Bytecode, CompiledFunction, Disassembly, FormatVersion, GLOBAL_NAMES, GLOBAL_PURITY,
        NameEntry, Purity,
    };
}
