#![allow(unused)]

mod globals;

// White-box tests for the one unwind invariant not observable through the public
// API: native-arg-pool buffer recycling. (Operand-stack and frame restoration are
// covered black-box in tests/vm_errors.rs.) Kept in their own file -- a child
// module still reaches this module's private `Vm` internals.
#[cfg(test)]
mod arg_pool_tests;

use std::debug_assert_matches;
use std::num::NonZeroUsize;
use std::sync::Arc;

use itertools::Itertools;

use crate::{
    FrostArray, FrostError, FrostFloat, FrostMap, FrostResult, FrostTypeCategory, MapKey, Value,
};

// ============================================================
// Bytecode
// ============================================================

#[derive(Clone, Debug, Copy)]
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
    Jump(usize),        // unconditionally
    JumpIfTrue(usize),  // only if the top of the stack is true (NOT consumed)
    JumpIfFalse(usize), // only if the top of the stack is falsey (NOT consumed)

    // Functions
    // N args on the stack, with a function under the args
    // The last argument is the top of the stack
    Call(usize),
    TailCall(usize),

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
    HardIndexStructure, // Error on missing

    // Consumes the value at the top of the stack, and produces a bool depending if the value fits
    // the given type category.
    TypeTest(FrostTypeCategory),
}

// ============================================================
// VM Types
// ============================================================

/// VM state and execution context.
#[derive(Debug)]
pub struct Vm {
    // The working stack of the Vm
    stack: Vec<Value>,
    // Represents a call frame.
    // One is pushed by calling,
    // which is then popped when that call exits.
    // The bottom StackFrame holds globals (runtime populates predefined globals prior to execution).
    stack_frames: Vec<StackFrame>,
    // Used to hold the args of a native function call.
    // A native call acquires a Vec from this pool,
    // moves args from the stack to that Vec (or makes a new one), then clears it and returns it.
    // This allows for re-use of allocations for native args, while allowing a native call to hold
    // mutable references to their args AND the Vm separately.
    native_arg_pool: Vec<Vec<Value>>,
    globals: Arc<GlobalSet>,
}

#[derive(Debug, Clone)]
pub struct GlobalSet(Vec<Value>);

/// Compiled representation of a single function.
/// A script's top-level is also a function.
#[derive(Clone, Debug)]
pub struct CompiledFunction {
    // Functions always have a name
    pub name: String,
    pub code: Vec<Bytecode>,
    // Functions which are defined in this function's body
    pub child_fns: Vec<Arc<CompiledFunction>>,
    // Constant values that can't be inlined in an opcode.
    // Mostly strings, but can include any structured value the compiler can constant-fold.
    pub constants: Vec<Value>,
    // Table so that locals can be looked up by name at runtime,
    // or their slot given a name by an error.
    pub name_table: Vec<NameEntry>,
    // Arity of top-level is Exact(0)
    pub arity: Arity,
}

#[derive(Debug, Clone)]
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
/// Every function has a certain number of fixed args,
/// and may or may not be variadic.
/// ```frost
/// fn a, b, c, ...more -> ...
/// # at least 3
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arity {
    Exact(usize),
    AtLeast(usize),
}

#[derive(Debug)]
pub struct NativeCtx<'a>(pub(crate) &'a mut Vm);

impl NativeCtx<'_> {
    pub fn invoke(
        &mut self,
        function: &Value,
        args: impl IntoIterator<Item = Value>,
    ) -> FrostResult {
        match function {
            Value::NativeFunction(native) => {
                let mut buf = self.0.native_arg_pool.pop().unwrap_or_default();
                buf.extend(args);
                self.0.run_native(native, buf)
            }

            Value::Closure(closure) => {
                let base = self.0.stack.len();
                self.0.stack.push(function.clone());
                self.0.stack.extend(args);
                let argc = self.0.stack.len() - base - 1;

                // This native boundary is where an unwinding error stops. On any
                // error path below, restore the Vm to its pre-call shape before
                // returning Err -- truncate the operand stack back to `base` and
                // discard the Frost frames left above our entry floor -- so that a
                // catching native (e.g. `try_call`) resumes on a clean Vm.
                if let Err(err) =
                    Vm::check_arity(closure.function.arity, argc, &closure.function.name)
                {
                    // The callee never ran: only the function value and its args
                    // (no frame) sit above `base`.
                    self.0.stack.truncate(base);
                    return Err(err);
                }

                let frame_floor = self.0.stack_frames.len();
                self.0.push_closure_frame(closure, base, None);

                if let Err(err) = self.0.execute_function() {
                    let err = self.0.unwind_frames(frame_floor, err);
                    self.0.stack.truncate(base);
                    return Err(err);
                }

                let result = self
                    .0
                    .stack
                    .pop()
                    .expect("IMPOSSIBLE: closure left no result");
                self.0.stack_frames.pop();
                Ok(result)
            }
            _ => Err(Vm::not_callable(function)),
        }
    }
}

type NativeFn = dyn Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static;

pub struct NativeFunction {
    arity: Arity,
    function: Box<NativeFn>,
    name: String,
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
    pub fn new<F>(function: F, name: &str, arity: Arity) -> Self
    where
        F: Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static,
    {
        Self {
            arity,
            name: name.to_owned(),
            function: Box::new(function),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn arity(&self) -> Arity {
        self.arity
    }
}

#[derive(Debug)]
pub struct Closure {
    function: Arc<CompiledFunction>,

    // If nonempty, occupy slots 0..n
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
            .and_then(|(_, slot)| slot.as_ref())
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

    /// Prepare the Vm for another execution.
    /// Any values bound using set_binding are not preserved.
    /// Any customized globals are preserved.
    /// The configured importer is preserved.
    ///
    /// The advantage to using this function over creating a new Vm from scratch
    /// is that this allows the implementation to re-use internal allocations,
    /// reducing the number of allocation calls.
    ///
    /// If you want to optimize for speed and predictable performance characteristics,
    /// keep re-using a Vm using this method.
    /// If you'd rather keep total memory consumption at a minimum, even at the cost of performance
    /// and predictability, just drop the ProgramResult and create a new Vm.
    pub fn reset(self, program: Arc<CompiledFunction>) -> Vm {
        let mut vm = self.0;
        vm.stack.clear();
        vm.stack_frames.clear();
        vm.stack_frames.push(StackFrame::VmFrame(VmFrame {
            base_idx: 0,
            local_slots: vec![None; program.name_table.len()],
            return_address: None,
            this_fn: program,
        }));
        vm
    }
}

impl Vm {
    /// Create a [Vm] instance from a [CompiledFunction].
    /// The [CompiledFunction] has many invariants which cannot be thoroughly checked,
    /// and the Frost compiler is relied upon for emitting correct code.
    /// If the provided [CompiledFunction] is incorrect (indexes a missing slot, pops an empty stack, etc),
    /// then execution or other Vm methods may panic.
    /// Any use of a [CompiledFunction] which was not emitted by the compiler is unsupported.
    ///
    /// A Vm is pretty cheap to construct, and so a Vm will only run one Frost program.
    pub fn new(program: Arc<CompiledFunction>) -> Result<Vm, FrostError> {
        Self::new_with_globals(program, GlobalSet::defaults())
    }

    /// Create a [Vm] with explicit [GlobalSet].
    /// This function is only useful if you intend to override some default globals,
    /// otherwise just use [Vm::new].
    pub fn new_with_globals(
        program: Arc<CompiledFunction>,
        globals: Arc<GlobalSet>,
    ) -> Result<Vm, FrostError> {
        Ok(Self {
            stack: Vec::new(),
            stack_frames: vec![StackFrame::VmFrame(VmFrame {
                base_idx: 0,
                local_slots: vec![None; program.name_table.len()],
                return_address: None,
                this_fn: program,
            })],
            native_arg_pool: Vec::new(),
            globals,
        })
    }

    /// Assign a predefined binding to be used by the script.
    /// Some scripts may use values provided directly by the host application,
    /// and this is the mechanism to provide them.
    /// This method is not necessary if the script only uses bindings that it defines itself or
    /// that are provided by the Frost runtime.
    /// Returns true if the script may use the binding, or false if it does not.
    pub fn set_binding(&mut self, name: &str, value: Value) -> bool {
        let base_frame = self.base_frame_mut();

        let Some(slot) = base_frame
            .this_fn
            .name_table
            .iter()
            .position(|entry| entry.name == name)
        else {
            return false;
        };

        base_frame.local_slots[slot] = Some(value);

        true
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

    fn base_frame_mut(&mut self) -> &mut VmFrame {
        match (self.stack_frames.first_mut()) {
            Some(StackFrame::VmFrame(vm_frame)) => vm_frame,
            Some(StackFrame::NativeFrame) => panic!("IMPOSSIBLE: base Vm frame is native frame"),
            None => panic!("IMPOSSIBLE: Vm has no frame"),
        }
    }

    fn execute_function(&mut self) -> Result<(), FrostError> {
        let mut pc: usize = 0;
        let floor = self.stack_frames.len(); // 1 for run(),
        // or more for native -> Vm re-entrancy

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
                        self.stack.pop();
                    }
                    Bytecode::Dup => self
                        .stack
                        .push(self.stack.last().expect("FROST STACK UNDERFLOW").clone()),
                    Bytecode::PeekDown(idx) => {
                        self.stack.push(self.stack[self.stack.len() - idx].clone());
                    }
                    Bytecode::DefLocal(idx) => {
                        self.this_frame_mut().local_slots[idx] =
                            Some(self.stack.pop().expect("FROST STACK UNDERFLOW"));
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
                    // Arithmetic delegates to the operators defined on `Value`; a
                    // type error or division by zero surfaces as an `Err` that `?`
                    // propagates straight out of this activation (see `unwind_frames`).
                    Bytecode::Add => self.binary_op(Value::add)?,
                    Bytecode::Subtract => self.binary_op(Value::subtract)?,
                    Bytecode::Multiply => self.binary_op(Value::multiply)?,
                    Bytecode::Divide => self.binary_op(Value::divide)?,
                    Bytecode::Modulus => self.binary_op(Value::modulus)?,
                    // Equality is infallible (`Value: Eq`). Ordering delegates to
                    // `Value::compare`, where an unorderable pair is a type error.
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
                        let operand = self.stack.pop().expect("FROST STACK UNDERFLOW");
                        let result = Value::from(!operand.is_truthy());
                        self.stack.push(result);
                    }
                    Bytecode::Negate => {
                        let operand = self.stack.pop().expect("FROST STACK UNDERFLOW");
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
                        let base = self.stack.len() - (argc + 1);
                        let function = self.stack[base].clone();

                        // Errors below `?` straight out of `execute_function`; the
                        // boundary that entered this activation (`invoke` or `run`)
                        // truncates the abandoned frames and operands.
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
                                self.push_closure_frame(&closure, base, NonZeroUsize::new(pc + 1));
                                pc = 0;
                                continue;
                            }
                            _ => return Err(Self::not_callable(&function)),
                        }
                    }
                    Bytecode::TailCall(argc) => {
                        let base = self.stack.len() - (argc + 1);
                        let function = self.stack[base].clone();

                        match function {
                            // A native callee adds no VM frame, so there is nothing to
                            // elide: run it exactly like a plain `Call`. Its result is
                            // left on the stack and, since `TailCall` sits in tail
                            // position, the enclosing function returns it via the normal
                            // end-of-code path on the next loop turn.
                            Value::NativeFunction(native_fn) => {
                                self.native_call(&native_fn, argc)?
                            }
                            Value::Closure(closure) => {
                                // Check arity BEFORE popping the caller frame, so an arity error
                                // does not destroy the frame the error path still needs.
                                Self::check_arity(
                                    closure.function.arity,
                                    argc,
                                    &closure.function.name,
                                )?;

                                let StackFrame::VmFrame(gone_frame) = self
                                    .stack_frames
                                    .pop()
                                    .expect("IMPOSSIBLE: Vm has no frame")
                                else {
                                    panic!("IMPOSSIBLE: tail call in native frame");
                                };

                                // Reuse the popped frame's slot, inheriting its base and return
                                // address so the callee returns to F's original caller.
                                // The TCO: one frame popped, one pushed, net zero.
                                self.push_closure_frame(
                                    &closure,
                                    gone_frame.base_idx,
                                    gone_frame.return_address,
                                );
                                pc = 0;
                                continue;
                            }
                            _ => return Err(Self::not_callable(&function)),
                        }
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
                    Bytecode::ExplodeArray => {
                        let arr = self.stack.pop().expect("FROST STACK UNDERFLOW");
                        let arr = match arr {
                            Value::Array(inner_arr) => inner_arr,
                            _ => panic!("ExplodeArray: operand not Array"),
                        };

                        match arr.try_extract() {
                            Ok(vec) => self.stack.extend(vec),
                            Err(arr) => self.stack.extend(arr.iter().cloned()),
                        }
                    }
                    Bytecode::SoftIndexStructure => {
                        todo!();
                    }
                    Bytecode::HardIndexStructure => {
                        todo!();
                    }
                    Bytecode::TypeTest(tc) => {
                        let operand = self.stack.pop().expect("FROST STACK UNDERFLOW");
                        self.stack.push(operand.fits_category(tc).into());
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

            pc = frame
                .return_address
                .expect("IMPOSSIBLE: Callee lacks return address")
                .get();
        }
    }

    /// Execute this script.
    /// Any script errors not handled by the script itself are surfaced in the Err case.
    pub fn run(mut self) -> Result<ProgramResult, FrostError> {
        match self.execute_function() {
            Ok(_) => Ok(ProgramResult(self)),
            // No `NativeFrame` exists above the top level, so the error has nowhere
            // to be caught: accumulate the backtrace across every remaining frame
            // and surface it to the host. `self` -- now in an unrecoverable state --
            // is dropped (a `Vm` runs a single program; `reset` lives only on the
            // success path).
            Err(err) => Err(self.unwind_frames(0, err)),
        }
    }

    /// Pop the top two operands (rhs on top, lhs below) and push `op(lhs, rhs)`.
    /// Any operator error `?`-propagates out of the current activation; the
    /// consumed operands are simply dropped, since the unwind path truncates the
    /// operand stack back to the boundary floor regardless of its exact height.
    fn binary_op(
        &mut self,
        op: impl FnOnce(&Value, &Value) -> Result<Value, FrostError>,
    ) -> Result<(), FrostError> {
        let rhs = self.stack.pop().expect("FROST STACK UNDERFLOW");
        let lhs = self.stack.pop().expect("FROST STACK UNDERFLOW");
        self.stack.push(op(&lhs, &rhs)?);
        Ok(())
    }

    /// Append the names of the `VmFrame`s in `stack_frames[floor..]` to `err`'s
    /// backtrace -- innermost (top of the frame stack) first -- then discard those
    /// frames.
    ///
    /// This is the only place a Frost frame's name reaches the backtrace: `?`
    /// propagation has no hook, so the trace is built here as the abandoned frames
    /// are dropped. Called at each native boundary that catches an unwinding error
    /// ([`NativeCtx::invoke`]) and at the top-level terminus ([`Vm::run`]).
    /// `NativeFrame` markers carry no name; a native's own name is recorded by
    /// [`Vm::run_native`] instead.
    fn unwind_frames(&mut self, floor: usize, mut err: FrostError) -> FrostError {
        for frame in self.stack_frames.drain(floor..).rev() {
            if let StackFrame::VmFrame(vm_frame) = frame {
                err = err.with_frame(vm_frame.this_fn.name.clone());
            }
        }
        err
    }

    /// Returns an arity-mismatch error if `argc` does not satisfy `arity`, or `Ok(())` if it does.
    /// `name` is the called function's name, for the error message.
    fn check_arity(arity: Arity, argc: usize, name: &str) -> Result<(), FrostError> {
        let ok = match arity {
            Arity::Exact(n) => argc == n,
            Arity::AtLeast(n) => argc >= n,
        };
        if ok {
            return Ok(());
        }
        Err(FrostError::new(match arity {
            Arity::Exact(n) => {
                format!("Function {name} expects {n} arguments, but was called with {argc}")
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

    /// Push a [VmFrame] to enter `closure`: captures seat into slots `0..n`,
    /// then variadic args are collapsed into a trailing rest array.
    /// `base` is the frame base (the closure's slot on the stack);
    /// `return_address` is where the callee returns to.
    /// Arity must already be checked.
    fn push_closure_frame(
        &mut self,
        closure: &Closure,
        base: usize,
        return_address: Option<NonZeroUsize>,
    ) {
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

        if let Arity::AtLeast(fixed_argc) = closure.function.arity {
            let varargs = self.stack.split_off(base + 1 + fixed_argc);
            self.stack.push(Value::Array(varargs.into()));
        }
    }

    fn native_call(&mut self, function: &NativeFunction, argc: usize) -> Result<(), FrostError> {
        let mut buf = self.native_arg_pool.pop().unwrap_or_default();
        buf.extend(self.stack.drain((self.stack.len() - argc)..));
        self.stack.pop(); // pop the function value off the stack

        let result = self.run_native(function, buf)?;
        self.stack.push(result);
        Ok(())
    }

    /// Invoke `native` with its args already collected in `buf`, then recycle `buf`.
    /// Checks arity, brackets the call with a `NativeFrame` marker, and returns the native's result.
    /// `buf` is reclaimed to the pool on every path.
    fn run_native(&mut self, native: &NativeFunction, mut buf: Vec<Value>) -> FrostResult {
        if let Err(err) = Self::check_arity(native.arity, buf.len(), &native.name) {
            buf.clear();
            self.native_arg_pool.push(buf);
            return Err(err);
        }

        self.stack_frames.push(StackFrame::NativeFrame);
        let result = (native.function)(NativeCtx(self), &mut buf);
        buf.clear();
        self.native_arg_pool.push(buf);

        let popped = self.stack_frames.pop();
        debug_assert_matches!(
            popped,
            Some(StackFrame::NativeFrame),
            "run_native must pop the NativeFrame it pushed"
        );

        // A native that ran and failed contributes its own name to the backtrace.
        // Its only call-stack presence is a nameless `NativeFrame` marker, so the
        // frame-walk in `unwind_frames` cannot record it -- do it here.
        result.map_err(|err| err.with_frame(native.name.clone()))
    }
}
