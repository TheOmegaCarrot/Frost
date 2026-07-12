#![allow(unused)]

mod globals;
mod params;
mod serialize;

pub use globals::GLOBAL_NAMES;
pub use params::{Param, ParamSpec};
pub use serialize::FormatVersion;

// White-box tests for the one unwind invariant not observable through the public
// API: native-arg-pool buffer recycling. (Operand-stack and frame restoration are
// covered black-box in tests/vm_errors.rs.) Kept in their own file -- a child
// module still reaches this module's private `Vm` internals.
#[cfg(test)]
mod arg_pool_tests;

use std::debug_assert_matches;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::{borrow::Cow, collections::BTreeMap};

use itertools::Itertools;

use crate::{
    FrostArray, FrostError, FrostFloat, FrostMap, FrostResult, FrostTypeCategory, MapKey, Value,
};

// ============================================================
// Bytecode
// ============================================================

#[derive(Clone, Debug, Copy, serde::Serialize, serde::Deserialize)]
pub enum Bytecode {
    // Constants
    PushNull,
    PushTrue,
    PushFalse,
    PushInt(i64),
    PushFloat(FrostFloat),

    // Stack
    Pop,
    Dup,             // Duplicate the top item on the stack
    PeekDown(usize), // Index N down from the top of the stack, and copy that onto the top.
    // PeekDown(1) is just Dup with extra steps.

    // Drop the element N items from the top of the stack.
    // `DropBelow(0)` has the equivalent effect as `Pop`.
    DropBelow(usize),

    // Slots
    DefLocal(usize),   // Move the top of the stack to local slot N
    LoadLocal(usize),  // Copy local slot N to the top of the stack
    LoadConst(usize),  // Copy constant slot N to the top of the stack
    LoadGlobal(usize), // Copy global slot N to the top of the stack

    // Arithmetic
    // Consume 2 stack items, and the rhs is the top item, result is one item on the stack
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,

    // Comparison
    // Same stack conventions as arithmetic
    CompareEqual,
    CompareNotEqual,
    CompareLessThan,
    CompareLessThanOrEqual,
    CompareGreaterThan,
    CompareGreaterThanOrEqual,

    // Unary
    LogicalNot,
    Negate,

    // Flow
    // Jump ahead N instructions
    // N is the number of instructions that are skipped over,
    // such that `Jump(0)` is a funny way to spell `Nop`
    Jump(usize),        // unconditionally
    JumpIfTrue(usize),  // only if the top of the stack is true (NOT consumed)
    JumpIfFalse(usize), // only if the top of the stack is falsey (NOT consumed)

    // Functions
    // N args on the stack, with a function under the args
    // The last argument is the top of the stack
    Call(usize),
    TailCall(usize),

    // Dynamic-arity call: ( f arg_arry -- r )
    // Purpose-built for the `call` builtin
    DynTailCall,

    // Move num_captures elements from the stack to a closure capture structure,
    // as a part of a new closure with code from function (index into function table)
    CreateClosure { num_captures: u32, function: u32 },

    // Data structures

    // Consume N items from the stack, pushing an Array to the stack
    // The top of the stack is the back of the array
    MakeArray(usize),
    // Consume 2N items from the stack, as kv pairs
    // Keys are below their corresponding values
    MakeMap(usize),
    // Inverse of MakeArray: blast Array contents onto the stack
    ExplodeArray,

    // Index a structure, structure is below the index initially (consumed)
    // Leaves a single value on the stack
    SoftIndexStructure, // Null on missing

    // Index a Map with a constant String key.
    // The key is in the constant pool at the index stored in this variant.
    HardIndexMap(usize), // Error on missing

    // Consumes the value at the top of the stack, and produces a bool depending if the value fits
    // the given type category.
    TypeTest(FrostTypeCategory),

    // Consume the value atop the stack and attach it to an error.
    // The error is then produced, and enters the usual flow of a user-code error.
    ProduceError,
}

// ============================================================
// VM Types
// ============================================================

/// VM state and execution context.
#[derive(Debug)]
pub struct Vm {
    // The working stack of the Vm
    stack: Vec<Value>,

    // The call stack. Empty until `run`/`run_with_args` seats the top-level closure's frame;
    // a Frost call pushes a `VmFrame`, a native call a `NativeFrame` marker.
    // After a run, `stack_frames[0]` is the top-level frame, whose locals are the script's bindings/exports.
    stack_frames: Vec<StackFrame>,

    // Used to hold the args of a native function call.
    // A native call acquires a Vec from this pool, moves args from the stack to that Vec (or makes a new one), then clears it and returns it.
    // This allows for re-use of allocations for native args, while allowing a native call to hold mutable references to their args AND the Vm separately.
    native_arg_pool: Vec<Vec<Value>>,

    globals: Arc<GlobalSet>,

    // The top-level closure to run. Its captures (host + Frost-internal) are already bound;
    // `run`/`run_with_args` invoke it like any other closure.
    top_level: Arc<Closure>,

    // Runtime resource limits. Fixed at build time; persists across `reset`.
    config: VmRuntimeConfiguration,

    // Function calls made so far (the fuel meter). Incremented on every call and
    // compared against `config.fuel`; zeroed on `reset`.
    fuel_used: usize,

    // The unrecoverable-error channel.
    // Once set (e.g. fuel exhaustion) the run is fatally aborting:
    // unlike an ordinary error this cannot be caught.
    abort: Option<FrostError>,
}

/// Runtime resource limits for a [`Vm`]. The [`Default`] imposes no limits.
#[derive(Debug, Clone, Default)]
pub struct VmRuntimeConfiguration {
    /// Maximum call-stack depth (number of frames) before execution fails with a
    /// recoverable error.
    /// Tail calls do not contribute to the call depth.
    /// `None` leaves depth unbounded.
    pub max_call_depth: Option<NonZeroUsize>,

    /// Call budget ("fuel"): execution fails once this many function calls have been made.
    /// `None` leaves execution unmetered.
    /// This call budget does count tail calls.
    /// Because iteration in Frost is achieved with higher-order functions and/or tail-call
    /// recursion, a Frost script often involves more function calls than comparable code in a more
    /// procedural language like Lua. Consider setting this value higher than your intuition may
    /// lead you.
    ///
    /// Frost bytecode has no backward jumps, so each function body runs a bounded number
    /// of instructions -- bounding the number of *calls* therefore bounds total execution.
    /// This is what catches unbounded recursion, tail recursion included (which is
    /// depth-flat, so the depth cap never fires on it). A native function that loops forever
    /// without returning or re-entering the Vm is outside this guarantee.
    pub fuel: Option<NonZeroUsize>,
}

/// Fixed configuration from which [`Vm`]s are built. Obtain one from [`Vm::factory`].
///
/// The final [build](VmFactory::build) is what binds the script to execute.
/// This factory is well-suited to creating several identically-configured Vms.
#[derive(Debug, Clone, Default)]
pub struct VmFactory {
    config: VmRuntimeConfiguration,
}

impl VmFactory {
    /// Set the runtime resource limits (depth, fuel) for the Vms this factory builds.
    /// See [`VmRuntimeConfiguration`] for the defaults,
    /// which are used if this method is not invoked.
    pub fn configuration(mut self, config: VmRuntimeConfiguration) -> Self {
        self.config = config;
        self
    }

    /// Build a [`Vm`] to run `closure` under this factory's configuration.
    pub fn build(&self, closure: Arc<Closure>) -> Result<Vm, FrostError> {
        Ok(Vm {
            stack: Vec::new(),
            stack_frames: Vec::new(),
            native_arg_pool: Vec::new(),
            globals: GlobalSet::defaults(),
            top_level: closure,
            config: self.config.clone(),
            fuel_used: 0,
            abort: None,
        })
    }
}

// A fixed, pure-internal set of predefined globals shared by every `Vm` (observation
// only; never host-configurable). The compiler's seam onto it is [`GLOBAL_NAMES`].
#[derive(Debug, Clone)]
pub(crate) struct GlobalSet(Vec<Value>);

/// Compiled representation of a single function.
/// A script's top-level is also a function.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompiledFunction {
    // Version marker: stamps the runtime version on serialize, rejects a mismatch on
    // deserialize. First, so a mismatched image fails before the rest is decoded.
    pub version: serialize::FormatVersion,
    // Functions always have a name
    pub name: String,
    pub code: Vec<Bytecode>,
    // Functions which are defined in this function's body
    pub child_fns: Vec<Arc<CompiledFunction>>,
    // Constant values that can't be inlined in an opcode.
    // Mostly strings, but can include any structured value the compiler can constant-fold.
    // Serialized through `ConstValue` (see `serialize`); a function-valued constant is rejected.
    #[serde(with = "serialize::const_pool")]
    pub constants: Vec<Value>,
    // Table so that locals can be looked up by name at runtime,
    // or their slot given a name by an error.
    pub name_table: Vec<NameEntry>,
    // Number of leading `name_table`/slot entries that are captures (slots `0..num_captures`);
    // the remainder are locals, including params. May be 0.
    pub num_captures: usize,
    // Arity of top-level is Exact(0)
    pub arity: Arity,
}

impl CompiledFunction {
    /// The capture names this function expects.
    /// Tells a host which names to include in the map passed to [`close`](Self::close).
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.name_table[..self.num_captures]
            .iter()
            .map(|entry| entry.name.as_str())
    }

    /// Vouch that this compiled-function tree is well-formed, yielding a runnable [`TrustedProgram`].
    ///
    /// The VM only runs trusted bytecode: [`Closure`] (hence [`Vm`]) construction goes
    /// through [`TrustedProgram`], so this is the gate. The Frost compiler's output is trusted by
    /// construction; a host loading bytecode from elsewhere (deserialized, cached) uses this
    /// to vouch for it -- e.g. it controls the source and only guards tampering/corruption
    /// with a version check.
    ///
    /// This is an assertion, not a check: malformed bytecode *panics* at runtime. It is
    /// never memory-unsafe (the VM has no `unsafe`), so this is a safe contract rather than
    /// an `unsafe` one -- but a real obligation. A future verifier will offer the *checked*
    /// path (`untrusted -> Result<TrustedProgram, _>`); prefer it for bytecode you did not author.
    pub fn assert_trusted(self: Arc<Self>) -> TrustedProgram {
        TrustedProgram(self)
    }
}

/// A [`CompiledFunction`] tree asserted (or, in future, verified) well-formed, and therefore
/// runnable. Mint one via [`CompiledFunction::assert_trusted`]; bind its captures with
/// [`close`](Self::close) to obtain a [`Closure`].
///
/// "Trusted" means the bytecode upholds the Vm's internal invariants -- *not* that the program is
/// safe or well-behaved. Sandboxing is separate (see [`VmRuntimeConfiguration`]).
///
/// Note that this does not imply trust that a program does what it's intended to do, or is
/// "guaranteed safe" in all senses of "safe", but rather trust that the bytecode within upholds
/// the necessary invariants of the Vm. A script that spins forever doing nothing is "Trusted" in
/// the sense that the compiler emitted correct bytecode that never, for example, tries to pop from
/// an empty working stack, or jump out-of-bounds.
/// A `TrustedProgram` could do absolutely anything it has the capability
/// to do within the configuration of the [`Vm`], up to importable code, call stack limits, function call
/// limits, etc. A malicious script can still be "trusted" after compilation as it does uphold
/// internal Vm invariants, so sandboxing is necessary when running untrusted scripts in
/// security-conscious contexts -- and useful even for trusted-authored ones, where a stray infinite
/// loop becomes a recoverable [`RunError`] (via a [`fuel`](VmRuntimeConfiguration::fuel) limit)
/// rather than a process hang.
pub struct TrustedProgram(Arc<CompiledFunction>);

impl TrustedProgram {
    /// Bind this function's captures into a runnable [`Closure`].
    ///
    /// Required captures are looked up by name in `captures`.
    /// Extra entries in the map are ignored.
    /// Any required capture name absent from the map is reported, together, as [`MissingCaptures`].
    pub fn close(self, captures: BTreeMap<String, Value>) -> Result<Arc<Closure>, MissingCaptures> {
        let function = self.0;
        let mut seated = Vec::with_capacity(function.num_captures);
        let mut missing = Vec::new();
        for entry in &function.name_table[..function.num_captures] {
            match entry.name.as_str() {
                // Frost-internal capture: runtime-supplied, not overridable.
                // (Always false for now -- direct execution; the future `import`
                // path will need to supply `true`.)
                "imported" => seated.push(Value::Bool(false)),
                // `import` is intentionally not wired yet (registry NYI), so it
                // falls through to host resolution below.
                name => match captures.get(name) {
                    Some(value) => seated.push(value.clone()),
                    // Keep scanning so every missing name is reported at once.
                    None => missing.push(name.to_owned()),
                },
            }
        }
        if !missing.is_empty() {
            return Err(MissingCaptures { names: missing });
        }
        Ok(Arc::new(Closure {
            function,
            captures: seated,
        }))
    }

    /// Convenience for [`close`](Self::close) with no host-supplied captures.
    /// Succeeds when the function needs no host captures;
    /// otherwise returns the [`MissingCaptures`] it still requires.
    pub fn into_closure(self) -> Result<Arc<Closure>, MissingCaptures> {
        self.close(BTreeMap::new())
    }
}

/// One or more required captures were absent from the map passed to [`CompiledFunction::close`];
/// reports every missing name, not just the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingCaptures {
    pub names: Vec<String>,
}

impl std::fmt::Display for MissingCaptures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing capture(s): {}", self.names.join(", "))
    }
}

impl std::error::Error for MissingCaptures {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NameEntry {
    pub name: String,
    pub exported: bool,
}

#[derive(Debug)]
enum StackFrame {
    NativeFrame,
    VmFrame(VmFrame),
}

#[derive(Debug)]
struct VmFrame {
    // Absolute stack index of the base of a frame
    base_idx: usize,
    // Storage for local variables.
    local_slots: Vec<Option<Value>>,
    // Index into the code of the calling function which should be jumped back to.
    // The instruction _after_ the Call that pushed this StackFrame.
    return_address: Option<NonZeroUsize>,
    // The function represented by this StackFrame.
    this_fn: Arc<CompiledFunction>,
}

/// The result of executing a CompiledFunction.
/// Provides access to the top-level defined values and exports of a script.
#[derive(Debug)]
pub struct ProgramResult(Vm);

/// Representation of the arity of a Frost function.
/// Every function has a certain number of fixed args, and may or may not be variadic.
/// A native function may have a specific arity range, without being variadic.
/// ```frost
/// fn a, b, c, ...more -> ...
/// # at least 3
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Arity {
    Exact(usize),
    Between(usize, usize),
    AtLeast(usize),
}

/// Control-flow outcome of a tail call, shared by `TailCall` and `DynTailCall`.
enum TailFlow {
    /// Closure callee: the loop must re-enter at the callee's frame (`pc = 0`).
    Reenter,
    /// Native callee: it ran inline, so fall through to the next instruction.
    FellThrough,
}

#[derive(Debug)]
pub struct NativeCtx<'a> {
    pub(crate) vm: &'a mut Vm,
    function: &'a NativeFunction,
}

impl NativeCtx<'_> {
    pub fn invoke(
        &mut self,
        function: &Value,
        args: impl IntoIterator<Item = Value>,
    ) -> FrostResult {
        // Once the run is fatally aborting, refuse to do any more Frost work -- this is
        // what stops a native that caught the abort and tried to re-enter the Vm.
        if let Some(err) = self.vm.abort_error() {
            return Err(err);
        }
        // A native calling back into Frost is itself a call: it burns fuel.
        self.vm.expend_fuel()?;

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
                // On any error path below, restore the Vm to its pre-call shape before returning Err --
                // truncate the operand stack back to `base` and discard the Frost frames left above our entry floor --
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
                if let Err(err) = self.vm.push_closure_frame(closure, base, None) {
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

                let result = self
                    .vm
                    .stack
                    .pop()
                    .expect("IMPOSSIBLE: closure left no result");
                self.vm.stack_frames.pop();
                Ok(result)
            }
            _ => Err(Vm::not_callable(function)),
        }
    }

    /// Type-check the running native's args against `params`, attributing the error
    /// to this native by name. Forwards to [`NativeFunction::check_args`].
    pub fn check_args(&self, args: &[Value], params: &[Param]) -> Result<(), FrostError> {
        self.function.check_args(args, params)
    }

    pub fn name(&self) -> &str {
        self.function.name
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

pub struct NativeFunction {
    arity: Arity,
    function: Box<dyn NativeFn>,
    name: &'static str,
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
    pub fn new<F>(name: &'static str, arity: Arity, function: F) -> Self
    where
        F: NativeFn,
    {
        Self {
            arity,
            name,
            function: Box::new(function),
        }
    }

    /// Build a native whose [`Arity`] is derived from `params`, and whose arguments
    /// are type-checked against `params` before `body` runs, so `body` may trust
    /// its argument types.
    ///
    /// Most callers want [`Value::checked_native`] instead: it wraps the result in a [`Value`].
    /// Reach for this only when you specifically need a bare [`NativeFunction`].
    pub fn checked<F, const N: usize>(name: &'static str, params: [Param; N], body: F) -> Self
    where
        F: NativeFn,
    {
        Self {
            arity: params.as_slice().arity(),
            name,
            function: Box::new(move |ctx, args| {
                ctx.check_args(args, &params)?;
                body(ctx, args)
            }),
        }
    }

    /// Type-check `args` against `params`, reporting a mismatch as
    /// `Function {name} requires {types} as argument {N}{ (name)}, got {Type}`.
    /// A guard to call before trusting argument types in a native body. Arity is
    /// assumed already validated -- the VM checks a native's `Arity` before its body
    /// runs -- so only the present arguments' types are checked.
    pub fn check_args(&self, args: &[Value], params: &[Param]) -> Result<(), FrostError> {
        for (i, param) in params.iter().enumerate() {
            let Some(arg) = args.get(i) else { break };
            if !param.accepts(arg) {
                let position = match param.name {
                    Some(label) => format!("argument {} ({label})", i + 1),
                    None => format!("argument {}", i + 1),
                };
                return Err(FrostError::new(format!(
                    "Function {} requires {} as {position}, got {}",
                    self.name,
                    param.expected(),
                    arg.type_name(),
                )));
            }
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn arity(&self) -> Arity {
        self.arity
    }
}

impl Value {
    /// Build a Frost function value from a Rust closure.
    ///
    /// When called from Frost, `function` runs with a [`NativeCtx`] (for calling back
    /// into the interpreter) and the call's arguments; whatever it returns becomes the
    /// result. `arity` declares how many arguments the function accepts -- calling it
    /// with the wrong number produces a Frost error. The arguments are yours to consume
    /// -- steal one with [`Value::take`] rather than cloning (see [`NativeFn`]).
    ///
    /// Reach for this when the function validates its own arguments: it accepts
    /// flexible types, or which types are valid depends on more than one argument at
    /// once. To have argument types checked for you, use [`Value::checked_native`].
    pub fn native<F>(name: &'static str, arity: Arity, function: F) -> Value
    where
        F: NativeFn,
    {
        Value::NativeFunction(Arc::new(NativeFunction::new(name, arity, function)))
    }

    /// Build a Frost function value whose arguments are type-checked for you.
    ///
    /// `params` describes each parameter's accepted types and whether it is optional
    /// (see [`Param`]). The function's arity is derived from it, and every argument is
    /// validated before `body` runs -- so `body` can assume its arguments already match
    /// the spec. A bad argument raises a Frost error with an appropriate error message,
    /// before `body` is ever executed.
    ///
    /// This is the usual way to expose a Rust function to Frost. Use [`Value::native`]
    /// when the valid types can't be described per parameter and the function must
    /// check them itself.
    pub fn checked_native<F, const N: usize>(
        name: &'static str,
        params: [Param; N],
        body: F,
    ) -> Value
    where
        F: NativeFn,
    {
        Value::NativeFunction(Arc::new(NativeFunction::checked(name, params, body)))
    }
}

#[derive(Debug)]
pub struct Closure {
    function: Arc<CompiledFunction>,

    // If nonempty, these are the captured values, occupying slots 0..n.
    captures: Vec<Value>,
}

impl Closure {
    pub fn inner_fn(&self) -> &CompiledFunction {
        &self.function
    }

    pub fn inner_fn_arc(&self) -> Arc<CompiledFunction> {
        self.function.clone()
    }
}

// ============================================================
// Calling Convention
// ============================================================

// In all below stack diagrams, left == deeper
// PushInt(1),PushInt(2),PushInt(3) results in:
// stack: 1, 2, 3
//
// VM calling convention:
//
// f: function
// an: args
//
// A function is looked up (whether by computing a subexpression or direct name lookup).
// Then its arguments are evaluated in lexical order.
// Then:
//
// Call(3)
// stack: f, a1, a2, a3
//
// VM function path:
// The VM peeks down (top - 3) and grabs the function,
// and pushed a new StackFrame whose base_idx is a1.
// The VM starts interpreting the target function.
// The function has a prelude that moves from the stack to slots corresponding to params.
// After it's finished, the function leaves exactly one value on the stack,
// and the VM jumps back to the next instruction after the Call
//
// Native function path:
// The VM peeks down (top - 3) and grabs the function,
// the args are _moved_ to the first available NativeArgFrame,
// and the native function is invoked, given a &mut[Value] to the NativeArgFrame.
// A call increments native_arg_frames.next, and decrements it upon returning.

// ============================================================
// Vm Methods
// ============================================================

impl ProgramResult {
    /// Get the value of the tail expression of a script.
    /// Often `null`.
    pub fn tail(&self) -> &Value {
        self.0.stack.last().unwrap_or(&Value::Null)
    }

    /// Look up the value of an exported binding.
    /// None indicates the name was not exported.
    pub fn get_export<'a>(&'a self, name: &str) -> Option<&'a Value> {
        let base = self.0.base_frame();
        // There cannot be duplicate names that are both exported.
        // Exports can only be defined at the top-level in an `export def`,
        // and the compiler must reject any duplicate bindings.
        base.this_fn
            .name_table
            .iter()
            .zip(&base.local_slots)
            .find(|(entry, _)| entry.exported && entry.name == name)
            .map(|(_, slot)| {
                slot.as_ref()
                    .expect("IMPOSSIBLE: exported slot unfilled after execution")
            })
    }

    /// Get all values exported by the script.
    pub fn exports(&self) -> impl Iterator<Item = (&str, &Value)> {
        let base = self.0.base_frame();
        base.this_fn
            .name_table
            .iter()
            .zip(&base.local_slots)
            .filter(|(entry, _)| entry.exported)
            .map(|(entry, slot)| {
                (
                    entry.name.as_str(),
                    slot.as_ref()
                        .expect("IMPOSSIBLE: exported slot unfilled after execution"),
                )
            })
    }

}

/// The surface common to both outcomes of a run (success or failure): each still owns the
/// warm [`Vm`], so either can be metered or recycled.
pub trait RunOutcome {
    /// The number of function calls the program made -- the fuel it consumed.
    /// Reported whether or not a [`fuel`](VmRuntimeConfiguration::fuel) limit was set.
    fn fuel_consumed(&self) -> usize;

    /// Recycle the warm [`Vm`] to run another [`Closure`], reusing its internal allocations
    /// rather than building a fresh one.
    fn reset(self, closure: Arc<Closure>) -> Vm;
}

impl RunOutcome for ProgramResult {
    fn fuel_consumed(&self) -> usize {
        self.0.fuel_used
    }

    fn reset(self, closure: Arc<Closure>) -> Vm {
        self.0.rearm(closure)
    }
}

/// A failed run. Holds the raised [`FrostError`] and the warm [`Vm`], which -- unlike a
/// [`ProgramResult`] -- exposes no program state (`tail`/`exports`), since a failed run
/// leaves the Vm indeterminate. Recover the error with [`into_error`](Self::into_error),
/// or recycle the Vm via [`RunOutcome::reset`].
pub struct RunError {
    vm: Vm,
    error: FrostError,
}

impl RunError {
    /// The error that ended the run.
    pub fn error(&self) -> &FrostError {
        &self.error
    }

    /// Take the error, discarding the Vm. The cheap exit for a host that will not reuse it.
    pub fn into_error(self) -> FrostError {
        self.error
    }
}

impl RunOutcome for RunError {
    fn fuel_consumed(&self) -> usize {
        self.vm.fuel_used
    }

    fn reset(self, closure: Arc<Closure>) -> Vm {
        self.vm.rearm(closure)
    }
}

impl std::fmt::Debug for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Elide the (large, indeterminate) Vm from `{:?}`/`unwrap` output.
        f.debug_struct("RunError").field("error", &self.error).finish_non_exhaustive()
    }
}

impl Vm {
    /// A default-configured [`VmFactory`].
    ///
    /// The closure carries its already-bound captures (see [`CompiledFunction::close`]).
    /// Any referenced [`CompiledFunction`] must be well-formed, and running malformed
    /// bytecode may panic; only the Frost compiler emits guaranteed-well-formed bytecode.
    ///
    /// A Vm runs a single program; reuse a warm Vm via [`ProgramResult::reset`], or stamp
    /// out fresh identically-configured Vms by reusing one factory.
    pub fn factory() -> VmFactory {
        VmFactory::default()
    }

    /// Scrub a spent Vm back to a runnable state for `closure`, keeping its allocations.
    /// Clears the operand stack and frames (a failed run leaves both dirty), the fuel
    /// meter, and the abort latch.
    fn rearm(mut self, closure: Arc<Closure>) -> Vm {
        self.stack.clear();
        self.stack_frames.clear();
        self.top_level = closure;
        self.fuel_used = 0;
        self.abort = None;
        self
    }

    fn this_frame(&self) -> &VmFrame {
        match (self.stack_frames.last()) {
            Some(StackFrame::VmFrame(vm_frame)) => vm_frame,
            Some(StackFrame::NativeFrame) => panic!("IMPOSSIBLE: current Vm frame is native frame"),
            None => panic!("IMPOSSIBLE: Vm has no frame"),
        }
    }

    fn base_frame(&self) -> &VmFrame {
        match (self.stack_frames.first()) {
            Some(StackFrame::VmFrame(vm_frame)) => vm_frame,
            Some(StackFrame::NativeFrame) => panic!("IMPOSSIBLE: base Vm frame is native frame"),
            None => panic!("IMPOSSIBLE: Vm has no frame"),
        }
    }

    fn this_frame_mut(&mut self) -> &mut VmFrame {
        match (self.stack_frames.last_mut()) {
            Some(StackFrame::VmFrame(vm_frame)) => vm_frame,
            Some(StackFrame::NativeFrame) => panic!("IMPOSSIBLE: current Vm frame is native frame"),
            None => panic!("IMPOSSIBLE: Vm has no frame"),
        }
    }

    fn execute_function(&mut self) -> Result<(), FrostError> {
        let mut pc: usize = 0;
        let floor = self.stack_frames.len(); // 1 for run(), or more for native -> Vm re-entrancy

        loop {
            while let Some(&op) = self.this_frame().this_fn.code.get(pc) {
                match op {
                    Bytecode::PushNull => {
                        self.stack.push(Value::Null);
                    }
                    Bytecode::PushTrue => {
                        self.stack.push(Value::Bool(true));
                    }
                    Bytecode::PushFalse => {
                        self.stack.push(Value::Bool(false));
                    }
                    Bytecode::PushInt(i) => self.stack.push(Value::Int(i)),
                    Bytecode::PushFloat(f) => self.stack.push(Value::Float(f)),
                    Bytecode::Pop => {
                        self.stack_pop();
                    }
                    Bytecode::Dup => self
                        .stack
                        .push(self.stack.last().expect("FROST STACK UNDERFLOW").clone()),
                    Bytecode::PeekDown(idx) => {
                        self.stack.push(self.stack[self.stack.len() - idx].clone());
                    }
                    Bytecode::DropBelow(idx) => {
                        self.stack.remove(self.stack.len() - (1 + idx));
                    }
                    Bytecode::DefLocal(idx) => {
                        self.this_frame_mut().local_slots[idx] = Some(self.stack_pop());
                    }
                    Bytecode::LoadLocal(idx) => {
                        self.stack.push(
                            self.this_frame().local_slots[idx]
                                .as_ref()
                                .expect("IMPOSSIBLE: local value is undefined")
                                .clone(),
                        );
                    }
                    Bytecode::LoadConst(idx) => self
                        .stack
                        .push(self.this_frame().this_fn.constants[idx].clone()),
                    Bytecode::LoadGlobal(idx) => self.stack.push(self.globals.get(idx).clone()),
                    // Arithmetic delegates to the operators defined on `Value`;
                    // a type error or division by zero surfaces as an `Err` that `?` propagates straight out of this activation (see `unwind_frames`).
                    Bytecode::Add => self.do_add()?,
                    Bytecode::Subtract => self.binary_op(Value::subtract)?,
                    Bytecode::Multiply => self.binary_op(Value::multiply)?,
                    Bytecode::Divide => self.binary_op(Value::divide)?,
                    Bytecode::Modulus => self.binary_op(Value::modulus)?,
                    // Equality is infallible (`Value: Eq`).
                    // Ordering delegates to `Value::compare`, where an unorderable pair is a type error.
                    Bytecode::CompareEqual => self.binary_op(|l, r| Ok(Value::Bool(l == r)))?,
                    Bytecode::CompareNotEqual => self.binary_op(|l, r| Ok(Value::Bool(l != r)))?,
                    Bytecode::CompareLessThan => {
                        self.binary_op(|l, r| Ok(Value::Bool(l.compare(r)?.is_lt())))?
                    }
                    Bytecode::CompareLessThanOrEqual => {
                        self.binary_op(|l, r| Ok(Value::Bool(l.compare(r)?.is_le())))?
                    }
                    Bytecode::CompareGreaterThan => {
                        self.binary_op(|l, r| Ok(Value::Bool(l.compare(r)?.is_gt())))?
                    }
                    Bytecode::CompareGreaterThanOrEqual => {
                        self.binary_op(|l, r| Ok(Value::Bool(l.compare(r)?.is_ge())))?
                    }
                    Bytecode::LogicalNot => {
                        let operand = self.stack_pop();
                        let result = Value::from(!operand.is_truthy());
                        self.stack.push(result);
                    }
                    Bytecode::Negate => {
                        let operand = self.stack_pop();
                        let result = match operand {
                            Value::Int(i) => Value::from(i.wrapping_neg()),
                            // Unwrap is safe here because I'm just negating a float that's already
                            // proven not to be NaN or Inf
                            Value::Float(f) => Value::Float(FrostFloat::new(-f.get()).unwrap()),
                            _ => {
                                return Err(FrostError::new(format!(
                                    "Cannot negate value of type {}",
                                    operand.type_name()
                                )));
                            }
                        };
                        self.stack.push(result);
                    }
                    // Jump(n) (and conditional variants) skip n instructions.
                    // The `pc += 1` is intended to still be hit.
                    // Jump(0) is just a funny way to spell "Nop".
                    Bytecode::Jump(n) => {
                        pc += n;
                    }
                    Bytecode::JumpIfTrue(n) => {
                        let operand = self.stack.last().expect("FROST STACK UNDERFLOW");
                        if operand.is_truthy() {
                            pc += n;
                        }
                    }
                    Bytecode::JumpIfFalse(n) => {
                        let operand = self.stack.last().expect("FROST STACK UNDERFLOW");
                        if !operand.is_truthy() {
                            pc += n;
                        }
                    }
                    Bytecode::Call(argc) => {
                        self.expend_fuel()?;
                        let base = self.stack.len() - (argc + 1);
                        let function = self.stack[base].clone();

                        // Errors `?` straight out of `execute_function`;
                        // the boundary that entered this activation (`invoke` or `run`) truncates the abandoned frames and operands.
                        match function {
                            Value::NativeFunction(native_fn) => {
                                self.native_call(&native_fn, argc)?
                            }
                            Value::Closure(closure) => {
                                Self::check_arity(
                                    closure.function.arity,
                                    argc,
                                    &closure.function.name,
                                )?;
                                // The next loop iteration enters the closure, whose prelude
                                // consumes the args on the stack and leaves one return value.
                                self.push_closure_frame(&closure, base, NonZeroUsize::new(pc + 1))?;
                                pc = 0;
                                continue;
                            }
                            _ => return Err(Self::not_callable(&function)),
                        }
                    }
                    Bytecode::TailCall(argc) => {
                        match self.tail_call(argc, NonZeroUsize::new(pc + 1))? {
                            TailFlow::Reenter => {
                                pc = 0;
                                continue;
                            }
                            TailFlow::FellThrough => {}
                        }
                    }
                    // Spread the args array on top, then tail-call the function beneath it.
                    // Hand-rolled spreaders (`call`, future combinators) pass user values
                    // here, so a non-Array args operand is a recoverable error -- the
                    // callee being non-callable is likewise caught by `tail_call`.
                    Bytecode::DynTailCall => {
                        let args = self.stack.last().expect("FROST STACK UNDERFLOW");
                        if !args.is_array() {
                            return Err(FrostError::new(format!(
                                "Spread call expects an Array of arguments, but got {}",
                                args.type_name()
                            )));
                        }
                        let argc = self.explode_array();
                        match self.tail_call(argc, NonZeroUsize::new(pc + 1))? {
                            TailFlow::Reenter => {
                                pc = 0;
                                continue;
                            }
                            TailFlow::FellThrough => {}
                        }
                    }
                    Bytecode::ExplodeArray => {
                        self.explode_array();
                    }
                    Bytecode::CreateClosure {
                        num_captures,
                        function,
                    } => {
                        let function =
                            self.this_frame().this_fn.child_fns[function as usize].clone();
                        let captures = self
                            .stack
                            .split_off(self.stack.len() - num_captures as usize);

                        self.stack
                            .push(Value::Closure(Arc::new(Closure { function, captures })))
                    }
                    Bytecode::MakeArray(num_elems) => {
                        let arr: FrostArray =
                            self.stack.split_off(self.stack.len() - num_elems).into();
                        self.stack.push(arr.into());
                    }
                    Bytecode::MakeMap(num_pairs) => {
                        let split_point = self.stack.len() - 2 * num_pairs;
                        let flat_pairs = self.stack.split_off(split_point);

                        let map: FrostMap = flat_pairs
                            .into_iter()
                            .tuples()
                            .map(|(k, v)| Ok((MapKey::try_from(k)?, v)))
                            .collect::<Result<_, FrostError>>()?;

                        self.stack.push(map.into());
                    }
                    Bytecode::SoftIndexStructure => {
                        let index = self.stack_pop();
                        let structure = self.stack_pop();

                        let result = match (&structure, &index) {
                            (Value::Array(arr), Value::Int(i)) => {
                                arr.frost_get(*i).unwrap_or(&Value::Null)
                            }
                            (Value::Array(_), _) => {
                                return Err(FrostError::new(format!(
                                    "Cannot index Array with value of type {}",
                                    index.type_name()
                                )));
                            }
                            (Value::Map(map), _) => {
                                let key = MapKey::try_from(index)?;
                                map.get(&key).unwrap_or(&Value::Null)
                            }
                            _ => {
                                return Err(FrostError::new(format!(
                                    "Cannot index value of type {}",
                                    structure.type_name()
                                )));
                            }
                        }
                        .clone();

                        self.stack.push(result);
                    }
                    Bytecode::HardIndexMap(const_pool_idx_of_key) => {
                        let val = self.stack_pop();
                        let Value::Map(map) = val else {
                            return Err(FrostError::new(format!(
                                "Cannot index value of type {}",
                                val.type_name()
                            )));
                        };
                        let key = &self.this_frame().this_fn.constants[const_pool_idx_of_key];

                        // The key constant is compiler-guaranteed to be a String (it is the
                        // field name from `foo.bar`); any other type is broken bytecode.
                        let Value::String(s) = key else {
                            panic!(
                                "IMPOSSIBLE: HardIndexMap key constant must be a String, but was {}",
                                key.type_name()
                            );
                        };
                        let key = MapKey::String(s.clone());

                        match map.get(&key) {
                            Some(result) => self.stack.push(result.clone()),
                            // TODO: improve error message with "did you mean ...?" hint
                            // (Error message sucks for now, and that's ok for now)
                            None => {
                                return Err(FrostError::new(format!(
                                    "Map has no value at key '{}'",
                                    String::from_utf8_lossy(s)
                                )));
                            }
                        }
                    }
                    Bytecode::TypeTest(tc) => {
                        let operand = self.stack_pop();
                        self.stack.push(operand.fits_category(tc).into());
                    }
                    Bytecode::ProduceError => match self.stack_pop() {
                        // TODO: enhance FrostError to support attaching an arbitrary Value
                        // (with an optimization for the most-common case of a UTF-8 string)
                        Value::String(s) => {
                            return Err(FrostError::new(String::from_utf8_lossy(&s)));
                        }
                        // This error message is kinda lame, but can be removed after
                        // addressing the above TODO
                        _ => return Err(FrostError::new("An unknown error occurred")),
                    },
                };
                pc += 1;
            }

            if self.stack_frames.len() == floor {
                // We're done!
                // Return back to either `run()` or native function re-entrancy handling
                return Ok(());
            }

            // Vm function return path

            let StackFrame::VmFrame(frame) = self
                .stack_frames
                .pop()
                .expect("IMPOSSIBLE: Vm has no stack frame")
            else {
                panic!("IMPOSSIBLE: function execution completed through native frame");
            };

            pc = frame
                .return_address
                .expect("IMPOSSIBLE: Callee lacks return address")
                .get();
        }
    }

    /// Run the top-level closure with no arguments.
    /// Equivalent to [`run_with_args`](Self::run_with_args) with an empty list.
    // Both variants carry the warm Vm by design (that is the whole point), so the
    // `Result` is large regardless of the Err -- boxing would only add an allocation.
    #[allow(clippy::result_large_err)]
    pub fn run(self) -> Result<ProgramResult, RunError> {
        self.run_with_args(std::iter::empty())
    }

    /// Run the top-level closure, passing `args` as its call arguments.
    ///
    /// Success yields a [`ProgramResult`] (tail value + exports); failure a [`RunError`].
    /// Either outcome still owns the warm Vm (recyclable via [`RunOutcome::reset`]); an
    /// arity mismatch is a recoverable failure.
    #[allow(clippy::result_large_err)]
    pub fn run_with_args(
        mut self,
        args: impl IntoIterator<Item = Value>,
    ) -> Result<ProgramResult, RunError> {
        let closure = self.top_level.clone();
        self.stack.push(Value::Closure(closure.clone()));
        self.stack.extend(args);
        let argc = self.stack.len() - 1;
        if let Err(error) = Self::check_arity(closure.function.arity, argc, &closure.function.name) {
            return Err(RunError { vm: self, error });
        }
        // The top-level frame is seated at depth 0, so this never trips the depth cap;
        // it does not consume fuel (the program has not made a call yet).
        if let Err(error) = self.push_closure_frame(&closure, 0, None) {
            return Err(RunError { vm: self, error });
        }

        match self.execute_function() {
            Ok(()) => {
                #[cfg(debug_assertions)]
                self.debug_verify_terminal_state();
                Ok(ProgramResult(self))
            }
            // No `NativeFrame` exists above the top level, so the error has nowhere to be
            // caught: accumulate the backtrace across every remaining frame and surface it.
            // The (now spent, dirty) Vm rides along in the `RunError` for reuse via `reset`.
            Err(err) => {
                let error = self.unwind_frames(0, err);
                Err(RunError { vm: self, error })
            }
        }
    }

    /// Debug-only sanity check that a successful run unwound to a valid terminal state.
    /// A malformed program that trips one of these has violated an invariant the public
    /// API (`tail`, `exports`) then relies on, so we catch it at the boundary in debug builds.
    #[cfg(debug_assertions)]
    fn debug_verify_terminal_state(&self) {
        // The operand stack holds at most the tail value -- nothing for a program of
        // only `def`/`export def` statements (whose tail is null).
        debug_assert!(
            self.stack.len() <= 1,
            "a completed program must leave at most one value on the stack, found {}",
            self.stack.len()
        );
        // Exactly the top-level frame remains -- every call has returned, and the
        // bottom frame is preserved rather than popped.
        debug_assert_eq!(
            self.stack_frames.len(),
            1,
            "a completed program must leave exactly the top-level frame, found {}",
            self.stack_frames.len()
        );
        // That frame is the pristine top-level closure's frame (base_frame panics if
        // it is a native frame): based at 0, no caller to return to, right function.
        let base = self.base_frame();
        debug_assert_eq!(
            base.base_idx, 0,
            "the top-level frame must be based at stack index 0"
        );
        debug_assert!(
            base.return_address.is_none(),
            "the top-level frame must have no return address"
        );
        debug_assert!(
            Arc::ptr_eq(&base.this_fn, &self.top_level.function),
            "the top-level frame must belong to the top-level closure"
        );
        // Every exported binding was assigned -- the invariant `exports`/`get_export` trust.
        for (entry, slot) in base.this_fn.name_table.iter().zip(&base.local_slots) {
            debug_assert!(
                !entry.exported || slot.is_some(),
                "exported binding `{}` was left unfilled after execution",
                entry.name
            );
        }
    }

    /// Pop the top of the operand stack.
    /// A missing operand is a compiler/bytecode bug, not a recoverable error, so underflow panics.
    fn stack_pop(&mut self) -> Value {
        self.stack.pop().expect("FROST STACK UNDERFLOW")
    }

    /// Pop the top two operands (rhs on top, lhs below) and push `op(lhs, rhs)`.
    /// Any operator error `?`-propagates out of the current activation;
    /// the consumed operands are simply dropped, since the unwind path truncates the operand stack back to the boundary floor regardless of its exact height.
    fn binary_op(
        &mut self,
        op: impl FnOnce(&Value, &Value) -> Result<Value, FrostError>,
    ) -> Result<(), FrostError> {
        let rhs = self.stack_pop();
        let lhs = self.stack_pop();
        self.stack.push(op(&lhs, &rhs)?);
        Ok(())
    }

    /// The `Add` opcode.
    /// `Array + Array` and `Map + Map` are the only overloads where the borrowing `Value::add` would clone every element/entry,
    /// so for those we *steal* the operands' storage -- reusing it in place when the `Arc` is uniquely owned (a frequent case for stack temporaries), cloning only when shared.
    /// Numeric addition, string concat, and every type error have nothing worth stealing and fall back to the shared `binary_op` path.
    fn do_add(&mut self) -> Result<(), FrostError> {
        let n = self.stack.len();
        let both_structural = n >= 2
            && matches!(
                (&self.stack[n - 2], &self.stack[n - 1]),
                (Value::Array(_), Value::Array(_)) | (Value::Map(_), Value::Map(_))
            );

        if !both_structural {
            return self.binary_op(Value::add);
        }

        let rhs = self.stack_pop();
        let lhs = self.stack_pop();
        let combined = match (lhs, rhs) {
            (Value::Array(lhs), Value::Array(rhs)) => {
                let mut elems = lhs.to_owned(); // steals lhs's Vec when uniquely owned
                elems.extend(rhs.to_owned()); // steals rhs's elements when uniquely owned
                Value::Array(elems.into())
            }
            (Value::Map(lhs), Value::Map(rhs)) => {
                let mut entries = lhs.to_owned();
                entries.extend(rhs.to_owned()); // on key collision rhs wins, matching `+`
                Value::Map(entries.into())
            }
            // `both_structural` guarantees one of the two arms above.
            _ => unreachable!("add: both_structural implies Array+Array or Map+Map"),
        };
        self.stack.push(combined);
        Ok(())
    }

    /// Append the names of the `VmFrame`s in `stack_frames[floor..]` to `err`'s backtrace
    /// -- innermost (top of the frame stack) first -- then discard those frames.
    ///
    /// This is the only place a Frost frame's name reaches the backtrace: `?` propagation has no hook,
    /// so the trace is built here as the abandoned frames are dropped.
    /// Called at each native boundary that catches an unwinding error ([`NativeCtx::invoke`]) and at the top-level terminus ([`Vm::run`]).
    /// `NativeFrame` markers carry no name; a native's own name is recorded by [`Vm::run_native`] instead.
    fn unwind_frames(&mut self, floor: usize, mut err: FrostError) -> FrostError {
        let names: Vec<String> = self
            .stack_frames
            .drain(floor..)
            .rev()
            .filter_map(|frame| match frame {
                StackFrame::VmFrame(vm_frame) => Some(vm_frame.this_fn.name.clone()),
                StackFrame::NativeFrame => None,
            })
            .collect();
        // During an abort the latch is the authoritative fatal error:
        // append the frame names to it and hand back a copy,
        // so a native above that swallows this copy can't drop the accumulated trace.
        // Otherwise grow the ordinary propagating error as it unwinds.
        match self.abort.as_mut() {
            Some(abort) => {
                abort.backtrace.extend(names);
                abort.clone()
            }
            None => {
                err.backtrace.extend(names);
                err
            }
        }
    }

    /// Returns an arity-mismatch error if `argc` does not satisfy `arity`, or `Ok(())` if it does.
    /// `name` is the called function's name, for the error message.
    fn check_arity(arity: Arity, argc: usize, name: &str) -> Result<(), FrostError> {
        let ok = match arity {
            Arity::Exact(n) => argc == n,
            Arity::Between(lo, hi) => (lo..=hi).contains(&argc),
            Arity::AtLeast(n) => argc >= n,
        };
        if ok {
            return Ok(());
        }
        Err(FrostError::new(match arity {
            Arity::Exact(n) => {
                format!("Function {name} expects {n} arguments, but was called with {argc}")
            }
            Arity::Between(lo, hi) => {
                format!(
                    "Function {name} expects between {lo} and {hi} arguments, but was called with {argc}"
                )
            }
            Arity::AtLeast(n) => {
                format!(
                    "Function {name} expects at least {n} arguments, but was called with {argc}"
                )
            }
        }))
    }

    /// The error produced when a non-callable value is called.
    fn not_callable(value: &Value) -> FrostError {
        FrostError::new(format!(
            "Attempt to call non-function value of type {}",
            value.type_name()
        ))
    }

    // ----- Resource limits (`VmRuntimeConfiguration`) -----
    // Centralized guards + error messages, called from the several call/frame-push sites.

    /// Count one function call against the fuel budget, failing if it is exhausted.
    /// Called at every call site (`Call`, tail calls, and native re-entry via `invoke`).
    /// The meter increments even when unmetered, so the consumed count stays reportable.
    fn expend_fuel(&mut self) -> Result<(), FrostError> {
        self.fuel_used = self.fuel_used.saturating_add(1);
        if let Some(budget) = self.config.fuel
            && self.fuel_used > budget.get()
        {
            // Exhaustion is unrecoverable, not an ordinary error -- latch it.
            return Err(self.abort_with(Self::fuel_exhausted(budget.get())));
        }
        Ok(())
    }

    /// Latch `err` into the unrecoverable-error channel and hand it back to propagate.
    /// Once latched, `run_native`/`invoke` refuse to let any native resume Frost code.
    fn abort_with(&mut self, err: FrostError) -> FrostError {
        self.abort = Some(err.clone());
        err
    }

    /// The latched abort error if the run is fatally aborting, else `None`.
    /// Returns a clone: the slot stays latched so every enclosing native re-asserts it.
    fn abort_error(&self) -> Option<FrostError> {
        self.abort.clone()
    }

    /// Fail if pushing another frame would exceed the configured call-depth limit.
    /// Called by the frame-push primitives; tail-call frame reuse is net-neutral so it
    /// never trips, and the top-level frame (depth 0 at entry) is always admitted.
    fn check_call_depth(&self) -> Result<(), FrostError> {
        match self.config.max_call_depth {
            Some(max) if self.stack_frames.len() >= max.get() => {
                Err(Self::call_depth_exceeded(max.get()))
            }
            _ => Ok(()),
        }
    }

    fn fuel_exhausted(limit: usize) -> FrostError {
        FrostError::new(format!(
            "Execution exceeded its fuel limit of {limit} function calls"
        ))
    }

    fn call_depth_exceeded(limit: usize) -> FrostError {
        FrostError::new(format!(
            "Execution exceeded the maximum call depth of {limit}"
        ))
    }

    /// Push a [VmFrame] to enter `closure`: captures seat into slots `0..n`, then variadic args are collapsed into a trailing rest array.
    /// `base` is the frame base (the closure's slot on the stack);
    /// `return_address` is where the callee returns to.
    /// Arity must already be checked.
    fn push_closure_frame(
        &mut self,
        closure: &Closure,
        base: usize,
        return_address: Option<NonZeroUsize>,
    ) -> Result<(), FrostError> {
        self.check_call_depth()?;

        // TODO: perhaps pool local slot vecs to reuse allocations, like with native arg vecs
        let mut local_slots = vec![None; closure.function.name_table.len()];
        for (i, capture) in closure.captures.iter().enumerate() {
            local_slots[i] = Some(capture.clone());
        }

        self.stack_frames.push(StackFrame::VmFrame(VmFrame {
            base_idx: base,
            local_slots,
            return_address,
            this_fn: closure.function.clone(),
        }));

        // The incoming argument count: everything on the operand stack above the
        // function value at `base`, captured before the rearrangement below.
        let argc = self.stack.len() - (base + 1);

        match closure.function.arity {
            // Surplus args beyond the fixed params collapse into the rest array.
            Arity::AtLeast(fixed_argc) => {
                let varargs = self.stack.split_off(base + 1 + fixed_argc);
                self.stack.push(Value::Array(varargs.into()));
            }
            // A `Between` closure is always hand-rolled bytecode. Hand its prelude the
            // actual arg count on top of the args, so it can tell an omitted optional
            // from one explicitly passed as null and seat its slots accordingly.
            Arity::Between(..) => {
                self.stack.push(Value::Int(argc as i64));
            }
            Arity::Exact(_) => {}
        }
        Ok(())
    }

    /// Pop the top operand (which must be an Array) and push its elements in order,
    /// returning the count. Backs `ExplodeArray` and the arg-spread of `DynTailCall`.
    fn explode_array(&mut self) -> usize {
        let Value::Array(arr) = self.stack_pop() else {
            panic!("explode: operand not Array");
        };
        let before = self.stack.len();
        match arr.try_extract() {
            Ok(vec) => self.stack.extend(vec),
            Err(arr) => self.stack.extend(arr.iter().cloned()),
        }
        self.stack.len() - before
    }

    /// Shared dispatch for `TailCall` and `DynTailCall`: with the callee and its
    /// `argc` operands on top of the stack, run a native inline or set up a closure
    /// frame for tail-call reuse. The returned [`TailFlow`] tells the caller whether
    /// to re-enter the loop in the callee's frame (closure) or fall through (native).
    ///
    /// `return_address` is consulted only by the bottom-frame guard: the top-level
    /// frame backs `exports`, so a tail call from it is degraded to a normal call
    /// that returns into the preserved frame rather than eliding it. (Re-entrant
    /// `invoke` activations always run with >= 2 frames, so TCO holds everywhere else.)
    fn tail_call(
        &mut self,
        argc: usize,
        return_address: Option<NonZeroUsize>,
    ) -> Result<TailFlow, FrostError> {
        self.expend_fuel()?;
        let base = self.stack.len() - (argc + 1);
        let function = self.stack[base].clone();

        match function {
            // A native callee adds no VM frame: run it inline like a plain Call.
            // Its result is left on the stack; in tail position the enclosing
            // function returns it via the normal end-of-code path next turn.
            Value::NativeFunction(native_fn) => {
                self.native_call(&native_fn, argc)?;
                Ok(TailFlow::FellThrough)
            }
            Value::Closure(closure) => {
                // Check arity before popping the caller frame, so an arity error
                // does not destroy the frame the error path still needs.
                Self::check_arity(closure.function.arity, argc, &closure.function.name)?;

                // A tail call reuses the current frame -- except the bottom frame,
                // which is preserved (see the doc comment).
                if self.stack_frames.len() == 1 {
                    self.push_closure_frame(&closure, base, return_address)?;
                } else {
                    let StackFrame::VmFrame(gone_frame) = self
                        .stack_frames
                        .pop()
                        .expect("IMPOSSIBLE: Vm has no frame")
                    else {
                        panic!("IMPOSSIBLE: tail call in native frame");
                    };

                    // Reuse the popped frame's slot, inheriting its base and return
                    // address so the callee returns to the original caller.
                    self.push_closure_frame(
                        &closure,
                        gone_frame.base_idx,
                        gone_frame.return_address,
                    )?;
                }
                Ok(TailFlow::Reenter)
            }
            _ => Err(Self::not_callable(&function)),
        }
    }

    fn native_call(&mut self, function: &NativeFunction, argc: usize) -> Result<(), FrostError> {
        let mut buf = self.native_arg_pool.pop().unwrap_or_default();
        buf.extend(self.stack.drain((self.stack.len() - argc)..));
        self.stack_pop(); // pop the function value off the stack

        let result = self.run_native(function, buf)?;
        self.stack.push(result);
        Ok(())
    }

    /// Invoke `native` with its args already collected in `buf`, then recycle `buf`.
    /// Checks arity, brackets the call with a `NativeFrame` marker, and returns the native's result.
    /// `buf` is reclaimed to the pool on every path.
    fn run_native(&mut self, native: &NativeFunction, mut buf: Vec<Value>) -> FrostResult {
        let reclaim = |vm: &mut Self, mut buf: Vec<Value>| {
            buf.clear();
            vm.native_arg_pool.push(buf);
        };

        if let Err(err) = Self::check_arity(native.arity, buf.len(), native.name) {
            reclaim(self, buf);
            return Err(err);
        }
        // The `NativeFrame` we are about to push is the Rust-stack growth vector.
        if let Err(err) = self.check_call_depth() {
            reclaim(self, buf);
            return Err(err);
        }

        self.stack_frames.push(StackFrame::NativeFrame);
        let result = native.function.invoke(
            NativeCtx {
                vm: self,
                function: native,
            },
            &mut buf,
        );
        reclaim(self, buf);

        let popped = self.stack_frames.pop();
        debug_assert_matches!(
            popped,
            Some(StackFrame::NativeFrame),
            "run_native must pop the NativeFrame it pushed"
        );

        // Unrecoverable abort (e.g. fuel exhaustion) is uncatchable: even if this native
        // swallowed the error and returned `Ok`, override its result. Append this native's
        // frame to the latched fatal error and re-assert it, so the host still sees the
        // full trace to wherever the run aborted.
        if let Some(abort) = self.abort.as_mut() {
            abort.backtrace.push(native.name.to_string());
            return Err(abort.clone());
        }

        // A native that ran and failed contributes its own name to the backtrace.
        // Its only call-stack presence is a nameless `NativeFrame` marker,
        // so the frame-walk in `unwind_frames` cannot record it -- do it here.
        result.map_err(|err| err.with_frame(native.name))
    }
}
