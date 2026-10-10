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

#[cfg(feature = "compile")]
pub mod compile;
mod core;
#[cfg(feature = "image")]
pub mod image;
pub mod stdlib;
mod vm;

// Type sets (`EnumSet<FrostType>`) appear in the public API, so the enumset items
// are re-exported for consumers.
pub use enumset::{EnumSet, enum_set};

pub use core::{
    BacktraceFrame, FrostArray, FrostBytes, FrostError, FrostFloat, FrostMap, FrostOpaque,
    FrostResult, FrostString, FrostType, KEYWORDS, MapKey, OpaqueHandle, SpecialFloat, Value,
    ValueMap, WithBacktrace, from_value, is_identifier_like, is_identifier_like_and_not_keyword,
    is_reserved_keyword, to_value, value_map,
};

pub use vm::{
    AbortReason, Arity, CancelToken, ChildVmFactory, Closure, Extension, ExtensionError,
    HostComponent, HostComponentError, IdleVm, ImportCtx, ImportResolver, Importer,
    ImporterBuilder, InvalidComponentName, InvalidParams, MissingCaptures, ModuleId, NativeCtx,
    NativeFn, NativeFunction, Param, Params, PrintSink, ProgramResult, RunError, RunErrorKind,
    Stdlib, StdlibError, StdlibModule, StdoutSink, TrustedProgram, Vm, VmFactory,
    VmRuntimeConfiguration,
};

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
