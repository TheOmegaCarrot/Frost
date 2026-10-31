//! `Bytecode::stack_effect`, checked against what the VM actually does.
//!
//! Each case runs one instruction on the VM between a sentinel and a measurement,
//! and checks that the instruction left exactly the number of values its claimed
//! effect says. The measurement does not rely on the VM's debug assertions,
//! so it holds in release builds too:
//!
//! ```text
//! MarkStack
//! PAD ... PAD SENTINEL                values below the instruction's operands
//! <operands> <prelude> <op> <postlude>
//! MakeArray(operands + claimed change) gather what the instruction left
//! PeekDown(1)                         the value just below them
//! DefLocal(BELOW) DefLocal(LEFT)      keep both past the rewind
//! RewindToMark                        discard everything else
//! LoadLocal(BELOW) LoadLocal(LEFT) MakeArray(2)
//! ```
//!
//! If the claim is right, the value below is the sentinel. If it claims too many,
//! the gather reaches past the sentinel and the value below is a pad.
//! If it claims too few, the value below is one the instruction left.
//! An instruction that reaches deeper than its operands disturbs the sentinel.
//!
//! Operands are distinct values, and where reading the right depth matters, a case
//! also pins the values its instruction leaves, so reading one too deep or shallow shows.

use std::num::NonZeroU64;
use std::sync::Arc;

use frostlang::bytecode::{Bytecode, CompiledFunction, FormatVersion, NameEntry, StackEffect};
use frostlang::{
    Arity, FrostFloat, FrostType, HostComponent, ImporterBuilder, MapKey, RunError, Value, Vm,
};

use crate::common::global_slot;

use Bytecode::*;

// Local slots the measurement uses.
const BELOW: u64 = 0;
const LEFT: u64 = 1;
/// The local slot a case may use.
const CASE_SLOT: u64 = 2;

/// How many pads lie below the sentinel: more than any case's claim could be off by.
const PADS: usize = 16;

// Constants a case may load. No case loads `SENTINEL` or `PAD`.
const SENTINEL: u64 = 0;
const PAD: u64 = 1;
const MAP_K: u64 = 2;
const ARRAY_1_2: u64 = 3;
const STRING_A_B: u64 = 4;
const STRING_COMMA: u64 = 5;
const IMPORT_SPEC: u64 = 6;
const ARGS_1: u64 = 7;
const STRING_K: u64 = 8;

fn constants() -> Vec<Value> {
    vec![
        Value::from("sentinel"),
        Value::from("pad"),
        map_k(),
        Value::from(vec![Value::from(1), Value::from(2)]),
        Value::from("a,b"),
        Value::from(","),
        Value::from("app"),
        Value::from(vec![Value::from(1)]),
        Value::from("k"),
    ]
}

/// The [`MAP_K`] constant.
fn map_k() -> Value {
    Value::map([("k", Value::from(1))])
}

/// The key constant every constant-keyed case uses: `"k"`, present in the [`MAP_K`] constant.
const KEY_K: u64 = 0;

// Child functions a case may close over or call.
/// A closure of arity N, for N in 0..=2, that returns null.
const RETURN_NULL_ARITY: [u32; 3] = [0, 1, 2];
/// A closure with two captures, never called.
const TWO_CAPTURES: u32 = 3;

fn child_fns() -> Vec<Arc<CompiledFunction>> {
    let return_null = |arity: usize| {
        // Consume the callee and its arguments, then leave the result.
        let mut code = vec![Bytecode::Pop; arity + 1];
        code.push(PushNull);
        function("return_null", code, Arity::Exact(arity), Vec::new(), 0)
    };
    let two_captures = function(
        "two_captures",
        vec![Bytecode::Pop, PushNull],
        Arity::Exact(0),
        vec![slot("a"), slot("b")],
        2,
    );
    vec![return_null(0), return_null(1), return_null(2), two_captures]
}

fn function(
    name: &str,
    code: Vec<Bytecode>,
    arity: Arity,
    name_table: Vec<NameEntry>,
    num_captures: usize,
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: name.to_string(),
        origin: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table,
        num_captures,
        arity,
        debug_info: None,
    })
}

fn slot(name: &str) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported: false,
    }
}

/// `n` instructions pushing distinct Ints, 10 upward: the deepest is 10.
fn ints(n: u64) -> Vec<Bytecode> {
    (10..).take(as_usize(n)).map(PushInt).collect()
}

/// The values [`ints`] pushes, at the given indices.
fn int_values(indices: &[i64]) -> Vec<Value> {
    indices.iter().map(|&i| Value::from(10 + i)).collect()
}

/// A closure that returns null, of the given arity, to call.
fn return_null(arity: u64) -> Bytecode {
    CreateClosure {
        child: RETURN_NULL_ARITY[as_usize(arity)],
        captures: 0,
    }
}

fn as_usize(n: u64) -> usize {
    usize::try_from(n).unwrap()
}

fn as_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap()
}

fn global(name: &str) -> Bytecode {
    LoadGlobal(global_slot(name))
}

/// How to run one instruction.
struct Case {
    op: Bytecode,
    /// Instructions pushing exactly the operands `op` needs, each pushing one value.
    operands: Vec<Bytecode>,
    /// Instructions with no net stack effect, run just before `op`.
    prelude: Vec<Bytecode>,
    /// Instructions with no net stack effect, run just after `op`.
    postlude: Vec<Bytecode>,
    /// The values `op` should leave, bottom first, when the case pins them.
    leaves: Option<Vec<Value>>,
}

impl Case {
    fn new(op: Bytecode, operands: impl Into<Vec<Bytecode>>) -> Self {
        Self {
            op,
            operands: operands.into(),
            prelude: Vec::new(),
            postlude: Vec::new(),
            leaves: None,
        }
    }

    fn with_prelude(mut self, prelude: impl Into<Vec<Bytecode>>) -> Self {
        self.prelude = prelude.into();
        self
    }

    fn with_postlude(mut self, postlude: impl Into<Vec<Bytecode>>) -> Self {
        self.postlude = postlude.into();
        self
    }

    fn leaving(mut self, values: Vec<Value>) -> Self {
        self.leaves = Some(values);
        self
    }

    fn instructions(&self) -> impl Iterator<Item = &Bytecode> {
        self.operands
            .iter()
            .chain(&self.prelude)
            .chain([&self.op])
            .chain(&self.postlude)
    }
}

/// The operands and surroundings `op` needs to run to completion,
/// or `None` for an instruction that never completes.
///
/// The match is exhaustive, so a new opcode cannot compile until it has a case here.
/// Add an instance to [`instances`] as well; nothing enforces that.
fn case(op: Bytecode) -> Option<Case> {
    // Grouped by the operands the instruction needs.
    let case = match op {
        PushNull | PushTrue | PushFalse | PushInt(_) | PushFloat(_) | LoadConst(_)
        | LoadGlobal(_) | Jump(_) => Case::new(op, []),
        // Define the slot first, so it holds a value to read or release.
        LoadLocal(_) | ConsumeLocal(_) | ClearLocals { .. } => {
            Case::new(op, []).with_prelude([PushInt(7), DefLocal(CASE_SLOT)])
        }
        DefLocal(_) | LogicalNot | Negate | TypeTest(_) => Case::new(op, ints(1)),
        Add
        | Subtract
        | Multiply
        | Divide
        | Modulus
        | CompareEqual
        | CompareNotEqual
        | CompareLessThan
        | CompareLessThanOrEqual
        | CompareGreaterThan
        | CompareGreaterThanOrEqual => Case::new(op, ints(2)),
        PeekDown(n) | DropBelow(n) => Case::new(op, ints(n + 1)),
        Concat(n) => Case::new(op, ints(n.get())),

        // `instances` also runs each with a falsy operand.
        JumpIfTrue(_) | JumpIfFalse(_) | PeekJumpIfTrue(_) | PeekJumpIfFalse(_) => {
            Case::new(op, [PushTrue])
        }

        // `instances` also runs calls with native callees.
        // In the bottom frame, which `measure` runs in, a closure tail call
        // runs as an ordinary call and completes, so it can be measured too.
        Call(argc) | TailCall(argc) => {
            Case::new(op, [vec![return_null(argc)], ints(argc)].concat())
        }
        DynTailCall => Case::new(op, [return_null(1), LoadConst(ARGS_1)]),
        CreateClosure { captures, .. } => Case::new(op, ints(captures.into())),

        MakeArray(n) => Case::new(op, ints(n)),
        MakeMap(pairs) => Case::new(
            op,
            [LoadConst(STRING_COMMA), PushInt(10)].repeat(as_usize(pairs)),
        ),
        ExplodeArray(_) | SplitArray(_) | TestArrayLenExact(_) | TestArrayLenAtLeast(_) => {
            Case::new(op, [LoadConst(ARRAY_1_2)])
        }

        SoftIndexStructure => Case::new(op, [LoadConst(ARRAY_1_2), PushInt(0)]),
        HardIndexMap(_) | TestConstKey(_) | ExtractConstKey(_) => Case::new(op, [LoadConst(MAP_K)]),
        TestKey | ExtractKey => Case::new(op, [LoadConst(MAP_K), LoadConst(STRING_K)]),

        // Balance the marks, so the measurement's own rewind takes its own mark.
        MarkStack => Case::new(op, []).with_postlude([DropMark]),
        DropMark => Case::new(op, []).with_prelude([MarkStack]),
        RewindToMark => unreachable!("RewindToMark has no fixed effect to measure"),

        ProduceError => return None,
        Import => Case::new(op, [LoadConst(IMPORT_SPEC)]),
    };
    Some(case)
}

fn cased(op: Bytecode) -> Case {
    case(op).unwrap_or_else(|| panic!("{op:?} never completes"))
}

/// At least one instance of every opcode, each with the operands to run it.
fn instances() -> Vec<Case> {
    let simple = [
        PushNull,
        PushTrue,
        PushFalse,
        PushInt(1),
        PushFloat(FrostFloat::new(1.5).unwrap()),
        DefLocal(CASE_SLOT),
        LoadLocal(CASE_SLOT),
        ConsumeLocal(CASE_SLOT),
        ClearLocals {
            first: CASE_SLOT as u32,
            count: 1,
        },
        ClearLocals {
            first: CASE_SLOT as u32,
            count: 0,
        },
        LoadConst(STRING_K),
        LoadGlobal(global_slot("type")),
        Add,
        Subtract,
        Multiply,
        Divide,
        Modulus,
        CompareEqual,
        CompareNotEqual,
        CompareLessThan,
        CompareLessThanOrEqual,
        CompareGreaterThan,
        CompareGreaterThanOrEqual,
        LogicalNot,
        Negate,
        Concat(NonZeroU64::new(1).unwrap()),
        Concat(NonZeroU64::new(3).unwrap()),
        Jump(0),
        Call(0),
        Call(2),
        TailCall(0),
        TailCall(1),
        TailCall(2),
        DynTailCall,
        CreateClosure {
            child: RETURN_NULL_ARITY[0],
            captures: 0,
        },
        CreateClosure {
            child: TWO_CAPTURES,
            captures: 2,
        },
        MakeArray(0),
        MakeArray(3),
        MakeMap(0),
        MakeMap(1),
        ExplodeArray(2),
        SplitArray(0),
        SplitArray(1),
        SoftIndexStructure,
        HardIndexMap(KEY_K),
        TypeTest(FrostType::INT),
        MarkStack,
        DropMark,
        Import,
    ]
    .map(cased);

    let array_1_2 = Value::from(vec![Value::from(1), Value::from(2)]);
    // The instructions that read deeper than they pop, or reach below the top.
    let pinned = [
        cased(PeekDown(0)).leaving(int_values(&[0, 0])),
        cased(PeekDown(2)).leaving(int_values(&[0, 1, 2, 0])),
        cased(DropBelow(0)).leaving(Vec::new()),
        cased(DropBelow(2)).leaving(int_values(&[1, 2])),
        cased(TestKey).leaving(vec![map_k(), Value::from("k"), Value::from(true)]),
        cased(ExtractKey).leaving(vec![map_k(), Value::from(1)]),
        cased(TestConstKey(KEY_K)).leaving(vec![map_k(), Value::from(true)]),
        cased(ExtractConstKey(KEY_K)).leaving(vec![map_k(), Value::from(1)]),
        cased(TestArrayLenExact(2)).leaving(vec![array_1_2.clone(), Value::from(true)]),
        cased(TestArrayLenAtLeast(1)).leaving(vec![array_1_2, Value::from(true)]),
    ];

    // A jump of 0 lands on the next instruction whether or not it is taken,
    // so each outcome runs into the measurement.
    let jumps = [
        JumpIfTrue(0),
        JumpIfFalse(0),
        PeekJumpIfTrue(0),
        PeekJumpIfFalse(0),
    ]
    .into_iter()
    .flat_map(|jump| {
        [PushTrue, PushFalse].map(|tested| {
            let left = match jump {
                PeekJumpIfTrue(_) | PeekJumpIfFalse(_) => vec![Value::from(tested == PushTrue)],
                _ => Vec::new(),
            };
            Case::new(jump, [tested]).leaving(left)
        })
    });

    // Calls with native callees: their effect is the same as with closures.
    let native_calls = [
        Case::new(Call(1), [global("type"), PushInt(10)]),
        Case::new(TailCall(1), [global("type"), PushInt(10)]),
        Case::new(
            TailCall(2),
            [
                global("split"),
                LoadConst(STRING_A_B),
                LoadConst(STRING_COMMA),
            ],
        ),
        Case::new(DynTailCall, [global("type"), LoadConst(ARGS_1)]),
    ];

    simple
        .into_iter()
        .chain(pinned)
        .chain(jumps)
        .chain(native_calls)
        .collect()
}

fn effect(op: Bytecode) -> StackEffect {
    op.stack_effect()
        .unwrap_or_else(|| panic!("{op:?} should have a fixed stack effect"))
}

/// Runs `case`, returning the value just below the values its instruction claims to leave,
/// and those values as an Array.
fn measure(case: &Case) -> (Value, Value) {
    let claimed = effect(case.op);
    let left = (as_u64(case.operands.len()) + claimed.pushes())
        .checked_sub(claimed.pops())
        .unwrap_or_else(|| panic!("{:?} claims to pop more than its operands", case.op));

    let code: Vec<Bytecode> = [
        vec![Bytecode::Pop, MarkStack],
        vec![LoadConst(PAD); PADS],
        vec![LoadConst(SENTINEL)],
        case.instructions().copied().collect(),
        vec![
            MakeArray(left),
            PeekDown(1),
            DefLocal(BELOW),
            DefLocal(LEFT),
            RewindToMark,
            LoadLocal(BELOW),
            LoadLocal(LEFT),
            MakeArray(2),
        ],
    ]
    .concat();

    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<stack effect>".to_string(),
        origin: None,
        code,
        child_fns: child_fns(),
        constants: constants(),
        key_constants: vec![MapKey::from("k")],
        name_table: vec![slot("below"), slot("left"), slot("case")],
        num_captures: 0,
        arity: Arity::Exact(0),
        debug_info: None,
    });
    let closure = program.assert_well_formed().into_closure().unwrap();
    let importer = ImporterBuilder::new()
        .with_component(HostComponent::new("app", Value::from(1)).unwrap())
        .unwrap()
        .build();
    let result = Vm::factory()
        .with_importer(importer)
        .build(closure)
        .run()
        .map_err(RunError::into_error)
        .unwrap_or_else(|err| panic!("{:?} failed to run: {err}", case.op));
    let pair: Vec<Value> = result
        .tail()
        .as_array()
        .expect("the measurement yields an Array")
        .iter()
        .cloned()
        .collect();
    let [below, left] = <[Value; 2]>::try_from(pair).expect("the measurement yields a pair");
    (below, left)
}

#[test]
fn no_case_loads_the_sentinel_or_a_pad() {
    for case in instances() {
        assert!(
            !case
                .instructions()
                .any(|op| matches!(op, LoadConst(SENTINEL | PAD))),
            "{:?}'s case loads the sentinel or a pad, so the measurement could mistake it",
            case.op
        );
    }
}

#[test]
fn every_effect_matches_the_vm() {
    for case in instances() {
        assert_eq!(
            effect(case.op).reads(),
            as_u64(case.operands.len()),
            "{:?} should read exactly the operands it is given",
            case.op
        );
        let (below, left) = measure(&case);
        assert_eq!(
            below,
            Value::from("sentinel"),
            "{:?} left a different number of values than its claimed effect {:?}, \
             or disturbed values below its operands (gathered {left:?})",
            case.op,
            effect(case.op)
        );
        if let Some(leaves) = &case.leaves {
            assert_eq!(
                &left,
                &Value::from(leaves.clone()),
                "{:?} left the wrong values: it read the wrong depth",
                case.op
            );
        }
    }
}

#[test]
fn every_effect_reads_at_least_what_it_pops() {
    for case in instances() {
        let effect = effect(case.op);
        assert!(
            effect.reads() >= effect.pops(),
            "{:?} pops more than it reads: {effect:?}",
            case.op
        );
    }
}

#[test]
fn only_rewind_to_mark_has_no_fixed_effect() {
    assert_eq!(RewindToMark.stack_effect(), None);
    for case in instances() {
        assert!(
            case.op.stack_effect().is_some(),
            "{:?} should have a fixed effect",
            case.op
        );
    }
    assert!(
        ProduceError.stack_effect().is_some(),
        "ProduceError should have a fixed effect, though it never completes"
    );
}

/// `ProduceError` never completes, so it cannot be measured; its effect follows
/// its documented `( x -- )`.
#[test]
fn produce_error_pops_its_operand() {
    let effect = effect(ProduceError);
    assert_eq!((effect.reads(), effect.pops(), effect.pushes()), (1, 1, 0));
}

/// Effects whose counts come from the instruction's operand.
#[test]
fn counted_effects_follow_their_operand() {
    let triple = |op: Bytecode| {
        let effect = effect(op);
        (effect.reads(), effect.pops(), effect.pushes())
    };
    for n in [0, 1, 5] {
        assert_eq!(triple(PeekDown(n)), (n + 1, 0, 1), "PeekDown({n})");
        assert_eq!(triple(DropBelow(n)), (n + 1, n + 1, n), "DropBelow({n})");
        assert_eq!(triple(Call(n)), (n + 1, n + 1, 1), "Call({n})");
        assert_eq!(triple(TailCall(n)), (n + 1, n + 1, 1), "TailCall({n})");
        assert_eq!(triple(MakeArray(n)), (n, n, 1), "MakeArray({n})");
        assert_eq!(triple(MakeMap(n)), (2 * n, 2 * n, 1), "MakeMap({n})");
        assert_eq!(triple(ExplodeArray(n)), (1, 1, n), "ExplodeArray({n})");
        let captures = u32::try_from(n).unwrap();
        assert_eq!(
            triple(CreateClosure { child: 0, captures }),
            (n, n, 1),
            "CreateClosure with {n} captures"
        );
    }
    for n in [1, 4] {
        let count = NonZeroU64::new(n).unwrap();
        assert_eq!(triple(Concat(count)), (n, n, 1), "Concat({n})");
    }
}

#[test]
fn apply_gives_the_height_after() {
    let call_2 = effect(Call(2));
    assert_eq!(call_2.apply(3), Some(1), "exactly the callee and arguments");
    assert_eq!(call_2.apply(10), Some(8), "with values beneath");
    assert_eq!(
        call_2.apply(2),
        None,
        "too few values for the callee and arguments"
    );

    let peek_1 = effect(PeekDown(1));
    assert_eq!(peek_1.apply(2), Some(3), "PeekDown(1) on two values");
    assert_eq!(peek_1.apply(1), None, "reads below what it pops");

    assert_eq!(
        effect(PushNull).apply(0),
        Some(1),
        "an empty stack suffices"
    );
}
