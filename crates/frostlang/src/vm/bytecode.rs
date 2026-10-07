//! The bytecode instruction set.

use std::num::NonZeroUsize;

use enumset::EnumSet;

use crate::{FrostFloat, FrostType};

/// One VM instruction; a [`CompiledFunction`](crate::bytecode::CompiledFunction)'s `code` is a sequence of them.
///
/// Stack effects are written `( before -- after )`, with the top of the stack rightmost.
/// A binary operator's effect is `( lhs rhs -- result )`: the right operand is on top.
///
/// An instruction that errors raises a recoverable Frost error.
/// One that panics has been given malformed bytecode
/// (see [`CompiledFunction::assert_trusted`](crate::bytecode::CompiledFunction::assert_trusted)).
///
/// # Calling convention
///
/// To call a function, push it, then its arguments in order, then [`Call`](Self::Call)`(N)`.
/// A native callee receives the arguments as a slice (see [`NativeFn`](crate::NativeFn)).
///
/// A closure callee runs in a new frame, with its captures already seated in slots `0..num_captures`.
/// The frame's stack starts as `( f a1 ... aN )`, with `f` the callee itself,
/// adjusted by the callee's [`Arity`](crate::Arity):
///
/// - `Exact`: unchanged.
/// - `AtLeast(k)`: the arguments after the first `k` are collected into a rest Array: `( f a1 ... ak rest )`.
/// - `Between`: the argument count is pushed as an Int: `( f a1 ... aN N )`.
///   This lets the callee tell an omitted argument from one passed as `null`.
///
/// The callee's code must consume all of these, `f` included,
/// and end with exactly its result on the stack.
/// That result replaces `f` in the caller.
#[derive(PartialEq, Eq, Clone, Debug, Copy, serde::Serialize, serde::Deserialize)]
pub enum Bytecode {
    // Constants
    /// Push `null`.
    PushNull,
    /// Push `true`.
    PushTrue,
    /// Push `false`.
    PushFalse,
    /// Push the given Int.
    PushInt(i64),
    /// Push the given Float.
    PushFloat(FrostFloat),

    /// Copy the value N below the top of the stack onto the top: `( xN ... x0 -- xN ... x0 xN )`.
    /// `PeekDown(0)` is [`Dup`](Self::Dup).
    PeekDown(usize),

    /// Remove the value N below the top of the stack, keeping the values above it.
    /// `DropBelow(0)` is [`Pop`](Self::Pop).
    DropBelow(usize),

    // Slots
    /// Move the top of the stack into local slot N: `( x -- )`.
    /// Slots are indexed like the current function's [`name_table`](crate::bytecode::CompiledFunction::name_table).
    DefLocal(usize),
    /// Copy local slot N onto the stack: `( -- x )`.
    LoadLocal(usize),
    /// Move local slot N onto the stack, leaving the slot empty: `( -- x )`.
    /// The value is not shared with the slot, so a structure held nowhere else
    /// can then be updated in place. The slot must not be read again unless it is
    /// defined anew.
    ConsumeLocal(usize),
    /// Copy entry N of the current function's [`constants`](crate::bytecode::CompiledFunction::constants) onto the stack.
    LoadConst(usize),
    /// Copy predefined global N onto the stack; [`GLOBAL_NAMES`](crate::bytecode::GLOBAL_NAMES) gives the slot order.
    LoadGlobal(usize),

    // Arithmetic
    /// `lhs + rhs`; see [`Value::add`](crate::Value::add).
    Add,
    /// `lhs - rhs`; see [`Value::subtract`](crate::Value::subtract).
    Subtract,
    /// `lhs * rhs`; see [`Value::multiply`](crate::Value::multiply).
    Multiply,
    /// `lhs / rhs`; see [`Value::divide`](crate::Value::divide).
    Divide,
    /// `lhs % rhs`; see [`Value::modulus`](crate::Value::modulus).
    Modulus,

    // Comparison
    /// `lhs == rhs`; never errors.
    CompareEqual,
    /// `lhs != rhs`; never errors.
    CompareNotEqual,
    /// `lhs < rhs`; see [`Value::compare`](crate::Value::compare).
    CompareLessThan,
    /// `lhs <= rhs`; see [`Value::compare`](crate::Value::compare).
    CompareLessThanOrEqual,
    /// `lhs > rhs`; see [`Value::compare`](crate::Value::compare).
    CompareGreaterThan,
    /// `lhs >= rhs`; see [`Value::compare`](crate::Value::compare).
    CompareGreaterThanOrEqual,

    // Unary
    /// `not x`: `( x -- b )`, where `b` is true when `x` is falsy.
    LogicalNot,
    /// Unary `-x`; see [`Value::negate`](crate::Value::negate).
    Negate,

    /// Concatenate the top N values, deepest first, into one String: `( x1 ... xN -- s )`.
    /// Each value is converted as by the Frost `to_string` global
    /// ([`Value::to_frost_string`](crate::Value::to_frost_string)): top-level Strings are unquoted.
    Concat(NonZeroUsize),

    // Flow
    /// Skip the next N instructions.
    /// `Jump(0)` is [`Nop`](Self::Nop).
    Jump(usize),
    /// Pop the top of the stack, and skip the next N instructions if it is truthy: `( x -- )`.
    JumpIfTrue(usize),
    /// Pop the top of the stack, and skip the next N instructions if it is falsy: `( x -- )`.
    JumpIfFalse(usize),
    /// Skip the next N instructions if the top of the stack is truthy, leaving it in place: `( x -- x )`.
    PeekJumpIfTrue(usize),
    /// Skip the next N instructions if the top of the stack is falsy, leaving it in place: `( x -- x )`.
    PeekJumpIfFalse(usize),

    // Functions
    /// Call a function with N arguments, the last on top: `( f a1 ... aN -- r )`.
    /// Errors if `f` is not a function or does not accept N arguments.
    /// See the [calling convention](Self#calling-convention).
    Call(usize),
    /// [`Call`](Self::Call) in tail position: the callee's result becomes the current function's result,
    /// and the call does not grow the call stack.
    /// Emit it only where the call's result is the function's result: the instructions after it may or may not run.
    TailCall(usize),

    // Purpose-built for the `call` builtin.
    /// [`TailCall`](Self::TailCall) with the arguments spread from an Array: `( f args -- r )`.
    /// Errors if `args` is not an Array.
    DynTailCall,

    /// Create a closure over entry N of the current function's [`child_fns`](crate::bytecode::CompiledFunction::child_fns):
    /// `( c1 ... cK -- f )`, where K is the child's [`num_captures`](crate::bytecode::CompiledFunction::num_captures).
    /// The top K values become its captures, deepest first.
    CreateClosure(usize),

    // Data structures
    /// Collect the top N values, deepest first, into an Array: `( x1 ... xN -- a )`.
    MakeArray(usize),
    /// Collect the top N key-value pairs into a Map, each key below its value: `( k1 v1 ... kN vN -- m )`.
    /// Errors if a key is not a valid Map key.
    MakeMap(usize),
    /// Push an Array's elements in reverse, so the first element ends on top: `( a -- aN ... a1 a0 )`.
    /// Panics if the operand is not an Array.
    ExplodeArray,

    /// Split an Array after its first N elements: `( [X] -- [X-N] [N] )`.
    /// The remaining elements (possibly none) replace the operand, and an Array of the first N goes on top of it.
    /// Errors if the operand is not an Array or has fewer than N elements.
    SplitArray(usize),

    /// Index an Array or Map, yielding `null` for a missing element or key: `( s i -- v )`.
    /// Errors if `s` is neither, or if `i` is not an Int (for an Array) or a valid Map key (for a Map).
    SoftIndexStructure,

    /// Look up a constant key in a Map: `( m -- v )`.
    /// The key is entry N of the current function's [`key_constants`](crate::bytecode::CompiledFunction::key_constants).
    /// Errors if `m` is not a Map or lacks the key.
    HardIndexMap(usize),

    /// Test whether a Map contains a key, consuming neither: `( m k -- m k b )`.
    /// `b` is false if the key is absent or `m` is not a Map.
    /// Errors if `m` is a Map and `k` is not a valid Map key.
    TestKey,

    /// Look up a key in a Map, keeping the Map: `( m k -- m v )`.
    /// Errors if the key is absent, `m` is not a Map, or `k` is not a valid Map key.
    ExtractKey,

    /// Test whether a Map contains a constant key, keeping the Map: `( m -- m b )`.
    /// The key is entry N of the current function's [`key_constants`](crate::bytecode::CompiledFunction::key_constants).
    /// `b` is false if the key is absent or `m` is not a Map.
    TestConstKey(usize),

    /// Look up a constant key in a Map, keeping the Map: `( m -- m v )`.
    /// The key is entry N of the current function's [`key_constants`](crate::bytecode::CompiledFunction::key_constants).
    /// Errors if the key is absent or `m` is not a Map.
    ExtractConstKey(usize),

    /// Test whether a value's type is in the given set: `( x -- b )`.
    TypeTest(EnumSet<FrostType>),

    /// Test whether a value is an Array of exactly N elements, keeping it: `( x -- x b )`.
    /// `b` is false if `x` is not an Array.
    TestArrayLenExact(usize),
    /// Test whether a value is an Array of at least N elements, keeping it: `( x -- x b )`.
    /// `b` is false if `x` is not an Array.
    TestArrayLenAtLeast(usize),

    // Stack marking and truncation
    /// Mark the current stack height, with no stack effect.
    /// Marks belong to the current function call,
    /// and each must be consumed by [`DropMark`](Self::DropMark) or [`RewindToMark`](Self::RewindToMark) before that call returns.
    MarkStack,

    /// Discard the most recent stack height mark, with no stack effect.
    /// Panics if there is no mark.
    DropMark,

    /// Truncate the stack to the most recent stack height mark, consuming that mark.
    /// Panics if there is no mark.
    RewindToMark,

    /// Raise the top of the stack as a Frost error: `( x -- )`.
    /// It propagates like any other runtime error.
    ProduceError,

    /// Resolve a module spec through the Vm's [`Importer`](crate::Importer): `( spec -- module )`.
    /// Errors if `spec` is not a String or the import fails.
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
