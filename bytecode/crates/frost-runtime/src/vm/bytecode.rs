use crate::core::FrostFloat;

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
