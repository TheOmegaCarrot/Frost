//! Debug info: the line table, spans, stripping, serialization, and the compiler option.

use std::sync::Arc;

use frostlang::bytecode::{
    Bytecode, CompiledFunction, DebugInfo, FormatVersion, NameEntry, OpDebugInfo, SourceLines,
    Span, closure_function, program_from_function, program_function,
};
use frostlang::compile::{CompilerOptions, OptimizationOptions, compile_in_scope};
use frostlang::{Arity, MapKey, Value, Vm, WellFormedProgram, image};

/// A line and column, as `(line, column)`.
fn at(line: u32, column: u32) -> (u32, u32) {
    (line, column)
}

/// The line and column of `offset` in `lines`, as `(line, column)`.
fn position(lines: &SourceLines, offset: u32) -> (u32, u32) {
    let location = lines.location(offset);
    (location.line(), location.column())
}

// ============================================================
// SourceLines
// ============================================================

/// Every offset of `source`, and one past its end, with its position.
fn positions(source: &str) -> Vec<(u32, (u32, u32))> {
    let lines = SourceLines::new(source);
    let end = u32::try_from(source.len()).unwrap();
    (0..=end)
        .map(|offset| (offset, position(&lines, offset)))
        .collect()
}

#[test]
fn locations_count_lines_and_columns_from_one() {
    // a b \n c \n \n d
    // 0 1 2  3 4  5 6
    assert_eq!(
        positions("ab\nc\n\nd"),
        vec![
            (0, at(1, 1)),
            (1, at(1, 2)),
            (2, at(1, 3)), // the newline ending line 1
            (3, at(2, 1)),
            (4, at(2, 2)),
            (5, at(3, 1)), // an empty line is its newline alone
            (6, at(4, 1)),
            (7, at(4, 2)), // the end: just past the last line's last byte
        ]
    );
}

#[test]
fn a_trailing_newline_starts_an_empty_last_line() {
    assert_eq!(
        positions("a\n"),
        vec![(0, at(1, 1)), (1, at(1, 2)), (2, at(2, 1))]
    );
}

#[test]
fn a_carriage_return_is_a_byte_of_its_line() {
    // a \r \n b
    // 0 1  2  3
    assert_eq!(
        positions("a\r\nb"),
        vec![
            (0, at(1, 1)),
            (1, at(1, 2)),
            (2, at(1, 3)),
            (3, at(2, 1)),
            (4, at(2, 2)),
        ]
    );
}

#[test]
fn empty_source_has_one_line() {
    assert_eq!(positions(""), vec![(0, at(1, 1))]);
}

#[test]
fn an_offset_past_the_end_continues_the_last_line() {
    let lines = SourceLines::new("ab\ncd");
    assert_eq!(position(&lines, 5), at(2, 3), "the end");
    assert_eq!(position(&lines, 9), at(2, 7), "four bytes past the end");
    assert_eq!(
        position(&lines, u32::MAX),
        at(2, u32::MAX - 2),
        "the largest offset"
    );
    assert_eq!(
        position(&SourceLines::new(""), 3),
        at(1, 4),
        "past the end of empty source"
    );
}

/// Only a source over 4 GiB reaches the largest offset on its first line;
/// its column saturates rather than overflowing.
#[test]
fn the_largest_offset_on_the_first_line_saturates_its_column() {
    assert_eq!(position(&SourceLines::new("ab"), u32::MAX), at(1, u32::MAX));
}

#[test]
fn columns_count_bytes_not_characters() {
    // `é` is two bytes, so `x` is at byte column 3, not character column 2.
    let source = "\u{e9}x\n\u{e9}";
    assert_eq!(source.find('x'), Some(2));
    let lines = SourceLines::new(source);
    assert_eq!(position(&lines, 2), at(1, 3), "x");
    assert_eq!(position(&lines, 4), at(2, 1), "the second line's start");
    assert_eq!(
        position(&lines, 6),
        at(2, 3),
        "the end, past the two-byte character"
    );
}

#[test]
fn locations_order_by_line_then_column() {
    // a \n b c
    // 0 1  2 3
    let lines = SourceLines::new("a\nbc");
    let [end_of_line_1, start_of_line_2, next_column] =
        [1, 2, 3].map(|offset| lines.location(offset));
    assert!(
        end_of_line_1 < start_of_line_2,
        "a later line orders after, whatever the columns"
    );
    assert!(
        start_of_line_2 < next_column,
        "a later column on the same line orders after"
    );
}

// ============================================================
// Span
// ============================================================

fn span(start: u32, end: u32) -> Span {
    Span { start, end }
}

#[test]
fn filled_union_covers_the_gap_between_disjoint_spans() {
    assert_eq!(span(1, 10).filled_union(span(20, 30)), span(1, 30));
}

#[test]
fn filled_union_of_overlapping_spans_covers_both() {
    assert_eq!(span(1, 10).filled_union(span(5, 15)), span(1, 15));
}

#[test]
fn filled_union_with_a_contained_span_is_the_container() {
    assert_eq!(span(1, 30).filled_union(span(5, 15)), span(1, 30));
    assert_eq!(span(5, 15).filled_union(span(1, 30)), span(1, 30));
}

#[test]
fn filled_union_with_itself_is_itself() {
    assert_eq!(span(4, 8).filled_union(span(4, 8)), span(4, 8));
}

#[test]
fn filled_union_with_an_empty_span_reaches_it() {
    assert_eq!(span(4, 8).filled_union(span(12, 12)), span(4, 12));
    assert_eq!(span(4, 4).filled_union(span(4, 4)), span(4, 4));
}

#[test]
fn filled_union_is_symmetric() {
    let spans = [
        span(0, 0),
        span(1, 10),
        span(5, 15),
        span(20, 30),
        span(3, 4),
    ];
    for a in spans {
        for b in spans {
            assert_eq!(a.filled_union(b), b.filled_union(a), "{a:?} and {b:?}");
        }
    }
}

// ============================================================
// Stripping
// ============================================================

/// Debug info for `code`, one entry per instruction, against `lines`.
fn debug_info_for(code: &[Bytecode], lines: &Arc<SourceLines>) -> DebugInfo {
    // `code` runs straight through from a function of no parameters, so each
    // height follows from the one before; the VM checks them in a debug build.
    let heights = code.iter().scan(Some(1), |height: &mut Option<u64>, op| {
        let entry = *height;
        *height = entry.and_then(|height| op.stack_effect()?.apply(height));
        Some(entry)
    });
    let ops = heights
        .enumerate()
        .map(|(index, height)| {
            let index = u32::try_from(index).unwrap();
            OpDebugInfo {
                span: span(index, index + 1),
                stack_height: height.map(|height| u32::try_from(height).unwrap()),
            }
        })
        .collect();
    DebugInfo {
        ops,
        lines: Arc::clone(lines),
    }
}

fn function(
    name: &str,
    code: Vec<Bytecode>,
    child_fns: Vec<Arc<CompiledFunction>>,
    lines: &Arc<SourceLines>,
) -> CompiledFunction {
    let debug_info = Some(debug_info_for(&code, lines));
    CompiledFunction {
        version: FormatVersion,
        name: name.to_string(),
        origin: None,
        code,
        child_fns,
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
        debug_info,
    }
}

/// `main` defines `outer`, which defines `inner`: every one with debug info.
/// Runnable: it returns 1.
fn nested_program() -> CompiledFunction {
    let lines = Arc::new(SourceLines::new("first\nsecond\nthird"));
    let inner = function(
        "inner",
        vec![Bytecode::Pop, Bytecode::PushInt(1)],
        Vec::new(),
        &lines,
    );
    let closure_of_inner = Bytecode::CreateClosure {
        child: 0,
        captures: 0,
    };
    let outer = function(
        "outer",
        vec![Bytecode::Pop, closure_of_inner, Bytecode::Call(0)],
        vec![Arc::new(inner)],
        &lines,
    );
    let closure_of_outer = Bytecode::CreateClosure {
        child: 0,
        captures: 0,
    };
    function(
        "main",
        vec![Bytecode::Pop, closure_of_outer, Bytecode::Call(0)],
        vec![Arc::new(outer)],
        &lines,
    )
}

/// A function and a child with every field set to something other than its default,
/// so stripping that reset any of them would show. Not runnable.
fn fully_populated() -> CompiledFunction {
    let lines = Arc::new(SourceLines::new("one\ntwo"));
    let origin: Option<Arc<str>> = Some(Arc::from("populated.frst"));
    let entry = |name: &str, exported| NameEntry {
        name: name.to_string(),
        exported,
    };
    let child_code = vec![Bytecode::LoadLocal(0), Bytecode::PushInt(2)];
    let child = CompiledFunction {
        version: FormatVersion,
        name: "child".to_string(),
        origin: origin.clone(),
        debug_info: Some(debug_info_for(&child_code, &lines)),
        code: child_code,
        child_fns: Vec::new(),
        constants: vec![Value::from(vec![Value::from(2)])],
        key_constants: vec![MapKey::Int(7)],
        name_table: vec![entry("captured", false), entry("param", false)],
        num_captures: 1,
        arity: Arity::AtLeast(1),
    };
    let code = vec![Bytecode::LoadConst(0), Bytecode::HardIndexMap(0)];
    CompiledFunction {
        version: FormatVersion,
        name: "parent".to_string(),
        origin,
        debug_info: Some(debug_info_for(&code, &lines)),
        code,
        child_fns: vec![Arc::new(child)],
        constants: vec![Value::from("constant")],
        key_constants: vec![MapKey::from("key")],
        name_table: vec![entry("exported", true)],
        num_captures: 0,
        arity: Arity::Between(1, 3),
    }
}

/// `function`, serialized, with every function's `debug_info` left out.
fn without_debug_info(function: &CompiledFunction) -> serde_json::Value {
    fn remove_debug_info(json: &mut serde_json::Value) {
        if let Some(object) = json.as_object_mut() {
            object.remove("debug_info");
            for child in object
                .get_mut("child_fns")
                .and_then(serde_json::Value::as_array_mut)
                .into_iter()
                .flatten()
            {
                remove_debug_info(child);
            }
        }
    }
    let mut json = serde_json::to_value(function).unwrap();
    remove_debug_info(&mut json);
    json
}

/// The name of every function in `function`'s tree, root first, with whether it has debug info.
fn debug_info_presence(function: &CompiledFunction) -> Vec<(String, bool)> {
    let mut found = vec![(function.name.clone(), function.debug_info.is_some())];
    for child in &function.child_fns {
        found.extend(debug_info_presence(child));
    }
    found
}

fn names_with(has_debug_info: bool) -> Vec<(String, bool)> {
    ["main", "outer", "inner"]
        .map(|name| (name.to_string(), has_debug_info))
        .to_vec()
}

#[test]
fn the_nested_program_starts_with_debug_info_throughout() {
    assert_eq!(debug_info_presence(&nested_program()), names_with(true));
}

#[test]
fn strip_removes_debug_info_from_every_nested_function() {
    let stripped = nested_program().strip();
    assert_eq!(debug_info_presence(&stripped), names_with(false));
}

#[test]
fn strip_keeps_everything_else() {
    let original = fully_populated();
    let stripped = original.clone().strip();
    assert!(
        stripped.debug_info.is_none() && stripped.child_fns[0].debug_info.is_none(),
        "stripped of debug info"
    );
    assert_eq!(
        without_debug_info(&stripped),
        without_debug_info(&original),
        "every other field, in every function, is unchanged"
    );
}

#[test]
fn strip_without_debug_info_changes_nothing() {
    let stripped = fully_populated().strip();
    let restripped = stripped.clone().strip();
    assert_eq!(
        serde_json::to_value(&restripped).unwrap(),
        serde_json::to_value(&stripped).unwrap()
    );
}

#[test]
fn strip_copies_a_nested_function_shared_elsewhere() {
    let program = nested_program();
    let shared_outer = Arc::clone(&program.child_fns[0]);
    let stripped = program.strip();
    assert!(
        shared_outer.debug_info.is_some(),
        "the other holder's function is untouched"
    );
    assert!(
        stripped.child_fns[0].debug_info.is_none(),
        "the stripped program's copy is stripped"
    );
}

#[test]
fn compiled_program_strip_removes_debug_info_throughout() {
    let program = program_from_function(Arc::new(nested_program()));
    let stripped = program.strip();
    assert_eq!(
        debug_info_presence(program_function(&stripped)),
        names_with(false)
    );
}

#[test]
fn compiled_program_strip_copies_a_shared_program() {
    let program = program_from_function(Arc::new(nested_program()));
    let kept = program.clone();
    let stripped = program.strip();
    assert_eq!(
        debug_info_presence(program_function(&kept)),
        names_with(true)
    );
    assert_eq!(
        debug_info_presence(program_function(&stripped)),
        names_with(false)
    );
}

/// The function tree of `program`, by way of a closure made from it.
fn tree_of(program: WellFormedProgram) -> Arc<CompiledFunction> {
    let closure = program.into_closure().unwrap();
    Arc::clone(closure_function(&closure))
}

#[test]
fn well_formed_program_strip_removes_debug_info_throughout() {
    let program = Arc::new(nested_program()).assert_well_formed();
    assert_eq!(
        debug_info_presence(&tree_of(program.strip())),
        names_with(false)
    );
}

#[test]
fn well_formed_program_strip_copies_a_shared_program() {
    let program = Arc::new(nested_program()).assert_well_formed();
    let kept = program.clone();
    let stripped = program.strip();
    assert_eq!(debug_info_presence(&tree_of(kept)), names_with(true));
    assert_eq!(debug_info_presence(&tree_of(stripped)), names_with(false));
}

#[test]
fn a_stripped_well_formed_program_runs_the_same() {
    let run = |program: WellFormedProgram| {
        let closure = program.into_closure().unwrap();
        Vm::factory().build(closure).run().unwrap().tail().clone()
    };
    let program = Arc::new(nested_program()).assert_well_formed();
    let expected = run(program.clone());
    assert_eq!(expected, Value::from(1), "main returns inner's result");
    assert_eq!(run(program.strip()), expected);
}

// ============================================================
// Serialization
// ============================================================

#[test]
fn debug_info_round_trips_through_postcard() {
    let program = nested_program();
    let bytes = postcard::to_allocvec(&program).unwrap();
    let restored: CompiledFunction = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(restored.debug_info, program.debug_info);
    assert_eq!(
        restored.child_fns[0].child_fns[0].debug_info,
        program.child_fns[0].child_fns[0].debug_info
    );
    assert_eq!(postcard::to_allocvec(&restored).unwrap(), bytes);
}

#[test]
fn debug_info_round_trips_through_json() {
    let program = nested_program();
    let json = serde_json::to_string(&program).unwrap();
    let restored: CompiledFunction = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.debug_info, program.debug_info);
    assert_eq!(
        restored.child_fns[0].debug_info,
        program.child_fns[0].debug_info
    );
}

#[test]
fn absent_debug_info_round_trips() {
    let program = nested_program().strip();
    let bytes = postcard::to_allocvec(&program).unwrap();
    let restored: CompiledFunction = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(debug_info_presence(&restored), names_with(false));
}

#[test]
fn a_stripped_image_is_smaller() {
    let program = Arc::new(nested_program()).assert_well_formed();
    let full = image::encode(&program);
    let stripped = image::encode(&program.strip());
    assert!(
        stripped.len() < full.len(),
        "stripped {} bytes, full {} bytes",
        stripped.len(),
        full.len()
    );
}

// ============================================================
// Compiler option
// ============================================================

#[test]
fn debug_info_is_off_by_default() {
    assert!(!CompilerOptions::new().debug_info);
    assert!(!CompilerOptions::default().debug_info);
}

#[test]
fn with_debug_info_sets_only_debug_info() {
    let on = CompilerOptions::new().with_debug_info(true);
    assert!(on.debug_info);
    assert_eq!(on.with_debug_info(false), CompilerOptions::new());
}

/// Every function in the tree of `source`, compiled with `options`, by whether it has debug info.
fn compiled_debug_info_presence(source: &str, options: CompilerOptions) -> Vec<(String, bool)> {
    let program = compile_in_scope("debug_info.frst", source, options, &[])
        .unwrap_or_else(|errors| panic!("{source:?} compiles:\n{}", errors.render_plain()))
        .code;
    debug_info_presence(&tree_of(program))
}

const NESTED_SOURCE: &str = r"
defn outer() -> fn x -> x + 1
outer()(1)
";

#[test]
fn compiling_without_debug_info_gives_none_anywhere() {
    // Unoptimized, so folding cannot collapse the nested functions into a constant.
    let options = CompilerOptions::new()
        .with_optimization(OptimizationOptions::NONE)
        .with_debug_info(false);
    let presence = compiled_debug_info_presence(NESTED_SOURCE, options);
    assert!(
        presence.len() >= 3,
        "the program, outer, and its lambda: {presence:?}"
    );
    assert!(
        presence.iter().all(|(_, has)| !has),
        "no function has debug info: {presence:?}"
    );
}

#[test]
fn compiling_with_debug_info_gives_every_function_some() {
    // Unoptimized, so folding cannot collapse the nested functions into a constant.
    let options = CompilerOptions::new()
        .with_optimization(OptimizationOptions::NONE)
        .with_debug_info(true);
    let presence = compiled_debug_info_presence(NESTED_SOURCE, options);
    assert!(
        presence.len() >= 3,
        "the program, outer, and its lambda: {presence:?}"
    );
    assert!(
        presence.iter().all(|(_, has)| *has),
        "every function has debug info: {presence:?}"
    );
}

#[test]
fn debug_info_locates_an_instruction_by_its_index() {
    let lines = Arc::new(SourceLines::new("one\ntwo"));
    let debug_info = DebugInfo {
        ops: vec![
            OpDebugInfo {
                span: span(0, 3),
                stack_height: Some(1),
            },
            OpDebugInfo {
                span: span(4, 7),
                stack_height: Some(0),
            },
        ],
        lines,
    };
    assert_eq!(
        debug_info.location(1).map(|at| (at.line(), at.column())),
        Some((2, 1)),
        "the second instruction starts the second line"
    );
    assert_eq!(debug_info.location(2), None, "past the last entry");
}
