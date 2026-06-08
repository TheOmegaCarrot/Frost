#![allow(unused)]

use std::{collections::BTreeMap, sync::Arc};

use crate::core::FrostFloat;
use crate::Value;

// ============================================================
// Bytecode
// ============================================================

#[derive(Clone, Debug)]
pub enum Bytecode {
    // Constants
    PushNull,
    PushTrue,
    PushFalse,
    PushInt(i64),
    PushFloat(FrostFloat),

    // Stack
    Pop,
    PeekDown(usize), // Index N down from the top of the stack, and copy that onto the top

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
    Jump(usize), // unconditionally
    JumpIfTrue(usize), // only if the top of the stack is true (NOT consumed)
    JumpIfFalse(usize), // only if the top of the stack is falsey (NOT consumed)

    // Functions
    // N args on the stack, with a function under the args
    // The last argument is the top of the stack
    Call(usize),
    TailCall(usize),

    // Move num_captures elements from the stack to a closure capture structure,
    // as a part of a new closure with code from function (index into function table)
    CreateClosure{ num_captures: u32, function: u32 },

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
struct Vm {
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
struct CompiledFunction {
    // Functions may or may not have a name
    name: Option<String>,
    code: Vec<Bytecode>,
    // Functions which are defined in this function's body
    child_fns: Vec<Arc<CompiledFunction>>,
    // Constant values that can't be inlined in an opcode.
    // Mostly strings, but can include any structured value the compiler can constant-fold.
    constants: Vec<Value>,
    // Table so that locals can be looked up by name at runtime,
    // or their slot given a name by an error.
    name_table: BTreeMap<String, NameTableEntry>,
}

struct NameTableEntry {
    slot: usize,
    exported: bool,
}

struct NativeArgFrames {
    // All of the native frames.
    args: Vec<Vec<Value>>,
    // Index to the next free frame.
    next: usize,
}

struct StackFrame {
    // Absolute stack index of the base of a frame
    base_idx: usize,
    // Storage for local variables.
    local_slots: Vec<Option<Value>>,
    // Index into the code of the calling function which should be jumped back to.
    // The instruction _after_ the Call that pushed this StackFrame.
    return_address: usize,
    // The function represented by this StackFrame.
    this_fn: Arc<CompiledFunction>,
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
