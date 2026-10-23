//! `CompiledFunction::listing`: what a function tree holds, gathered as data.
//!
//! The functions here are written by hand, so the tests pin what the listing
//! gathers, not what the compiler emits; one compiles, to check the compiler's
//! debug info reaches the listing.

use std::sync::Arc;

use frostlang::bytecode::{
    Bytecode, CompiledFunction, DebugInfo, FormatVersion, JumpTarget, ListedDebugInfo,
    ListedFunction, NameEntry, OpDebugInfo, Reference, SourceLines, Span, closure_function,
};
use frostlang::compile::{CompilerOptions, OptimizationOptions, compile_program};
use frostlang::{Arity, MapKey, Value};

use crate::common::global_slot;

use Bytecode::*;

/// A function named `name` running `code`, taking no arguments, with nothing
/// else: no slots, pools, or children.
fn function(name: &str, code: Vec<Bytecode>) -> CompiledFunction {
    CompiledFunction {
        version: FormatVersion,
        name: name.to_string(),
        origin: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
        debug_info: None,
    }
}

fn slot(name: &str, exported: bool) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported,
    }
}

/// The reference of each instruction of `function`.
fn references<'a>(function: &ListedFunction<'a>) -> Vec<Option<Reference<'a>>> {
    function
        .instructions
        .iter()
        .map(|instruction| instruction.reference)
        .collect()
}

#[test]
fn functions_are_listed_depth_first_with_their_paths() {
    let mut first = function("first", vec![PushNull]);
    first.child_fns = vec![Arc::new(function("inner", vec![PushNull]))];
    let mut main = function("<main>", vec![PushNull]);
    main.child_fns = vec![
        Arc::new(first),
        Arc::new(function("second", vec![PushNull])),
    ];

    let listing = main.listing();
    let shape: Vec<(&str, Vec<usize>, Vec<&str>)> = listing
        .functions
        .iter()
        .map(|function| {
            (
                function.name,
                function.path.clone(),
                function.children.clone(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            ("<main>", vec![], vec!["first", "second"]),
            ("first", vec![0], vec!["inner"]),
            ("inner", vec![0, 0], vec![]),
            ("second", vec![1], vec![]),
        ],
        "each function after its parent, before its parent's later children"
    );
}

#[test]
fn a_function_lists_its_metadata_and_pools() {
    let mut main = function("<main>", vec![PushNull]);
    main.arity = Arity::AtLeast(1);
    main.num_captures = 1;
    main.name_table = vec![slot("captured", false), slot("shown", true)];
    main.constants = vec![Value::from("text")];
    main.key_constants = vec![MapKey::from("key")];

    let listing = main.listing();
    let listed = &listing.functions[0];
    assert_eq!(listed.arity, Arity::AtLeast(1));
    assert_eq!(listed.num_captures, 1);
    let slots: Vec<(&str, bool, bool)> = listed
        .slots
        .iter()
        .map(|slot| (slot.name, slot.capture, slot.exported))
        .collect();
    assert_eq!(
        slots,
        [("captured", true, false), ("shown", false, true)],
        "a slot below the capture count is a capture"
    );
    assert_eq!(listed.constants, [Value::from("text")]);
    assert_eq!(listed.keys, [MapKey::from("key")]);
}

#[test]
fn each_kind_of_operand_refers_to_what_it_indexes() {
    let print = global_slot("print");
    let mut main = function(
        "<main>",
        vec![
            LoadLocal(0),
            LoadConst(0),
            HardIndexMap(0),
            LoadGlobal(print),
            CreateClosure {
                child: 0,
                captures: 0,
            },
            JumpIfFalse(0),
            PushInt(7),
        ],
    );
    main.name_table = vec![slot("x", false)];
    main.constants = vec![Value::from("text")];
    main.key_constants = vec![MapKey::from("key")];
    main.child_fns = vec![Arc::new(function("child", vec![PushNull]))];

    let listing = main.listing();
    let text = Value::from("text");
    let key = MapKey::from("key");
    let expected = [
        Some(Reference::Slot {
            index: 0,
            name: Some("x"),
        }),
        Some(Reference::Constant {
            index: 0,
            value: Some(&text),
        }),
        Some(Reference::Key {
            index: 0,
            key: Some(&key),
        }),
        Some(Reference::Global {
            index: print,
            name: Some("print"),
        }),
        Some(Reference::Child {
            index: 0,
            name: Some("child"),
            captures: 0,
        }),
        Some(Reference::Jump {
            offset: 0,
            target: Some(JumpTarget { index: 6, label: 0 }),
        }),
        None,
    ];
    assert_eq!(
        references(&listing.functions[0]),
        expected,
        "each operand refers to its entry; a literal refers to nothing"
    );
}

#[test]
fn a_malformed_reference_gathers_as_absent() {
    let main = function(
        "<main>",
        vec![
            LoadLocal(9),
            LoadConst(9),
            HardIndexMap(9),
            LoadGlobal(u64::MAX),
            CreateClosure {
                child: 9,
                captures: 2,
            },
            Jump(99),
        ],
    );
    let listing = main.listing();
    let expected: [Option<Reference>; 6] = [
        Some(Reference::Slot {
            index: 9,
            name: None,
        }),
        Some(Reference::Constant {
            index: 9,
            value: None,
        }),
        Some(Reference::Key {
            index: 9,
            key: None,
        }),
        Some(Reference::Global {
            index: u64::MAX,
            name: None,
        }),
        Some(Reference::Child {
            index: 9,
            name: None,
            captures: 2,
        }),
        Some(Reference::Jump {
            offset: 99,
            target: None,
        }),
    ];
    assert_eq!(
        references(&listing.functions[0]),
        expected,
        "the index stays, and what it would refer to is absent"
    );
}

#[test]
fn labels_number_jump_targets_in_code_order() {
    // Index 1 jumps to 4 (the end), index 2 jumps to 3.
    let main = function(
        "<main>",
        vec![PushTrue, JumpIfFalse(2), PeekJumpIfTrue(0), PushNull],
    );
    let listing = main.listing();
    let listed = &listing.functions[0];
    let labels: Vec<Option<usize>> = listed
        .instructions
        .iter()
        .map(|instruction| instruction.label)
        .collect();
    assert_eq!(
        labels,
        [None, None, None, Some(0)],
        "the target within the code is label 0"
    );
    assert_eq!(
        listed.end_label,
        Some(1),
        "the target just past the end is label 1"
    );
}

/// `Nop` is a `Jump` that skips nothing: it refers to nothing and labels
/// nothing, while a `Jump` that skips something still does both.
#[test]
fn a_nop_is_not_a_jump() {
    let main = function("<main>", vec![Bytecode::Nop, Jump(1), PushNull, PushNull]);
    let listing = main.listing();
    let listed = &listing.functions[0];
    assert_eq!(
        references(listed),
        [
            None,
            Some(Reference::Jump {
                offset: 1,
                target: Some(JumpTarget { index: 3, label: 0 }),
            }),
            None,
            None,
        ],
        "only the Jump that skips an instruction refers to a target"
    );
    let labels: Vec<Option<usize>> = listed
        .instructions
        .iter()
        .map(|instruction| instruction.label)
        .collect();
    assert_eq!(
        labels,
        [None, None, None, Some(0)],
        "the instruction after the Nop is not labeled"
    );
}

#[test]
fn an_instruction_carries_its_entry_of_debug_info_and_where_its_span_starts() {
    let entries = [
        OpDebugInfo {
            span: Span { start: 0, end: 3 },
            stack_height: Some(1),
        },
        OpDebugInfo {
            span: Span { start: 4, end: 8 },
            stack_height: None,
        },
    ];
    let mut main = function("<main>", vec![PushNull, PushNull, PushNull]);
    // The third instruction has no entry.
    main.debug_info = Some(DebugInfo {
        ops: entries.to_vec(),
        lines: Arc::new(SourceLines::new("one\ntwo")),
    });

    let listing = main.listing();
    let debug: Vec<Option<ListedDebugInfo>> = listing.functions[0]
        .instructions
        .iter()
        .map(|instruction| instruction.debug)
        .collect();
    let at = |offset| main.debug_info.as_ref().unwrap().lines.location(offset);
    assert_eq!(
        debug,
        [
            Some(ListedDebugInfo {
                op: entries[0],
                location: at(0),
            }),
            Some(ListedDebugInfo {
                op: entries[1],
                location: at(4),
            }),
            None,
        ],
        "each instruction carries its entry, located where its span starts; \
         one past the entries carries none"
    );
    assert_eq!(
        (at(4).line(), at(4).column()),
        (2, 1),
        "the second span starts the second line"
    );
}

#[test]
fn an_instruction_carries_no_debug_info_when_the_function_has_none() {
    let main = function("<main>", vec![PushNull]);
    let listing = main.listing();
    assert_eq!(
        listing.functions[0].instructions[0].debug, None,
        "without debug info, no instruction carries any"
    );
}

#[test]
fn a_compiled_function_with_debug_info_carries_it_on_every_instruction() {
    let options = CompilerOptions::new()
        .with_optimization(OptimizationOptions::NONE)
        .with_debug_info(true);
    let closure = compile_program("test.frst", "print(1)", options)
        .expect("the source compiles")
        .code
        .into_closure()
        .expect("the source captures nothing");
    let listing = closure_function(&closure).listing();
    assert!(
        listing.functions[0]
            .instructions
            .iter()
            .all(|instruction| instruction.debug.is_some()),
        "the compiler gives every instruction an entry"
    );
}

#[test]
fn a_listing_dumps_as_json() {
    let mut main = function("<main>", vec![LoadConst(0), JumpIfFalse(0)]);
    main.constants = vec![Value::from("text")];
    let json = serde_json::to_value(main.listing()).expect("a listing serializes");
    let function = &json["functions"][0];
    assert_eq!(function["name"], "<main>");
    assert_eq!(function["path"], serde_json::json!([]));
    assert_eq!(function["constants"], serde_json::json!(["text"]));
    assert_eq!(
        function["instructions"][0]["reference"],
        serde_json::json!({"Constant": {"index": 0, "value": "text"}})
    );
    assert_eq!(
        function["instructions"][1]["reference"],
        serde_json::json!({"Jump": {"offset": 0, "target": {"index": 2, "label": 0}}})
    );
    assert_eq!(function["end_label"], 0, "the jump lands at the end");
}
