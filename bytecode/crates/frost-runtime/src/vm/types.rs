use std::{collections::BTreeMap, sync::Arc};

use crate::{Value, vm::bytecode::Bytecode};

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
