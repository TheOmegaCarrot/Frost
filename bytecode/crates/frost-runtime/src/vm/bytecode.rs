//! The bytecode instruction set.

use std::num::NonZeroUsize;

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

    // Concatenate multiple values into one String.
    // First converts each value as if by the Frost-level `to_string`
    // (`Value::to_frost_string`): top-level Strings are unquoted.
    // The top of the stack appears as the last component of the String.
    // ( x1 x2 ... xN -- s )
    Concat(NonZeroUsize),

    // Flow
    // Jump ahead N instructions
    // N is the number of instructions that are skipped over,
    // such that `Jump(0)` is a funny way to spell `Nop`
    Jump(usize), // unconditionally
    // Consuming conditional jumps: pop the top of the stack and jump if it was
    // true / falsey respectively.
    JumpIfTrue(usize),
    JumpIfFalse(usize),
    // Non-consuming conditional jumps: peek the top of the stack (leaving it in
    // place) and jump if it is true / falsey respectively.
    PeekJumpIfTrue(usize),
    PeekJumpIfFalse(usize),

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
    // Explode an Array's contents onto the stack, but in reverse,
    // so the first element ends on top. Operand must be an Array (panics otherwise).
    // ( a -- aN ... a1 a0 )
    ExplodeArray,

    // Split an Array into two Arrays.
    // The operand is replaced by an Array containing the first N elements of the operand.
    // The remaining elements are collected into an Array which is at the top of the stack.
    // The latter Array is permitted to be empty.
    // Produces an error if its operand is not an Array or is of length less than N.
    // ( [X] -- [N] [X-N] )
    SplitArray(usize),

    // Index a structure, structure is below the index initially (consumed)
    // Leaves a single value on the stack
    SoftIndexStructure, // Null on missing

    // Index a Map with a constant key.
    // The key is in the key-constant pool at the index stored in this variant.
    HardIndexMap(usize), // Error on missing

    // Test if a map contains a key: ( m k -- m k b )
    // Pushes true if Map m contains key k, pushes false if absent or if m is not a Map.
    // Produces an error if k is not a valid Map key.
    TestKey,

    // Look up a key in a Map, without consuming the Map.
    // Produces an error if the key is absent, if m is not a Map, or if k is not a valid Map key.
    // ( m k -- m v )
    ExtractKey,

    // Consumes the value at the top of the stack, and produces a bool depending if the value's
    // type is in the given set.
    TypeTest(EnumSet<FrostType>),

    // Array length queries: ( x -- x b )
    // Does not consume its operand, and pushes true if its operand is an Array satisfying the
    // specified length requirement. Pushes false if the Array size requirement is unsatisfied, or
    // if its operand is not an Array.
    TestArrayLenExact(usize),
    TestArrayLenAtLeast(usize),

    // Consume the value atop the stack and attach it to an error.
    // The error is then produced, and enters the usual flow of a user-code error.
    ProduceError,

    // Pop a module spec from the stack, and push the resolved Value
    Import,
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
