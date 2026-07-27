#![allow(unused)]

mod bytecode;
mod function;
mod globals;
mod import;
mod native;
mod outcome;
mod params;
mod serialize;

pub use bytecode::Bytecode;
pub use function::{Arity, Closure, CompiledFunction, MissingCaptures, NameEntry, TrustedProgram};
pub use globals::GLOBAL_NAMES;
pub use import::{
    Extension, ExtensionError, HostComponent, HostComponentError, ImportCtx, ImportResolver,
    Importer, ImporterBuilder, InvalidComponentName, ModuleId, Stdlib, StdlibModule,
};
pub use native::{NativeCtx, NativeFn, NativeFunction};
pub use outcome::{ProgramResult, RunError, RunOutcome};
pub use params::{InvalidParams, Param, Params};
pub use serialize::FormatVersion;

use std::{collections::BTreeMap, debug_assert_matches, num::NonZeroUsize, sync::Arc};

use itertools::Itertools;

use crate::{FrostArray, FrostError, FrostFloat, FrostMap, FrostResult, MapKey, Value};

use globals::GlobalSet;

// White-box tests for native-arg-pool recycling (see the module doc);
// as a child module it reaches this module's private `Vm` internals.
#[cfg(test)]
mod arg_pool_tests;

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

    // The module importer backing the `Import` opcode.
    // Fixed at build time; persists across `reset`.
    importer: Arc<Importer>,

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

    // Identity of the module this Vm runs, as assigned by the resolver that loaded it.
    // `None` for a top-level script the host did not identify.
    module_id: Option<ModuleId>,

    // How many imports deep this Vm is: 0 for a top-level script, 1 for a module it imported.
    // Checked against `config.max_import_depth` before importing.
    import_depth: usize,
}

/// Runtime resource limits for a [`Vm`]. The [`Default`] imposes no limits.
///
/// Each bounds how much work a program may do,
/// so that a mistake in a script you trust becomes a recoverable [`RunError`]
/// rather than a hung or crashed process.
/// Every limit is optional, and unset means unbounded.
///
/// They bound how much a script runs, never what it can reach,
/// so they are not a security boundary:
/// that is decided by what the host grants it, on [`Importer`].
#[derive(Debug, Clone, Default)]
pub struct VmRuntimeConfiguration {
    /// Maximum call-stack depth (number of frames) before execution fails with a
    /// recoverable error.
    /// Native calls and non-tail Vm calls contribute to the depth; tail calls do not.
    /// `None` leaves depth unbounded.
    pub max_call_depth: Option<NonZeroUsize>,

    /// Call budget ("fuel"): execution fails once this many function calls have been made.
    /// `None` leaves execution unmetered.
    ///
    /// Fuel is the limit that catches runaway execution: all iteration in Frost is built
    /// from function calls (higher-order functions and tail recursion, which fuel does count),
    /// so bounding calls bounds total execution.
    /// That includes unbounded tail recursion, which is depth-flat and therefore never trips
    /// [`max_call_depth`](Self::max_call_depth).
    /// A native function that loops forever without returning or re-entering the Vm is not covered.
    ///
    /// Because iteration is function calls, a Frost script makes many more calls than
    /// comparable code in a more procedural language like Lua.
    /// Consider setting this value higher than your intuition may lead you.
    pub fuel: Option<NonZeroUsize>,

    /// How deeply imports may nest:
    /// a module imported by a module imported by the top-level script is at depth 3.
    /// `None` leaves import nesting unbounded.
    ///
    /// Each level runs in its own Vm, so [`max_call_depth`](Self::max_call_depth)
    /// bounds each of them separately but not the nesting.
    /// This is the backstop for an [`ImportResolver`] that does not detect cycles.
    pub max_import_depth: Option<NonZeroUsize>,
}

/// Fixed configuration from which [`Vm`]s are built. Obtain one from [`Vm::factory`].
///
/// The final [build](VmFactory::build) is what binds the script to execute.
/// This factory is well-suited to creating several identically-configured Vms.
#[derive(Debug, Clone, Default)]
pub struct VmFactory {
    config: VmRuntimeConfiguration,
    importer: Arc<Importer>,
    // Import nesting level for the Vms this factory builds. Non-zero only for a
    // factory obtained from `Vm::child_factory`.
    import_depth: usize,
}

impl VmFactory {
    /// Set the runtime resource limits (depth, fuel) for the Vms this factory builds.
    /// See [`VmRuntimeConfiguration`] for the defaults,
    /// which are used if this method is not invoked.
    pub fn configuration(mut self, config: VmRuntimeConfiguration) -> Self {
        self.config = config;
        self
    }

    pub fn with_importer(mut self, importer: Arc<Importer>) -> Self {
        self.importer = importer;
        self
    }

    /// Build a [`Vm`] to run `closure` under this factory's configuration.
    pub fn build(&self, closure: Arc<Closure>) -> Result<Vm, FrostError> {
        Ok(Vm {
            stack: Vec::new(),
            stack_frames: Vec::new(),
            native_arg_pool: Vec::new(),
            globals: GlobalSet::defaults(),
            importer: self.importer.clone(),
            top_level: closure,
            config: self.config.clone(),
            fuel_used: 0,
            abort: None,
            module_id: None,
            import_depth: self.import_depth,
        })
    }
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

/// Control-flow outcome of a tail call, shared by `TailCall` and `DynTailCall`.
enum TailFlow {
    /// Closure callee: the loop must re-enter at the callee's frame (`pc = 0`).
    Reenter,
    /// Native callee: it ran inline, so fall through to the next instruction.
    FellThrough,
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
// and pushes a new StackFrame whose base_idx is a1.
// The VM starts interpreting the target function.
// The function has a prelude that moves from the stack to slots corresponding to params.
// After it's finished, the function leaves exactly one value on the stack,
// and the VM jumps back to the next instruction after the Call
//
// Native function path:
// The VM peeks down (top - 3) and grabs the function,
// the args are _moved_ into a buffer taken from `native_arg_pool`,
// and the native function is invoked with a &mut [Value] over that buffer.
// When the call returns, the buffer is cleared and returned to the pool.

// ============================================================
// Vm Methods
// ============================================================

impl Vm {
    /// A default-configured [`VmFactory`].
    ///
    /// A Vm runs a single program; reuse a warm Vm via [`RunOutcome::reset`], or stamp
    /// out fresh identically-configured Vms by reusing one factory.
    pub fn factory() -> VmFactory {
        VmFactory::default()
    }

    /// Identifies the script this Vm runs, for resolvers that care who is importing.
    /// Set by whatever loaded the script;
    /// a resolver stamps the id it assigned the module.
    pub fn with_module_id(mut self, id: ModuleId) -> Self {
        self.module_id = Some(id);
        self
    }

    /// A factory for Vms nested inside this one: same configuration and importer,
    /// one import level deeper.
    /// Resource counters start fresh in the child.
    pub(crate) fn child_factory(&self) -> VmFactory {
        VmFactory {
            config: self.config.clone(),
            importer: self.importer.clone(),
            import_depth: self.import_depth + 1,
        }
    }

    /// Guard the nesting level a child Vm would run at,
    /// then package what a resolver needs to build one.
    fn import_ctx(&self) -> Result<ImportCtx<'_>, FrostError> {
        if let Some(limit) = self.config.max_import_depth
            && self.import_depth + 1 > limit.get()
        {
            return Err(FrostError::from_string(format!(
                "Import depth limit of {} exceeded",
                limit.get()
            )));
        }
        Ok(ImportCtx::new(self.child_factory(), self.module_id.clone()))
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
                    Bytecode::PeekDown(idx) => {
                        let from = self
                            .stack
                            .len()
                            .checked_sub(idx + 1)
                            .expect("FROST STACK UNDERFLOW");
                        self.debug_assert_own_operand(from, "PeekDown");
                        self.stack.push(self.stack[from].clone());
                    }
                    Bytecode::DropBelow(idx) => {
                        let at = self
                            .stack
                            .len()
                            .checked_sub(idx + 1)
                            .expect("FROST STACK UNDERFLOW");
                        self.debug_assert_own_operand(at, "DropBelow");
                        self.stack.remove(at);
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
                            // Negating a float that is already non-NaN and finite
                            // cannot produce NaN or Infinity, so the unwrap cannot fail.
                            Value::Float(f) => Value::Float(FrostFloat::new(-f.get()).unwrap()),
                            _ => {
                                return Err(FrostError::from_string(format!(
                                    "Cannot negate value of type {}",
                                    operand.type_name()
                                )));
                            }
                        };
                        self.stack.push(result);
                    }
                    // `pc += n` composes with the shared `pc += 1` below:
                    // the offset counts skipped instructions, not an absolute target.
                    Bytecode::Jump(n) => {
                        pc += n;
                        self.debug_assert_jump_target(pc);
                    }
                    Bytecode::JumpIfTrue(n) => {
                        self.debug_assert_own_top("JumpIfTrue");
                        let operand = self.stack.last().expect("FROST STACK UNDERFLOW");
                        if operand.is_truthy() {
                            pc += n;
                            self.debug_assert_jump_target(pc);
                        }
                    }
                    Bytecode::JumpIfFalse(n) => {
                        self.debug_assert_own_top("JumpIfFalse");
                        let operand = self.stack.last().expect("FROST STACK UNDERFLOW");
                        if !operand.is_truthy() {
                            pc += n;
                            self.debug_assert_jump_target(pc);
                        }
                    }
                    Bytecode::Call(argc) => {
                        self.expend_fuel()?;
                        let base = self
                            .stack
                            .len()
                            .checked_sub(argc + 1)
                            .expect("FROST STACK UNDERFLOW");
                        // The callee and its arguments must be operands this frame
                        // pushed; reaching lower would call one of the caller's values.
                        self.debug_assert_own_operand(base, "Call");
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
                    // here, so a non-Array args operand is a recoverable error; the
                    // callee being non-callable is likewise caught by `tail_call`.
                    Bytecode::DynTailCall => {
                        self.debug_assert_own_top("DynTailCall");
                        let args = self.stack.last().expect("FROST STACK UNDERFLOW");
                        if !args.is_array() {
                            return Err(FrostError::from_string(format!(
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
                        let split_point = self
                            .stack
                            .len()
                            .checked_sub(num_captures as usize)
                            .expect("FROST STACK UNDERFLOW");
                        self.debug_assert_own_operand(split_point, "CreateClosure");
                        let captures = self.stack.split_off(split_point);

                        self.stack
                            .push(Value::Closure(Arc::new(Closure { function, captures })))
                    }
                    Bytecode::MakeArray(num_elems) => {
                        let split_point = self
                            .stack
                            .len()
                            .checked_sub(num_elems)
                            .expect("FROST STACK UNDERFLOW");
                        self.debug_assert_own_operand(split_point, "MakeArray");
                        let arr: FrostArray = self.stack.split_off(split_point).into();
                        self.stack.push(arr.into());
                    }
                    Bytecode::MakeMap(num_pairs) => {
                        let split_point = self
                            .stack
                            .len()
                            .checked_sub(2 * num_pairs)
                            .expect("FROST STACK UNDERFLOW");
                        self.debug_assert_own_operand(split_point, "MakeMap");
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
                                return Err(FrostError::from_string(format!(
                                    "Cannot index Array with value of type {}",
                                    index.type_name()
                                )));
                            }
                            (Value::Map(map), _) => {
                                let key = MapKey::try_from(index)?;
                                map.get(&key).unwrap_or(&Value::Null)
                            }
                            _ => {
                                return Err(FrostError::from_string(format!(
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
                            return Err(FrostError::from_string(format!(
                                "Cannot index value of type {}",
                                val.type_name()
                            )));
                        };

                        let key = &self.this_frame().this_fn.key_constants[const_pool_idx_of_key];

                        match map.get(key) {
                            Some(result) => self.stack.push(result.clone()),
                            // TODO: improve error message with a "did you mean ...?" hint
                            None => {
                                return Err(FrostError::from_string(format!(
                                    "Map has no value at key '{key}'"
                                )));
                            }
                        }
                    }
                    Bytecode::TypeTest(types) => {
                        let operand = self.stack_pop();
                        self.stack.push(operand.fits(types).into());
                    }
                    Bytecode::ProduceError => {
                        return Err(FrostError::from_value(self.stack_pop()));
                    }
                    Bytecode::Import => {
                        let spec_value = self.stack_pop();
                        let Some(bytes) = spec_value.as_byte_string() else {
                            return Err(FrostError::from_string(format!(
                                "import expects a String module spec, got {}",
                                spec_value.type_name()
                            )));
                        };
                        let spec = std::str::from_utf8(bytes).map_err(|_| {
                            FrostError::from_static("import module spec is not valid UTF-8")
                        })?;
                        let ctx = self.import_ctx()?;
                        let importer = self.importer.clone();
                        let module = importer.import(spec, &ctx)?;
                        self.stack.push(module);
                    }
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

            // The callee's result replaces the function value that sat at its base:
            // the prelude consumed the args and that function value, and the body
            // left exactly one value behind.
            debug_assert_eq!(
                self.stack.len(),
                frame.base_idx + 1,
                "a returning function must leave exactly its result at its frame base"
            );

            pc = frame
                .return_address
                .expect("IMPOSSIBLE: Callee lacks return address")
                .get();
        }
    }

    /// Run the top-level closure with no arguments.
    /// Equivalent to [`run_with_args`](Self::run_with_args) with an empty list.
    // Both variants carry the warm Vm by design (that is the whole point), so the
    // `Result` is large regardless of the Err; boxing would only add an allocation.
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
        if let Err(error) = Self::check_arity(closure.function.arity, argc, &closure.function.name)
        {
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
        // The operand stack holds at most the tail value: nothing for a program of
        // only `def`/`export def` statements (whose tail is null).
        debug_assert!(
            self.stack.len() <= 1,
            "a completed program must leave at most one value on the stack, found {}",
            self.stack.len()
        );
        // Exactly the top-level frame remains: every call has returned, and the
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
        // Every exported binding was assigned: the invariant `exports`/`get_export` trust.
        for (entry, slot) in base.this_fn.name_table.iter().zip(&base.local_slots) {
            debug_assert!(
                !entry.exported || slot.is_some(),
                "exported binding `{}` was left unfilled after execution",
                entry.name
            );
        }
    }

    /// The running frame's operand floor, or `None` while a native frame is on top
    /// (a native's stack use is its own, not bounded by a Frost frame).
    fn frame_base(&self) -> Option<usize> {
        match self.stack_frames.last() {
            Some(StackFrame::VmFrame(frame)) => Some(frame.base_idx),
            _ => None,
        }
    }

    /// Debug-only check that operands from `lowest` upward belong to the running frame.
    /// Below the frame base sit the caller's operands, which the running function must
    /// not read, consume, or displace: doing so corrupts a frame it cannot see, and
    /// nothing downstream would attribute the damage to this instruction.
    ///
    /// A frame may consume down to its base, where its own function value sits.
    fn debug_assert_own_operand(&self, lowest: usize, op: &str) {
        debug_assert!(
            self.frame_base().is_none_or(|base| lowest >= base),
            "{op} reached stack index {lowest}, below the running frame's base {}",
            self.frame_base().unwrap_or(0)
        );
    }

    /// Debug-only check that the top of the stack belongs to the running frame,
    /// for the operations that read it without consuming it.
    fn debug_assert_own_top(&self, op: &str) {
        self.debug_assert_own_operand(self.stack.len().saturating_sub(1), op);
    }

    /// Debug-only check that a jump landed inside the running function.
    /// One past the end is the return position: a function whose control flow falls
    /// off the end returns, so a jump there is how a branch reaches the exit.
    fn debug_assert_jump_target(&self, pc: usize) {
        debug_assert!(
            pc <= self.this_frame().this_fn.code.len(),
            "jump to {pc} leaves the function, whose code ends at {}",
            self.this_frame().this_fn.code.len()
        );
    }

    /// Pop the top of the operand stack.
    /// A missing operand is a compiler/bytecode bug, not a recoverable error, so underflow panics.
    ///
    /// The single choke point for consuming one operand, so the frame floor is
    /// checked here on behalf of every instruction that pops.
    fn stack_pop(&mut self) -> Value {
        self.debug_assert_own_top("pop");
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

    /// The `Add` opcode: pops both operands into the owned [`Value::add_owned`].
    fn do_add(&mut self) -> Result<(), FrostError> {
        let rhs = self.stack_pop();
        let lhs = self.stack_pop();
        self.stack.push(Value::add_owned(lhs, rhs)?);
        Ok(())
    }

    /// Append the names of the `VmFrame`s in `stack_frames[floor..]` to `err`'s backtrace,
    /// innermost (top of the frame stack) first, then discard those frames.
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
        Err(FrostError::from_string(match arity {
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
        FrostError::from_string(format!(
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
        // Counted unconditionally: `fuel_consumed` reports call counts whether or not
        // a budget is set, so the increment is not skippable when unmetered.
        self.fuel_used = self.fuel_used.saturating_add(1);
        if let Some(budget) = self.config.fuel
            && self.fuel_used > budget.get()
        {
            // Exhaustion is unrecoverable, not an ordinary error: latch it.
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
        FrostError::from_string(format!(
            "Execution exceeded its fuel limit of {limit} function calls"
        ))
    }

    fn call_depth_exceeded(limit: usize) -> FrostError {
        FrostError::from_string(format!(
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
        match arr.try_into_vec() {
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
        let base = self
            .stack
            .len()
            .checked_sub(argc + 1)
            .expect("FROST STACK UNDERFLOW");
        self.debug_assert_own_operand(base, "TailCall");
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

                // A tail call reuses the current frame, except the bottom frame,
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
        // so the frame-walk in `unwind_frames` cannot record it: do it here.
        result.map_err(|mut err| {
            err.backtrace.push(native.name.to_string());
            err
        })
    }
}
