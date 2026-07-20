//! The bytecode instruction set.

use enumset::EnumSet;

use crate::{FrostFloat, FrostType};

#[derive(PartialEq, Eq, Clone, Debug, Copy, serde::Serialize, serde::Deserialize)]
pub enum Bytecode {
    // Constants
    PushNull,
    PushTrue,
    PushFalse,
    PushInt(i64),
    PushFloat(FrostFloat),

    PeekDown(usize), // Index N down from the top of the stack, and copy that onto the top.
    // PeekDown(0) is just Dup with extra steps.

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

    // Dynamic-arity call: ( f arg_array -- r )
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
    // Inverse of MakeArray: blast Array contents onto the stack.
    // Operand MUST be an Array, and preceding code MUST validate this.
    ExplodeArray,

    // Index a structure, structure is below the index initially (consumed)
    // Leaves a single value on the stack
    SoftIndexStructure, // Null on missing

    // Index a Map with a constant String key.
    // The key is in the constant pool at the index stored in this variant.
    HardIndexMap(usize), // Error on missing

    // Consumes the value at the top of the stack, and produces a bool depending if the value's
    // type is in the given set.
    TypeTest(EnumSet<FrostType>),

    // Consume the value atop the stack and attach it to an error.
    // The error is then produced, and enters the usual flow of a user-code error.
    ProduceError,
}

impl Bytecode {
    /// Dup is equivalent to PeekDown(0).
    #[allow(non_upper_case_globals)]
    pub const Dup: Bytecode = Bytecode::PeekDown(0);

    /// Pop is equivalent to DropBelow(0).
    #[allow(non_upper_case_globals)]
    pub const Pop: Bytecode = Bytecode::DropBelow(0);

    /// Nop is equivalent to Jump(0).
    #[allow(non_upper_case_globals)]
    pub const Nop: Bytecode = Bytecode::Jump(0);
}
