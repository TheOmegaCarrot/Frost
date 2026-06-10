#![allow(unused)]

use std::num::NonZeroUsize;
use std::{collections::BTreeMap, sync::Arc};

use crate::core::FrostFloat;
use crate::{FrostError, Value};

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
    DefLocal(usize),  // Move the top of the stack to local slot N
    LoadLocal(usize), // Copy local slot N to the top of the stack
    LoadConst(usize), // Copy constant slot N to the top of the stack

    // Arithmetic
    // Consume 2 stack items, and the rhs is the top item, result is one item on the stack
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    // Same stack conventions as arithmetic
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,

    // Unary
    Not,
    Neg,

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

    // Iterative
    // Structure is below function (consumed)
    // Leaves a single value on the stack
    Map,
    Filter,
    Reduce,
    Foreach,

    // Structure is below the init, and the function is atop the init (consumed)
    // Leaves a single value on the stack
    ReduceWithInit,
}

// ============================================================
// VM Types
// ============================================================

/// VM state
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
    // A native call moves its args to the first available frame,
    // which is then marked unoccupied when that call exits.
    // This allows a native call to directly consume its arguments
    // without a mutable borrow of the stack, and call back into the VM cleanly.
    native_arg_frames: NativeArgFrames,
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
struct NativeArgFrames {
    // All of the native frames.
    args: Vec<Vec<Value>>,
    // Index to the next free frame.
    next: usize,
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
pub struct ProgramResult {
    // The top-level's slots, for post-execution retrieval
    slots: Vec<Option<Value>>,
    // Table of names of top-levels globals, so post-execution retrieval
    name_table: BTreeMap<String, NameTableEntry>,
    // The tail expression of the top-level. Often irrelevant, but it's available.
    tail: Value,
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
        &self.tail
    }

    /// Look up the value of a variable.
    /// None indicates the name was not defined.
    pub fn lookup<'a>(&'a self, name: &str) -> Option<&'a Value> {
        self.name_table
            .get(name)
            .and_then(|nte| self.slots.get(nte.slot))
            .and_then(|o| o.as_ref())
    }

    /// Get all values exported by the script.
    pub fn exports(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.name_table.iter().filter_map(|(name, nte)| {
            if nte.exported {
                Some((
                    name,
                    (self
                        .slots
                        .get(nte.slot)
                        .and_then(|o| o.as_ref())
                        .expect("IMPOSSIBLE: exported slot unfilled after execution")),
                ))
            } else {
                None
            }
        })
    }
}

impl Vm {
    /// Create a Vm instance from a CompiledFunction.
    /// The CompiledFunction has many invariants which cannot be thoroughly checked,
    /// and the Frost compiler is relied upon for emitting correct code.
    /// If the provided CompiledFunction is incorrect (indexes a missing slot, pops an empty stack, etc),
    /// then execution or other Vm methods may panic.
    /// Any use of a CompiledFunction which was not emitted by the compiler is unsupported.
    ///
    /// A Vm is pretty cheap to construct, and so a Vm will only run one Frost program.
    pub fn new(program: Arc<CompiledFunction>) -> Result<Vm, FrostError> {
        Ok(Self {
            stack: Vec::new(),
            stack_frames: vec![StackFrame {
                base_idx: 0,
                local_slots: vec![None; program.name_table.len()],
                return_address: None,
                this_fn: program,
            }],
            native_arg_frames: NativeArgFrames {
                next: 0,
                args: Vec::new(),
            },
        })

        // TODO: fill in global predefined values
    }

    /// Assign a global binding to be used by the script.
    /// Some scripts may use values provided directly by the host application,
    /// and this is the mechanism to provide them.
    /// This method is not necessary if the script only uses bindings that it defines itself or
    /// that are provided by the Frost runtime.
    /// This allows for overridding existing global bindings, including runtime-provided values,
    /// which should be done with care.
    /// Returns true if the script may use the binding, or false if it does not.
    pub fn set_global(&mut self, name: &str, value: Value) -> bool {
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

    fn this_frame_mut(&mut self) -> &mut StackFrame {
        self.stack_frames
            .last_mut()
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
                Bytecode::LoadConst(idx) => {
                    todo!();
                }
                Bytecode::Add => {
                    todo!();
                }
                Bytecode::Sub => {
                    todo!();
                }
                Bytecode::Mul => {
                    todo!();
                }
                Bytecode::Div => {
                    todo!();
                }
                Bytecode::Mod => {
                    todo!();
                }
                Bytecode::Eq => {
                    todo!();
                }
                Bytecode::Neq => {
                    todo!();
                }
                Bytecode::Lt => {
                    todo!();
                }
                Bytecode::Lte => {
                    todo!();
                }
                Bytecode::Gt => {
                    todo!();
                }
                Bytecode::Gte => {
                    todo!();
                }
                Bytecode::Not => {
                    todo!();
                }
                Bytecode::Neg => {
                    todo!();
                }
                Bytecode::Jump(_) => {
                    todo!();
                }
                Bytecode::JumpIfTrue(_) => {
                    todo!();
                }
                Bytecode::JumpIfFalse(_) => {
                    todo!();
                }
                Bytecode::Call(_) => {
                    todo!();
                }
                Bytecode::TailCall(_) => {
                    todo!();
                }
                Bytecode::CreateClosure {
                    num_captures,
                    function,
                } => {
                    todo!();
                }
                Bytecode::MakeArray(_) => {
                    todo!();
                }
                Bytecode::MakeMap(_) => {
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
                Bytecode::Map => {
                    todo!();
                }
                Bytecode::Filter => {
                    todo!();
                }
                Bytecode::Reduce => {
                    todo!();
                }
                Bytecode::Foreach => {
                    todo!();
                }
                Bytecode::ReduceWithInit => {
                    todo!();
                }
            };
            pc += 1;
        }

        let mut base_frame = self
            .stack_frames
            .into_iter()
            .next()
            .expect("IMPOSSIBLE: VM has no stack frame");

        let this_fn = base_frame.this_fn;

        Ok(ProgramResult {
            slots: base_frame.local_slots,
            name_table: match Arc::try_unwrap(this_fn) {
                Ok(func) => func.name_table,
                Err(arc) => arc.name_table.clone(),
            },
            tail: self.stack.pop().unwrap_or(Value::Null),
        })
    }
}
