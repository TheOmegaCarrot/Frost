//! [`Listing`]: what a [`CompiledFunction`] tree holds, gathered for display,
//! such as by [`CompiledFunction::disassemble`].

use crate::bytecode::{Bytecode, CompiledFunction, GLOBAL_NAMES, OpDebugInfo};
use crate::{Arity, MapKey, SourceLocation, Value};

impl CompiledFunction {
    /// What this function and each function nested in it hold, gathered for
    /// display: [`disassemble`](Self::disassemble) renders it as text, and its
    /// `Serialize` form suits other tools.
    ///
    /// Malformed bytecode gathers without panicking: a reference to nothing
    /// gathers as absent.
    pub fn listing(&self) -> Listing<'_> {
        let mut functions = Vec::new();
        gather(self, Vec::new(), &mut functions);
        Listing { functions }
    }
}

/// What a [`CompiledFunction`] tree holds, from [`CompiledFunction::listing`].
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Listing<'a> {
    /// Each function of the tree, depth first, from the root.
    pub functions: Vec<ListedFunction<'a>>,
}

/// One function of a [`Listing`].
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ListedFunction<'a> {
    /// The index of each nested function leading from the root to this one;
    /// empty for the root.
    pub path: Vec<usize>,
    /// The function's name.
    pub name: &'a str,
    /// The number of arguments the function accepts.
    pub arity: Arity,
    /// How many of its slots, the first ones, are captures.
    pub num_captures: usize,
    /// Its local slots, in slot order.
    pub slots: Vec<ListedSlot<'a>>,
    /// Its constant pool.
    pub constants: &'a [Value],
    /// Its constant Map keys.
    pub keys: &'a [MapKey],
    /// The name of each function nested in it, in order; each is listed after
    /// this one, at this one's path plus its index.
    pub children: Vec<&'a str>,
    /// Its code.
    pub instructions: Vec<ListedInstruction<'a>>,
    /// The label of the end of the code, if a jump lands just past the last
    /// instruction, where the function returns.
    pub end_label: Option<usize>,
}

/// One local slot of a [`ListedFunction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ListedSlot<'a> {
    /// The binding's name.
    pub name: &'a str,
    /// Whether the slot holds a capture.
    pub capture: bool,
    /// Whether the binding is exported.
    pub exported: bool,
}

/// One instruction of a [`ListedFunction`].
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct ListedInstruction<'a> {
    /// Its index in the function's code.
    pub index: usize,
    /// Its label, if a jump lands on it. Labels number the jump targets in code order.
    pub label: Option<usize>,
    /// The instruction.
    pub op: Bytecode,
    /// What its operand refers to, if it refers to anything.
    /// [`Bytecode::Nop`] refers to nothing, and labels no target.
    pub reference: Option<Reference<'a>>,
    /// Its debug info, if the function has an entry for it.
    pub debug: Option<ListedDebugInfo>,
}

/// What an instruction's operand refers to. Where a malformed operand refers to
/// nothing, the part that would name it is `None`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum Reference<'a> {
    /// A local slot.
    Slot {
        /// The slot's index.
        index: u64,
        /// The binding's name.
        name: Option<&'a str>,
    },
    /// An entry of the constant pool.
    Constant {
        /// The constant's index.
        index: u64,
        /// The constant.
        value: Option<&'a Value>,
    },
    /// A constant Map key.
    Key {
        /// The key's index.
        index: u64,
        /// The key.
        key: Option<&'a MapKey>,
    },
    /// A global.
    Global {
        /// The global's index.
        index: u64,
        /// The global's name.
        name: Option<&'static str>,
    },
    /// A nested function, closed over.
    Child {
        /// The nested function's index.
        index: u32,
        /// The nested function's name.
        name: Option<&'a str>,
        /// How many captures the closure takes.
        captures: u32,
    },
    /// A jump's target.
    Jump {
        /// How many instructions the jump skips.
        offset: u64,
        /// Where it lands.
        target: Option<JumpTarget>,
    },
}

/// Where a jump lands, as a [`Reference::Jump`] carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct JumpTarget {
    /// The index of the instruction it lands at, which may be just past the
    /// last instruction, where the function returns.
    pub index: usize,
    /// The target's label.
    pub label: usize,
}

/// An instruction's debug info, as a [`ListedInstruction`] carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ListedDebugInfo {
    /// The instruction's entry in the function's debug info.
    pub op: OpDebugInfo,
    /// Where its [`span`](OpDebugInfo::span) starts.
    pub location: SourceLocation,
}

/// Gather `function`, at `path`, then each function nested in it, depth first.
fn gather<'a>(
    function: &'a CompiledFunction,
    path: Vec<usize>,
    functions: &mut Vec<ListedFunction<'a>>,
) {
    let labels = Labels::of(&function.code);
    let instructions = function
        .code
        .iter()
        .enumerate()
        .map(|(index, &op)| ListedInstruction {
            index,
            label: labels.at(index),
            op,
            reference: reference(function, &labels, index, op),
            debug: debug_info(function, index),
        })
        .collect();
    let slots = function
        .name_table
        .iter()
        .enumerate()
        .map(|(slot, entry)| ListedSlot {
            name: &entry.name,
            capture: slot < function.num_captures,
            exported: entry.exported,
        })
        .collect();
    functions.push(ListedFunction {
        path: path.clone(),
        name: &function.name,
        arity: function.arity,
        num_captures: function.num_captures,
        slots,
        constants: &function.constants,
        keys: &function.key_constants,
        children: function
            .child_fns
            .iter()
            .map(|child| child.name.as_str())
            .collect(),
        instructions,
        end_label: labels.at(function.code.len()),
    });
    for (index, child) in function.child_fns.iter().enumerate() {
        let child_path = path.iter().copied().chain([index]).collect();
        gather(child, child_path, functions);
    }
}

/// What `op`, at `index` in `function`, refers to.
fn reference<'a>(
    function: &'a CompiledFunction,
    labels: &Labels,
    index: usize,
    op: Bytecode,
) -> Option<Reference<'a>> {
    use Bytecode::*;

    let reference = match op {
        DefLocal(slot) | LoadLocal(slot) | ConsumeLocal(slot) => Reference::Slot {
            index: slot,
            name: entry(&function.name_table, slot).map(|entry| entry.name.as_str()),
        },
        LoadConst(constant) => Reference::Constant {
            index: constant,
            value: entry(&function.constants, constant),
        },
        HardIndexMap(key) | TestConstKey(key) | ExtractConstKey(key) => Reference::Key {
            index: key,
            key: entry(&function.key_constants, key),
        },
        LoadGlobal(global) => Reference::Global {
            index: global,
            name: entry(GLOBAL_NAMES, global).copied(),
        },
        CreateClosure { child, captures } => Reference::Child {
            index: child,
            name: entry(&function.child_fns, u64::from(child)).map(|child| child.name.as_str()),
            captures,
        },
        // At 0, `Jump` is `Nop`, which jumps nowhere.
        Jump(offset @ 1..)
        | JumpIfTrue(offset)
        | JumpIfFalse(offset)
        | PeekJumpIfTrue(offset)
        | PeekJumpIfFalse(offset) => Reference::Jump {
            offset,
            target: jump_target(&function.code, index, offset).map(|target| JumpTarget {
                index: target,
                label: labels
                    .at(target)
                    .expect("every jump target within the code is labeled"),
            }),
        },
        // Exhaustive, so that a new opcode decides what it refers to.
        Bytecode::Nop
        | PushNull
        | PushTrue
        | PushFalse
        | PushInt(_)
        | PushFloat(_)
        | PeekDown(_)
        | DropBelow(_)
        | Add
        | Subtract
        | Multiply
        | Divide
        | Modulus
        | CompareEqual
        | CompareNotEqual
        | CompareLessThan
        | CompareLessThanOrEqual
        | CompareGreaterThan
        | CompareGreaterThanOrEqual
        | LogicalNot
        | Negate
        | Concat(_)
        | Call(_)
        | TailCall(_)
        | DynTailCall
        | MakeArray(_)
        | MakeMap(_)
        | ExplodeArray(_)
        | SplitArray(_)
        | SoftIndexStructure
        | TestKey
        | ExtractKey
        | TypeTest(_)
        | TestArrayLenExact(_)
        | TestArrayLenAtLeast(_)
        | MarkStack
        | DropMark
        | RewindToMark
        | ProduceError
        | Import => return None,
    };
    Some(reference)
}

/// The debug info of the instruction at `index` in `function`, if it has an entry.
fn debug_info(function: &CompiledFunction, index: usize) -> Option<ListedDebugInfo> {
    let debug_info = function.debug_info.as_ref()?;
    let op = *debug_info.ops.get(index)?;
    Some(ListedDebugInfo {
        op,
        location: debug_info.lines.location(op.span.start),
    })
}

/// The labels of a function's jump targets, numbered in code order.
struct Labels(Vec<usize>);

impl Labels {
    fn of(code: &[Bytecode]) -> Self {
        let mut targets: Vec<usize> = code
            .iter()
            .enumerate()
            .filter_map(|(index, op)| match *op {
                // At 0, `Jump` is `Nop`, which jumps nowhere.
                Bytecode::Jump(offset @ 1..)
                | Bytecode::JumpIfTrue(offset)
                | Bytecode::JumpIfFalse(offset)
                | Bytecode::PeekJumpIfTrue(offset)
                | Bytecode::PeekJumpIfFalse(offset) => jump_target(code, index, offset),
                _ => None,
            })
            .collect();
        targets.sort_unstable();
        targets.dedup();
        Self(targets)
    }

    /// The label of the target at `index`, if a jump lands there.
    fn at(&self, index: usize) -> Option<usize> {
        self.0.binary_search(&index).ok()
    }
}

/// The entry of `items` at the operand `index`, if there is one.
fn entry<T>(items: &[T], index: u64) -> Option<&T> {
    usize::try_from(index)
        .ok()
        .and_then(|index| items.get(index))
}

/// Where a jump at `index` skipping `offset` instructions lands,
/// if within `code` or just past its end, where the function returns.
fn jump_target(code: &[Bytecode], index: usize, offset: u64) -> Option<usize> {
    usize::try_from(offset)
        .ok()
        .and_then(|offset| (index + 1).checked_add(offset))
        .filter(|&target| target <= code.len())
}
