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
    AbortReason, Arity, CancelToken, ChildVmFactory, Closure, ComponentBuilder, Extension,
    ExtensionBuilder, ExtensionError, HostComponent, HostComponentBuilder, HostComponentError,
    IdleVm, ImportCtx, ImportResolver, Importer, ImporterBuilder, InvalidComponentName,
    InvalidParams, MissingCaptures, ModuleId, NativeCtx, NativeFn, NativeFunction, Param, Params,
    PrintSink, ProgramResult, RunError, RunErrorKind, Stdlib, StdlibError, StdlibModule,
    StdoutSink, TrustedProgram, Vm, VmFactory, VmRuntimeConfiguration,
};

/// Makes a Frost function from a Rust function.
///
/// `#[frost_fn]` keeps the function as written,
/// and adds its maker: a `const fn` named after it plus `_frost`,
/// with the same visibility,
/// that returns its [`NativeDef`](native::NativeDef),
/// which builds the Frost function [`Value`] and names it.
///
/// ```
/// use frostlang::{Value, frost_fn};
///
/// #[frost_fn]
/// fn repeat(text: String, times: usize) -> String {
///     text.repeat(times)
/// }
///
/// let repeat_for_frost: Value = repeat_frost().build();
/// // The function stays callable from Rust.
/// assert_eq!(repeat("ab".to_string(), 2), "abab");
/// ```
///
/// # Base name
///
/// The definition's base name is the function's name, less any `r#`.
/// `#[frost_fn(name = "...")]` gives another, as a string literal,
/// as when two functions in one Rust module are both `encode` to Frost,
/// placed in different modules.
/// Either way, it is a Frost identifier, not a keyword;
/// this is checked at compile time.
///
/// # Parameters
///
/// Each parameter's type gives its [`Param`] through
/// [`FrostArg`](native::FrostArg):
/// the types it accepts, and whether it is required,
/// [`Optional`](native::Optional), or a [`Rest`](native::Rest).
/// Parameters that break the rules of [`Params`] fail to compile.
///
/// Errors refer to an argument by its position and its parameter's name,
/// as in `argument 2 (times)`.
/// Mark a parameter `#[frost(unnamed)]` to leave its name out,
/// as suits an obvious leading argument;
/// its pattern then need not be a name, such as `_` or a destructuring one.
///
/// Mark a parameter `#[frost(types = ...)]` to accept only the types listed,
/// joined by `|` and named as Frost's `is` names them, categories included.
/// They must be among the types its Rust type accepts, or it fails to compile,
/// so this mostly serves a [`Value`] parameter
/// over a set of types no argument type covers.
/// Markers combine:
///
/// ```
/// use frostlang::{Value, frost_fn};
///
/// #[frost_fn]
/// fn is_empty(#[frost(unnamed, types = Flat | Array)] sequence: Value) -> bool {
///     match sequence {
///         Value::String(text) => text.is_empty(),
///         Value::Bytes(bytes) => bytes.is_empty(),
///         Value::Array(array) => array.is_empty(),
///         _ => unreachable!("the constraint admits no other type"),
///     }
/// }
/// ```
///
/// # Context
///
/// A first parameter marked `#[frost(ctx)]`, of type `&mut NativeCtx<'_>`,
/// takes no argument: it receives the call's [`NativeCtx`],
/// for calling back into Frost.
/// Rust code cannot create a `NativeCtx`,
/// so only Frost can call a function that takes one;
/// leave it out of a function you also call from Rust.
///
/// ```
/// use frostlang::native::Function;
/// use frostlang::{FrostResult, NativeCtx, Value, frost_fn};
///
/// #[frost_fn]
/// fn apply_twice(
///     #[frost(ctx)] ctx: &mut NativeCtx<'_>,
///     function: Function,
///     value: Value,
/// ) -> FrostResult {
///     let once = ctx.invoke(&function, [value])?;
///     ctx.invoke(&function, [once])
/// }
///
/// let apply_twice_for_frost: Value = apply_twice_frost().build();
/// ```
///
/// # Result
///
/// The function returns any [`IntoNativeResult`](native::IntoNativeResult);
/// returning nothing returns Null.
///
/// # Configuration
///
/// `#[cfg]` works on the function written either before or after the macro:
/// either way, the maker exists exactly when the function does.
///
/// # Restrictions
///
/// The function is a free function, not a method.
/// It is not generic (including through `impl Trait` parameters),
/// `async`, `unsafe`, or `extern`.
/// Each parameter not marked `#[frost(unnamed)]` is a plain name,
/// such as `text` or `mut text`.
pub use frostlang_macros::frost_fn;

/// Makes a Frost function from a Rust closure.
///
/// `frost_closure!(name, closure)` returns the Frost function [`Value`]
/// named `name`, which calls `closure`.
/// `name` is any `&'static str` expression,
/// evaluated once, when the function is made,
/// so a factory may name each function it makes:
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
/// The closure's parameters, context, and result work as with [`frost_fn`];
/// this covers only the differences.
///
/// # Parameters
///
/// A parameter written without a type is a [`Value`], accepting any value.
/// The `#[frost(ctx)]` parameter may omit its type too:
///
/// ```
/// use frostlang::native::Function;
/// use frostlang::{Value, frost_closure};
///
/// let apply_for_frost: Value = frost_closure!("apply", |#[frost(ctx)] ctx, function: Function, value| {
///     ctx.invoke(&function, [value])
/// });
/// ```
///
/// # Restrictions
///
/// The closure must be `Fn + Send + Sync + 'static`,
/// usually by capturing with `move`:
/// Frost may call it from any thread, and more than once.
/// It is not `async` or `const`, and has no `for<...>` binder.
pub use frostlang_macros::frost_closure;

/// Makes a group of Frost functions over state that never changes,
/// from the functions of an impl block,
/// to add to a [`Bundle`](native::Bundle).
///
/// `#[frost_bundle(prefix = "...")]` keeps the impl block as written,
/// and adds to the type a private associated function, `frost_functions`,
/// that takes a `Bundle<Self>`, adds the block's functions to it,
/// and returns it: a group, for [`Bundle::with`](native::Bundle::with).
/// Each function is keyed by its own name,
/// and its Frost name, which errors show, is the prefix, a dot, and the key:
/// below, `greet` is keyed `greet` and named `greeting.greet`.
/// `prefix` is required, and is a string literal.
///
/// ```
/// use frostlang::native::Bundle;
/// use frostlang::{Value, frost_bundle};
///
/// struct Greeting(String);
///
/// #[frost_bundle(prefix = "greeting")]
/// impl Greeting {
///     fn greet(&self, name: String) -> String {
///         format!("{}, {name}", self.0)
///     }
/// }
///
/// let greeting = Greeting(String::from("Hello"));
/// let greeting_for_frost: Value = Bundle::new(greeting)
///     .with(Greeting::frost_functions)
///     .build();
/// ```
///
/// # Functions
///
/// Every function in the block joins the group.
/// Each takes its arguments, context, and result as with [`frost_fn`],
/// and takes the state in one of three ways:
///
/// - As `&self`. Its `#[frost(ctx)]` parameter, if any,
///   comes first after the receiver, with type `&mut NativeCtx<'_>`;
///   the state stays readable while it calls back into Frost.
/// - Through its context: with no receiver,
///   a first parameter marked `#[frost(ctx)]`, of type
///   `&mut BundleCtx<'_, Self>`, receives the whole
///   [`BundleCtx`](native::BundleCtx).
/// - Not at all: with neither, it is a function of its arguments alone.
///
/// # Groups
///
/// `group = name` names the generated function `name`
/// instead of `frost_functions`,
/// as a second annotated impl block on the same type must.
/// Each group adds to the builder it is given,
/// so groups combine into a bundle of optional parts
/// through [`Bundle::with_if`](native::Bundle::with_if);
/// see [`frost_mutable_bundle`] for an example.
///
/// # Configuration
///
/// `#[cfg]` works on the impl block written either before or after the macro:
/// either way, the generated function exists exactly when the block does.
/// It works on a function in the block too:
/// the function is added exactly when it exists.
///
/// # Restrictions
///
/// The prefix and each function's name are Frost identifiers, not keywords;
/// this is checked at compile time.
/// The impl block is inherent, not generic, and holds only functions,
/// so keep any other item in an impl block of its own.
/// Its functions follow the restrictions of [`frost_fn`],
/// but for taking a receiver as above.
pub use frostlang_macros::frost_bundle;

/// Makes a group of Frost functions over state that changes,
/// from the functions of an impl block,
/// to add to a [`MutableBundle`](native::MutableBundle).
///
/// It works as [`frost_bundle`] does,
/// except in how each function takes the state.
///
/// # Functions
///
/// Each function takes the state in one of three ways:
///
/// - As `&self` or `&mut self`, locked for the call.
///   Its `#[frost(ctx)]` parameter, if any, has type `&NativeCtx<'_>`:
///   every capability but calling back into Frost,
///   which holding the state forbids.
/// - Through its context: with no receiver,
///   a first parameter marked `#[frost(ctx)]`, of type
///   `&mut MutableBundleCtx<'_, Self>`, receives the whole
///   [`MutableBundleCtx`](native::MutableBundleCtx),
///   to lock the state itself and call back into Frost between locks.
/// - Not at all: with neither, it is a function of its arguments alone.
///
/// # Groups
///
/// Here a second group makes resetting the counter optional:
///
/// ```
/// use frostlang::native::MutableBundle;
/// use frostlang::{Value, frost_mutable_bundle};
///
/// struct Counter(i64);
///
/// #[frost_mutable_bundle(prefix = "counter")]
/// impl Counter {
///     fn get(&self) -> i64 {
///         self.0
///     }
///
///     fn add(&mut self, amount: i64) -> i64 {
///         self.0 += amount;
///         self.0
///     }
/// }
///
/// #[frost_mutable_bundle(prefix = "counter", group = reset_functions)]
/// impl Counter {
///     fn reset(&mut self) {
///         self.0 = 0;
///     }
/// }
///
/// fn counter(resettable: bool) -> Value {
///     MutableBundle::new(Counter(0))
///         .with(Counter::frost_functions)
///         .with_if(resettable, Counter::reset_functions)
///         .build()
/// }
/// ```
pub use frostlang_macros::frost_mutable_bundle;

// What the binding macros' generated code names: its only path into this
// crate, so that public paths may move without breaking the macros.
#[doc(hidden)]
pub mod __private {
    pub use crate::native::{Args, Bundle, FrostArg, IntoNativeResult, MutableBundle};
    pub use crate::{FrostType, NativeCtx, Params, Value, is_identifier_like_and_not_keyword};

    pub use crate::native::NativeDef;

    use std::borrow::Cow;

    use crate::{Param, TypeSet};

    /// The definition of a native, which `make` builds under a given name,
    /// for `#[frost_fn]`.
    pub const fn native_def(
        base_name: &'static str,
        make: fn(Cow<'static, str>) -> Value,
    ) -> NativeDef {
        NativeDef::new(base_name, make)
    }

    /// `param`, narrowed to accept only `types`, for a `#[frost(types = ...)]`
    /// marker. Evaluated in a const context, it fails to compile when `types`
    /// reaches outside what `param`'s Rust type accepts, since that type's
    /// `FromArg` may treat any other type as unreachable.
    pub const fn constrain(param: Param, types: TypeSet) -> Param {
        assert!(
            types.is_subset(param.types()),
            "a `types` constraint must stay within the types its parameter's Rust type accepts"
        );
        param.with_types(types)
    }
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
