//! The Frost runtime: the [`Value`] type and its operations, and the [`Vm`] that runs compiled Frost code.

mod core;
pub mod stdlib;
mod vm;

// Type sets (`EnumSet<FrostType>`) appear in the public API, so the enumset items
// are re-exported for consumers.
pub use enumset::{EnumSet, enum_set};

pub use core::{
    BacktraceFrame, FrostArray, FrostError, FrostFloat, FrostMap, FrostOpaque, FrostResult,
    FrostType, KEYWORDS, MapKey, SpecialFloat, Value, ValueMap, WithBacktrace, from_value,
    is_identifier_like, is_identifier_like_and_not_keyword, is_reserved_keyword, to_value,
    value_map,
};

pub use vm::{
    AbortReason, Arity, Bytecode, Closure, CompiledFunction, Disassembly, Extension,
    ExtensionError, FormatVersion, GLOBAL_NAMES, GLOBAL_PURITY, HostComponent, HostComponentError,
    IdleVm, ImportCtx, ImportResolver, Importer, ImporterBuilder, InvalidComponentName,
    InvalidParams, MissingCaptures, ModuleId, NameEntry, NativeCtx, NativeFn, NativeFunction,
    Param, Params, PrintSink, ProgramResult, Purity, RunError, RunErrorKind, Stdlib, StdlibError,
    StdlibModule, StdoutSink, TrustedProgram, Vm, VmFactory, VmRuntimeConfiguration,
};
