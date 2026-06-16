#![allow(unused)]

mod globals;
pub use globals::GlobalName;

use std::num::NonZeroUsize;
use std::{collections::BTreeMap, sync::Arc};

use crate::{FrostError, FrostFloat, FrostResult, Value};

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
    MakeArray(usize), // Consume N items from the stack, pushing an Array to the stack
    // The top of the stack is the back of the array
    MakeMap(usize), // Consume 2N items from the stack, as kv pairs
    // Keys are below their corresponding values
    ExplodeArray, // Inverse of MakeArray: blast Array contents onto the stack

    // Index a structure, structure is below the index initially (consumed)
    // Leaves a single value on the stack
    SoftIndexStructure, // Null on missing
    HardIndexStructure, // Error on missing
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
pub struct GlobalSet {
    names: Arc<BTreeMap<String, usize>>,
    slots: Vec<Value>,
}

/// Compiled representation of a single function.
/// A script's top-level is also a function.
#[derive(Clone, Debug)]
pub struct CompiledFunction {
    // Functions may or may not have a name
    pub name: Option<String>,
    pub code: Vec<Bytecode>,
    // Functions which are defined in this function's body
    pub child_fns: Vec<Arc<CompiledFunction>>,
    // Constant values that can't be inlined in an opcode.
    // Mostly strings, but can include any structured value the compiler can constant-fold.
    pub constants: Vec<Value>,
    // Table so that locals can be looked up by name at runtime,
    // or their slot given a name by an error.
    pub name_table: BTreeMap<String, NameTableEntry>,
}

#[derive(Debug, Clone)]
pub struct NameTableEntry {
    pub slot: usize,
    pub exported: bool,
}

#[derive(Debug)]
struct StackFrame {
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

pub struct Closure {
    function: Arc<CompiledFunction>,

    // If nonempty, occupy slots 0..n
    captures: Vec<Value>,
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
        self.0
            .base_frame()
            .this_fn
            .name_table
            .get(name)
            .and_then(|nte| {
                if nte.exported {
                    self.0.this_frame().local_slots.get(nte.slot)
                } else {
                    None
                }
            })
            .and_then(|o| o.as_ref())
    }

    /// Get all values exported by the script.
    pub fn exports(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0
            .base_frame()
            .this_fn
            .name_table
            .iter()
            .filter_map(|(name, nte)| {
                if nte.exported {
                    Some((
                        name.as_str(),
                        (self
                            .0
                            .this_frame()
                            .local_slots
                            .get(nte.slot)
                            .and_then(|o| o.as_ref())
                            .expect("IMPOSSIBLE: exported slot unfilled after execution")),
                    ))
                } else {
                    None
                }
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
        vm.stack_frames.push(StackFrame {
            base_idx: 0,
            local_slots: vec![None; program.name_table.len()],
            return_address: None,
            this_fn: program,
        });
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
            stack_frames: vec![StackFrame {
                base_idx: 0,
                local_slots: vec![None; program.name_table.len()],
                return_address: None,
                this_fn: program,
            }],
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
        let base_frame = self
            .stack_frames
            .first_mut()
            .expect("IMPOSSIBLE: Vm lacking base stack frame");
        let Some(nte) = base_frame.this_fn.name_table.get(name) else {
            return false;
        };

        base_frame.local_slots[nte.slot] = Some(value);

        true
    }

    fn this_frame(&self) -> &StackFrame {
        self.stack_frames
            .last()
            .expect("IMPOSSIBLE: Vm has no stack frame")
    }

    fn base_frame(&self) -> &StackFrame {
        self.stack_frames
            .first()
            .expect("IMPOSSIBLE: Vm has no stack frame")
    }

    fn this_frame_mut(&mut self) -> &mut StackFrame {
        self.stack_frames
            .last_mut()
            .expect("IMPOSSIBLE: Vm has no stack frame")
    }

    fn base_frame_mut(&mut self) -> &mut StackFrame {
        self.stack_frames
            .first_mut()
            .expect("IMPOSSIBLE: Vm has no stack frame")
    }

    /// Execute this script.
    /// Any script errors not handled by the script itself are surfaced in the Err case.
    pub fn run(mut self) -> Result<ProgramResult, FrostError> {
        let mut pc: usize = 0;

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
                Bytecode::LoadGlobal(idx) => self.stack.push(self.globals.slots[idx].clone()),
                Bytecode::Add => {
                    todo!();
                }
                Bytecode::Subtract => {
                    todo!();
                }
                Bytecode::Multiply => {
                    todo!();
                }
                Bytecode::Divide => {
                    todo!();
                }
                Bytecode::Modulus => {
                    todo!();
                }
                Bytecode::CompareEqual => {
                    todo!();
                }
                Bytecode::CompareNotEqual => {
                    todo!();
                }
                Bytecode::CompareLessThan => {
                    todo!();
                }
                Bytecode::CompareLessThanOrEqual => {
                    todo!();
                }
                Bytecode::CompareGreaterThan => {
                    todo!();
                }
                Bytecode::CompareGreaterThanOrEqual => {
                    todo!();
                }
                Bytecode::LogicalNot => {
                    todo!();
                }
                Bytecode::Negate => {
                    todo!();
                }
                Bytecode::Jump(n) => {
                    todo!();
                }
                Bytecode::JumpIfTrue(n) => {
                    todo!();
                }
                Bytecode::JumpIfFalse(n) => {
                    todo!();
                }
                Bytecode::Call(arity) => {
                    todo!();
                }
                Bytecode::TailCall(arity) => {
                    todo!();
                }
                Bytecode::CreateClosure {
                    num_captures,
                    function,
                } => {
                    todo!();
                }
                Bytecode::MakeArray(num_elems) => {
                    todo!();
                }
                Bytecode::MakeMap(num_pairs) => {
                    todo!();
                }
                Bytecode::ExplodeArray => {
                    todo!();
                }
                Bytecode::SoftIndexStructure => {
                    todo!();
                }
                Bytecode::HardIndexStructure => {
                    todo!();
                }
            };
            pc += 1;
        }

        Ok(ProgramResult(self))
    }

    fn invoke_native(&self, function: &NativeFunction, args: &mut [Value]) -> FrostResult {
        match function.arity {
            Arity::Exact(req_arity) => {
                if req_arity != args.len() {
                    return Err(FrostError::new(format!(
                        "Function {} requires {} arguments, but got {}",
                        function.name,
                        req_arity,
                        args.len()
                    )));
                }
            }
            Arity::AtLeast(min_arity) => {
                if min_arity < args.len() {
                    return Err(FrostError::new(format!(
                        "Function {} requires at least {} arguments, but got {}",
                        function.name,
                        min_arity,
                        args.len()
                    )));
                }
            }
        }

        todo!()
    }
}
