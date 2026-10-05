//! `CompiledFunction::disassemble`: the listing of a function tree.
//!
//! Every function here is written by hand rather than compiled, so the tests
//! pin the listing's format, not what the compiler emits.

use std::num::NonZeroUsize;
use std::sync::Arc;

use frostlang_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostFloat, FrostType, MapKey, NameEntry,
    Value,
};

use Bytecode::*;

/// A function named `name` running `code`, taking no arguments, with nothing
/// else: no slots, pools, or children.
fn function(name: &str, code: Vec<Bytecode>) -> CompiledFunction {
    CompiledFunction {
        version: FormatVersion,
        name: name.to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
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
       0  greeting    capture
       1  total       export
       2  n
  code
       0  LoadLocal  0  ; greeting
       1  DefLocal   1  ; total
       2  LoadLocal  2  ; n
";
    assert_eq!(listing(&main), expected);
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
    let print = frostlang_runtime::GLOBAL_NAMES
        .iter()
        .position(|&name| name == "print")
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
            Jump(0),           // to 5
            PushInt(2),
            Jump(0), // to 7, the end
        ],
    );
    let expected = r"function <main>
  arity 0, 0 captures
  code
       0  PushTrue
       1  JumpIfFalse     L0  ; +3
       2  PushInt         1
       3  PeekJumpIfTrue  L1  ; +3
       4  Jump            L0  ; +0
  L0:
       5  PushInt         2
       6  Jump            L1  ; +0
  L1:
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
            Concat(NonZeroUsize::new(2).unwrap()),
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

#[test]
fn nested_functions_follow_depth_first() {
    let mut grandchild = function("inner", vec![PushNull]);
    grandchild.num_captures = 1;
    grandchild.name_table = vec![slot("x", false)];
    let mut first = function("first", vec![CreateClosure(0)]);
    first.child_fns = vec![Arc::new(grandchild)];
    first.arity = Arity::AtLeast(1);
    let mut second = function("second", vec![PushNull]);
    second.arity = Arity::Between(1, 3);
    let mut main = function("<main>", vec![CreateClosure(0), CreateClosure(1)]);
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
       0  x           capture
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
            LoadGlobal(usize::MAX),
            CreateClosure(0),
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
       4  CreateClosure  0                     ; ?
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
