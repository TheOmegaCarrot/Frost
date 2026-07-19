mod core;
mod vm;

// Type sets (`EnumSet<FrostType>`, built with `|` or `enum_set!`) appear in the
// public API (`Bytecode::TypeTest`, `Param::of`), so the enumset items are
// re-exported for consumers.
pub use enumset::{EnumSet, enum_set};

pub use core::{
    FrostArray, FrostError, FrostFloat, FrostMap, FrostResult, FrostType, KEYWORDS, MapKey, Value,
    from_value, to_value,
};

pub use vm::{
    Arity, Bytecode, Closure, CompiledFunction, FormatVersion, GLOBAL_NAMES, InvalidParams,
    MissingCaptures, NameEntry, NativeCtx, NativeFn, NativeFunction, Param, Params, ProgramResult,
    RunError, RunOutcome, TrustedProgram, Vm, VmFactory, VmRuntimeConfiguration,
};
