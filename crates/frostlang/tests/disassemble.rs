//! `CompiledFunction::disassemble`: the listing of a function tree.
//!
//! Every function here is written by hand rather than compiled, so the tests
//! pin the listing's format, not what the compiler emits.

use std::num::NonZeroU64;
use std::sync::Arc;

use frostlang::bytecode::{
    Bytecode, CompiledFunction, DebugInfo, FormatVersion, NameEntry, OpDebugInfo, SourceLines, Span,
};
use frostlang::{Arity, FrostFloat, FrostType, MapKey, Value};

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

fn listing(function: &CompiledFunction) -> String {
    function.disassemble().to_string()
}

#[test]
fn a_function_with_nothing_but_code() {
    let listed = listing(&function("<main>", vec![PushInt(1), PushNull, Add]));
    let expected = r"function <main>
  arity 0, 0 captures
  code
       0  PushInt   1
       1  PushNull
       2  Add
";
    assert_eq!(listed, expected);
}

#[test]
fn slots_are_listed_with_what_kind_each_is() {
    let mut main = function("<main>", vec![LoadLocal(0), DefLocal(1), LoadLocal(2)]);
    main.name_table = vec![
        slot("greeting", false),
        slot("total", true),
        slot("n", false),
    ];
    main.num_captures = 1;
    let expected = r"function <main>
  arity 0, 1 capture
  slots
       0  greeting  capture
       1  total     export
       2  n
  code
       0  LoadLocal  0  ; greeting
       1  DefLocal   1  ; total
       2  LoadLocal  2  ; n
";
    assert_eq!(listing(&main), expected);
}

#[test]
fn a_long_slot_name_stays_apart_from_its_markers() {
    let mut main = function("<main>", vec![PushNull]);
    main.name_table = vec![slot("continuation_prompt", true), slot("n", true)];
    let expected = r"function <main>
  arity 0, 0 captures
  slots
       0  continuation_prompt  export
       1  n                    export
  code
       0  PushNull
";
    assert_eq!(
        listing(&main),
        expected,
        "the markers sit two spaces past the longest name, in one column"
    );
}

#[test]
fn constants_are_listed_in_full_and_shown_beside_their_use_cut_short() {
    let long = Value::from_iter((0..30).map(Value::Int));
    let mut main = function("<main>", vec![LoadConst(0), LoadConst(1)]);
    main.constants = vec![Value::from("hello"), long];
    let expected = r#"function <main>
  arity 0, 0 captures
  constants
       0  "hello"
       1  [ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29 ]
  code
       0  LoadConst  0  ; "hello"
       1  LoadConst  1  ; [ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 1...
"#;
    assert_eq!(listing(&main), expected);
}

#[test]
fn keys_are_shown_as_source_would_access_them() {
    let mut main = function(
        "<main>",
        vec![
            HardIndexMap(0),
            TestConstKey(1),
            ExtractConstKey(2),
            HardIndexMap(3),
        ],
    );
    main.key_constants = vec![
        MapKey::from("name"),
        MapKey::from("two words"),
        MapKey::Int(1),
        MapKey::from("if"),
    ];
    let expected = r#"function <main>
  arity 0, 0 captures
  keys
       0  "name"
       1  "two words"
       2  1
       3  "if"
  code
       0  HardIndexMap     0  ; .name
       1  TestConstKey     1  ; ["two words"]
       2  ExtractConstKey  2  ; [1]
       3  HardIndexMap     3  ; ["if"]
"#;
    assert_eq!(listing(&main), expected);
}

#[test]
fn globals_are_named() {
    let print = frostlang::bytecode::GLOBAL_NAMES
        .iter()
        .position(|&name| name == "print")
        .map(|slot| slot as u64)
        .expect("a `print` global");
    let listed = listing(&function(
        "<main>",
        vec![LoadGlobal(print), PushNull, Call(1)],
    ));
    assert!(
        listed.contains(&format!("LoadGlobal  {print}  ; print")),
        "{listed}"
    );
}

#[test]
fn jumps_name_labels_in_code_order() {
    let main = function(
        "<main>",
        vec![
            PushTrue,
            JumpIfFalse(3), // to 5
            PushInt(1),
            PeekJumpIfTrue(3), // to 7, the end
            Jump(1),           // to 6
            PushInt(2),
            PushInt(3),
        ],
    );
    let expected = r"function <main>
  arity 0, 0 captures
  code
       0  PushTrue
       1  JumpIfFalse     L0  ; +3
       2  PushInt         1
       3  PeekJumpIfTrue  L2  ; +3
       4  Jump            L1  ; +1
  L0:
       5  PushInt         2
  L1:
       6  PushInt         3
  L2:
       7  (end)
";
    assert_eq!(listing(&main), expected);
}

#[test]
fn other_operands_are_shown_plainly() {
    let float = FrostFloat::new(2.5).unwrap();
    let main = function(
        "<main>",
        vec![
            PushFloat(float),
            Concat(NonZeroU64::new(2).unwrap()),
            TypeTest(FrostType::Int | FrostType::Float),
            TestArrayLenAtLeast(2),
            PeekDown(1),
            TailCall(0),
        ],
    );
    let expected = r"function <main>
  arity 0, 0 captures
  code
       0  PushFloat            2.5
       1  Concat               2
       2  TypeTest             Int | Float
       3  TestArrayLenAtLeast  2
       4  PeekDown             1
       5  TailCall             0
";
    assert_eq!(listing(&main), expected);
}

/// `Dup`, `Pop`, and `Nop` list by name, with no operand and, for `Nop`, no
/// label after it; with any other operand, their opcodes list as usual.
#[test]
fn aliases_list_by_their_own_names() {
    let main = function(
        "<main>",
        vec![
            Bytecode::Dup,
            PeekDown(1),
            Bytecode::Pop,
            DropBelow(2),
            Bytecode::Nop,
            Jump(1), // to 7
            PushNull,
            PushNull,
        ],
    );
    let expected = r"function <main>
  arity 0, 0 captures
  code
       0  Dup
       1  PeekDown   1
       2  Pop
       3  DropBelow  2
       4  Nop
       5  Jump       L0  ; +1
       6  PushNull
  L0:
       7  PushNull
";
    assert_eq!(listing(&main), expected);
}

#[test]
fn nested_functions_follow_depth_first() {
    let mut grandchild = function("inner", vec![PushNull]);
    grandchild.num_captures = 1;
    grandchild.name_table = vec![slot("x", false)];
    let mut first = function(
        "first",
        vec![CreateClosure {
            child: 0,
            captures: 1,
        }],
    );
    first.child_fns = vec![Arc::new(grandchild)];
    first.arity = Arity::AtLeast(1);
    let mut second = function("second", vec![PushNull]);
    second.arity = Arity::Between(1, 3);
    let mut main = function(
        "<main>",
        vec![
            CreateClosure {
                child: 0,
                captures: 0,
            },
            CreateClosure {
                child: 1,
                captures: 0,
            },
        ],
    );
    main.child_fns = vec![Arc::new(first), Arc::new(second)];
    let expected = r"function <main>
  arity 0, 0 captures
  children
       0  first (child 0)
       1  second (child 1)
  code
       0  CreateClosure  0  ; first, 0 captures
       1  CreateClosure  1  ; second, 0 captures

function first (child 0)
  arity 1 or more, 0 captures
  children
       0  inner (child 0.0)
  code
       0  CreateClosure  0  ; inner, 1 capture

function inner (child 0.0)
  arity 0, 1 capture
  slots
       0  x  capture
  code
       0  PushNull

function second (child 1)
  arity 1 to 3, 0 captures
  code
       0  PushNull
";
    assert_eq!(listing(&main), expected);
}

#[test]
fn malformed_references_list_without_panicking() {
    // Every operand here refers to nothing: no slot, constant, key, global,
    // child, or instruction.
    let main = function(
        "<main>",
        vec![
            LoadLocal(0),
            LoadConst(0),
            HardIndexMap(0),
            LoadGlobal(u64::MAX),
            CreateClosure {
                child: 0,
                captures: 0,
            },
            Jump(99),
        ],
    );
    let expected = r"function <main>
  arity 0, 0 captures
  code
       0  LoadLocal      0                     ; ?
       1  LoadConst      0                     ; ?
       2  HardIndexMap   0                     ; ?
       3  LoadGlobal     18446744073709551615  ; ?
       4  CreateClosure  0                     ; ?, 0 captures
       5  Jump           ?                     ; +99, past the end
";
    assert_eq!(listing(&main), expected);
}

#[test]
fn color_is_off_unless_asked_for() {
    let main = function("<main>", vec![PushTrue, JumpIfFalse(0), PushNull]);
    let plain = main.disassemble().to_string();
    assert!(!plain.contains('\x1b'), "{plain:?}");
    let colored = main.disassemble().with_color(true).to_string();
    assert!(colored.contains("\x1b["), "{colored:?}");
    // Coloring changes nothing but the escape codes.
    let stripped = strip_escapes(&colored);
    assert_eq!(stripped, plain);
}

// ============================================================
// Debug info
// ============================================================

/// `function` with debug info against `source`: for each instruction, the
/// span starting at the given offset (one byte long) and the given height.
fn with_debug_info(
    mut function: CompiledFunction,
    source: &str,
    ops: &[(u32, Option<u32>)],
) -> CompiledFunction {
    function.debug_info = Some(DebugInfo {
        ops: ops
            .iter()
            .map(|&(start, stack_height)| OpDebugInfo {
                span: Span {
                    start,
                    end: start + 1,
                },
                stack_height,
            })
            .collect(),
        lines: Arc::new(SourceLines::new(source)),
    });
    function
}

#[test]
fn debug_info_shows_each_instructions_location_and_entry_height() {
    // `1` is at 1:1, `null` at 2:3, `+` at 12:1.
    let source = format!("1\n  null{}+", "\n".repeat(10));
    let main = with_debug_info(
        function("<main>", vec![PushInt(1), PushNull, Add]),
        &source,
        &[(0, Some(1)), (4, Some(9)), (18, Some(10))],
    );
    let expected = r"function <main>
  arity 0, 0 captures
  code (line:column, stack height)
       0  1:1    1  PushInt   1
       1  2:3    9  PushNull
       2  12:1  10  Add
";
    assert_eq!(
        listing(&main),
        expected,
        "the location column is as wide as its widest, and heights align right"
    );
}

#[test]
fn an_unknown_height_shows_as_a_dash_and_a_missing_entry_as_blanks() {
    let main = with_debug_info(
        function("<main>", vec![MarkStack, RewindToMark, PushNull]),
        "abc",
        // The last instruction has no entry.
        &[(0, Some(1)), (1, None)],
    );
    let expected = r"function <main>
  arity 0, 0 captures
  code (line:column, stack height)
       0  1:1  1  MarkStack
       1  1:2  -  RewindToMark
       2          PushNull
";
    assert_eq!(listing(&main), expected);
}

#[test]
fn a_function_without_debug_info_lists_as_before_beside_one_with_it() {
    let child = function("child", vec![PushNull]);
    let mut main = with_debug_info(
        function(
            "<main>",
            vec![
                CreateClosure {
                    child: 0,
                    captures: 0,
                },
                Bytecode::Pop,
            ],
        ),
        "fn -> null",
        &[(0, Some(1)), (0, Some(2))],
    );
    main.child_fns = vec![Arc::new(child)];
    let expected = r"function <main>
  arity 0, 0 captures
  children
       0  child (child 0)
  code (line:column, stack height)
       0  1:1  1  CreateClosure  0  ; child, 0 captures
       1  1:1  2  Pop

function child (child 0)
  arity 0, 0 captures
  code
       0  PushNull
";
    assert_eq!(listing(&main), expected);
}

#[test]
fn labels_and_the_end_align_with_debug_info() {
    // Index 1 jumps to 4 (the end), index 2 jumps to 3.
    let main = with_debug_info(
        function(
            "<main>",
            vec![PushTrue, JumpIfFalse(2), PeekJumpIfTrue(0), PushNull],
        ),
        "abcd",
        &[(0, Some(1)), (1, Some(2)), (2, Some(1)), (3, Some(1))],
    );
    let expected = r"function <main>
  arity 0, 0 captures
  code (line:column, stack height)
       0  1:1  1  PushTrue
       1  1:2  2  JumpIfFalse     L1  ; +2
       2  1:3  1  PeekJumpIfTrue  L0  ; +0
  L0:
       3  1:4  1  PushNull
  L1:
       4          (end)
";
    assert_eq!(
        listing(&main),
        expected,
        "labels sit above their instructions, and the end under the opcodes"
    );
}

#[test]
fn debug_info_is_colored_like_the_rest() {
    let main = with_debug_info(
        function("<main>", vec![PushTrue, JumpIfFalse(0), PushNull]),
        "abc",
        &[(0, Some(1)), (1, Some(2)), (2, Some(1))],
    );
    let plain = main.disassemble().to_string();
    let colored = main.disassemble().with_color(true).to_string();
    assert_eq!(
        strip_escapes(&colored),
        plain,
        "coloring changes nothing but the escape codes"
    );
}

/// `text` without its ANSI color escape codes.
fn strip_escapes(text: &str) -> String {
    let mut stripped = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            chars.by_ref().find(|&c| c == 'm');
        } else {
            stripped.push(c);
        }
    }
    stripped
}
