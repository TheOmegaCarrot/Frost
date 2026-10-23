//! The debug info the compiler emits: the span each instruction is derived from,
//! and the operand stack height on entry to it.
//!
//! A code-shape test pins the options it is about (see `Script::code`), and
//! shows each span as the source text it covers. The height counts the callee
//! and its arguments, so a function of one parameter starts at 2.

use std::num::NonZeroU64;

use frostlang::Arity;
use frostlang::bytecode::{Bytecode, CompiledFunction, Span, closure_function};
use frostlang::compile::{CompilerOptions, Optimization, OptimizationOptions, compile_program};
use frostlang::{FrostType, TypeSet};

use crate::common::{Pop, global_slot};
use crate::script::{Emitted, Script, UNOPTIMIZED, optimization_permutations};

use Bytecode::*;

/// One instruction as its debug info describes it: the instruction, the source
/// text its span covers, and its entry height.
type Row<'a> = (Bytecode, &'a str, Option<u32>);

/// [`Bytecode::Dup`], which a glob import cannot bring in.
const DUP: Bytecode = Bytecode::Dup;

/// The source text `span` covers.
fn text(source: &str, span: Span) -> &str {
    &source[span.start as usize..span.end as usize]
}

/// Each instruction of `emitted`, compiled from `source`, as a [`Row`].
fn rows<'a>(source: &'a str, emitted: &Emitted) -> Vec<Row<'a>> {
    let debug_info = emitted
        .debug_info()
        .unwrap_or_else(|| panic!("{} should have debug info", emitted.name()));
    assert_eq!(
        debug_info.ops.len(),
        emitted.code.len(),
        "{} should have one debug entry per instruction",
        emitted.name()
    );
    emitted
        .code
        .iter()
        .zip(&debug_info.ops)
        .map(|(&op, info)| (op, text(source, info.span), info.stack_height))
        .collect()
}

/// The source text from the first `from` through the last `to`.
fn spanning<'a>(source: &'a str, from: &str, to: &str) -> &'a str {
    let start = source.find(from).expect("the source holds `from`");
    let end = source.rfind(to).expect("the source holds `to`") + to.len();
    &source[start..end]
}

/// `LoadGlobal` of the global `name`.
fn load_global(name: &str) -> Bytecode {
    LoadGlobal(global_slot(name))
}

fn count(n: u64) -> NonZeroU64 {
    NonZeroU64::new(n).expect("a nonzero count")
}

fn int_type() -> TypeSet {
    FrostType::INT
}

fn map_type() -> TypeSet {
    FrostType::MAP
}

/// Assert `source`'s top level, under `optimization`, has `expected` rows.
fn assert_top_level(source: &str, optimization: OptimizationOptions, expected: &[Row]) {
    let emitted = Script::new(source).code(optimization);
    assert_eq!(
        rows(source, &emitted),
        expected,
        "{source:?} under {optimization:?}"
    );
}

/// Assert the function nested first in `source`'s top level, under
/// `optimization`, has `expected` rows.
fn assert_nested(source: &str, optimization: OptimizationOptions, expected: &[Row]) {
    let emitted = Script::new(source).code(optimization).nested(0);
    assert_eq!(
        rows(source, &emitted),
        expected,
        "{source:?}'s nested function under {optimization:?}"
    );
}

// ============================================================
// Spans by construct
// ============================================================

#[test]
fn the_top_levels_own_value_is_dropped_by_the_whole_script() {
    let source = r"
print(1)
";
    assert_top_level(
        source,
        UNOPTIMIZED,
        &[
            (Pop, source, Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(1), "1", Some(1)),
            (TailCall(1), "print(1)", Some(2)),
        ],
    );
}

#[test]
fn an_operation_is_its_whole_expression_and_each_operand_its_own() {
    let source = r"
defn f(x) -> x * 2 + 1
f(3)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "x", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (LoadLocal(0), "x", Some(0)),
            (PushInt(2), "2", Some(1)),
            (Multiply, "x * 2", Some(2)),
            (PushInt(1), "1", Some(1)),
            (Add, "x * 2 + 1", Some(2)),
        ],
    );
}

#[test]
fn a_unary_operation_is_its_whole_expression() {
    let source = r"
defn f(x) -> [-x, not x]
f(1)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "x", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (LoadLocal(0), "x", Some(0)),
            (Negate, "-x", Some(1)),
            (LoadLocal(0), "x", Some(1)),
            (LogicalNot, "not x", Some(2)),
            (MakeArray(2), "[-x, not x]", Some(2)),
        ],
    );
}

#[test]
fn a_call_is_the_whole_call() {
    let source = r"
defn f(x) -> x
print(f(3))
";
    assert_top_level(
        source,
        UNOPTIMIZED,
        &[
            (Pop, source, Some(1)),
            (
                CreateClosure {
                    child: 0,
                    captures: 0,
                },
                "f(x) -> x",
                Some(0),
            ),
            (DefLocal(0), "f", Some(1)),
            (load_global("print"), "print", Some(0)),
            (LoadLocal(0), "f", Some(1)),
            (PushInt(3), "3", Some(2)),
            (Call(1), "f(3)", Some(3)),
            (TailCall(1), "print(f(3))", Some(2)),
        ],
    );
}

#[test]
fn an_omitted_else_is_the_ifs() {
    let source = r"
defn f(x) -> if x: 1
f(true)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "x", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (LoadLocal(0), "x", Some(0)),
            (JumpIfFalse(2), "if x: 1", Some(1)),
            (PushInt(1), "1", Some(0)),
            (Jump(1), "if x: 1", Some(1)),
            (PushNull, "if x: 1", Some(0)),
        ],
    );
}

#[test]
fn an_and_branches_as_its_whole_expression() {
    let source = r"
defn f(x, y) -> x and y
f(1, 2)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "y", Some(3)),
            (DefLocal(1), "x", Some(2)),
            (DefLocal(2), "f", Some(1)),
            (LoadLocal(1), "x", Some(0)),
            (PeekJumpIfFalse(2), "x and y", Some(1)),
            (Pop, "x and y", Some(1)),
            (LoadLocal(0), "y", Some(0)),
        ],
    );
}

#[test]
fn an_or_branches_as_its_whole_expression() {
    let source = r"
defn f(x, y) -> x or y
f(1, 2)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "y", Some(3)),
            (DefLocal(1), "x", Some(2)),
            (DefLocal(2), "f", Some(1)),
            (LoadLocal(1), "x", Some(0)),
            (PeekJumpIfTrue(2), "x or y", Some(1)),
            (Pop, "x or y", Some(1)),
            (LoadLocal(0), "y", Some(0)),
        ],
    );
}

#[test]
fn a_structure_literal_is_its_whole_literal() {
    let source = r"
defn f(x) -> [x, {k: x}]
f(1)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "x", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (LoadLocal(0), "x", Some(0)),
            (LoadConst(0), "k", Some(1)),
            (LoadLocal(0), "x", Some(2)),
            (MakeMap(1), "{k: x}", Some(3)),
            (MakeArray(2), "[x, {k: x}]", Some(2)),
        ],
    );
}

#[test]
fn a_format_strings_literal_segments_are_the_format_strings() {
    let source = r"
defn f(x) -> $'a${x}b'
f(1)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "x", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (LoadConst(0), "$'a${x}b'", Some(0)),
            (LoadLocal(0), "x", Some(1)),
            (LoadConst(1), "$'a${x}b'", Some(2)),
            (Concat(count(3)), "$'a${x}b'", Some(3)),
        ],
    );
}

#[test]
fn an_index_is_its_whole_expression() {
    let source = r"
defn f(m) -> m.a[0]
f({a: [1]})
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "m", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (LoadLocal(0), "m", Some(0)),
            (HardIndexMap(0), "m.a", Some(1)),
            (PushInt(0), "0", Some(1)),
            (SoftIndexStructure, "m.a[0]", Some(2)),
        ],
    );
}

#[test]
fn a_collection_function_call_spans_the_whole_call() {
    let source = r"
defn f(xs) -> map(xs, $($ + 1))
f([1])
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "xs", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (load_global("map"), "map", Some(0)),
            (LoadLocal(0), "xs", Some(1)),
            (
                CreateClosure {
                    child: 0,
                    captures: 0,
                },
                "$($ + 1)",
                Some(2),
            ),
            (TailCall(2), "map(xs, $($ + 1))", Some(3)),
        ],
    );
}

#[test]
fn a_statements_discarded_value_is_dropped_by_the_statement() {
    let source = r"
print(1)
print(2)
";
    assert_top_level(
        source,
        UNOPTIMIZED,
        &[
            (Pop, source, Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(1), "1", Some(1)),
            (Call(1), "print(1)", Some(2)),
            (Pop, "print(1)", Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(2), "2", Some(1)),
            (TailCall(1), "print(2)", Some(2)),
        ],
    );
}

// ============================================================
// Lambdas
// ============================================================

#[test]
fn a_lambda_seats_its_captures_and_creates_its_closure_as_the_whole_lambda() {
    let source = r"
def n = print(1)
def add = fn x -> x + n
add(2)
";
    assert_top_level(
        source,
        UNOPTIMIZED,
        &[
            (Pop, source, Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(1), "1", Some(1)),
            (Call(1), "print(1)", Some(2)),
            (DefLocal(0), "n", Some(1)),
            (LoadLocal(0), "fn x -> x + n", Some(0)),
            (
                CreateClosure {
                    child: 0,
                    captures: 1,
                },
                "fn x -> x + n",
                Some(1),
            ),
            (DefLocal(1), "add", Some(1)),
            (LoadLocal(1), "add", Some(0)),
            (PushInt(2), "2", Some(1)),
            (TailCall(1), "add(2)", Some(2)),
        ],
    );
}

#[test]
fn a_parameter_is_stored_as_itself_and_an_unnamed_self_as_the_lambda() {
    let source = r"
def add = fn x, ...rest -> x
add(1, 2)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "rest", Some(3)),
            (DefLocal(1), "x", Some(2)),
            (Pop, "fn x, ...rest -> x", Some(1)),
            (LoadLocal(1), "x", Some(0)),
        ],
    );
}

#[test]
fn an_abbreviated_lambdas_parameters_are_the_lambdas() {
    let source = r"
def second = $($2)
second(1, 2)
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "$($2)", Some(3)),
            (Pop, "$($2)", Some(2)),
            (Pop, "$($2)", Some(1)),
            (LoadLocal(0), "$2", Some(0)),
        ],
    );
}

// ============================================================
// Patterns and match
// ============================================================

#[test]
fn a_match_arm_is_tried_as_the_arm_and_its_checks_are_its_patterns() {
    let source = r"
defn f(x) -> match x {
    [1, b] => b,
    y is Int => y,
}
f([1, 2])
";
    let whole = spanning(source, "match", "}");
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "x", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (LoadLocal(0), "x", Some(0)),
            (MarkStack, "[1, b] => b", Some(1)),
            (DUP, "[1, b] => b", Some(1)),
            (TestArrayLenExact(2), "[1, b]", Some(2)),
            (JumpIfFalse(9), "[1, b]", Some(3)),
            (ExplodeArray(2), "[1, b]", Some(2)),
            (PushInt(1), "1", Some(3)),
            (CompareEqual, "1", Some(4)),
            (JumpIfFalse(5), "1", Some(3)),
            (DefLocal(2), "b", Some(2)),
            (DropMark, "[1, b] => b", Some(1)),
            (Pop, "[1, b] => b", Some(1)),
            (LoadLocal(2), "b", Some(0)),
            (Jump(16), "[1, b] => b", Some(1)),
            (RewindToMark, "[1, b] => b", Some(2)),
            (MarkStack, "y is Int => y", Some(1)),
            (DUP, "y is Int => y", Some(1)),
            (DUP, "Int", Some(2)),
            (TypeTest(int_type()), "Int", Some(3)),
            (JumpIfFalse(5), "Int", Some(3)),
            (DefLocal(3), "y", Some(2)),
            (DropMark, "y is Int => y", Some(1)),
            (Pop, "y is Int => y", Some(1)),
            (LoadLocal(3), "y", Some(0)),
            (Jump(5), "y is Int => y", Some(1)),
            (RewindToMark, "y is Int => y", Some(2)),
            (LoadConst(0), whole, Some(1)),
            (PeekDown(1), whole, Some(2)),
            (Concat(count(2)), whole, Some(3)),
            (ProduceError, whole, Some(2)),
        ],
    );
}

#[test]
fn a_value_pattern_and_a_guard_are_each_their_own_check() {
    let source = r"
defn f(x, k) -> match x {
    (k) => 1,
    n if: n > 0 => n,
    _ => 0,
}
f(1, 2)
";
    let emitted = Script::new(source).code(UNOPTIMIZED).nested(0);
    let checks: Vec<Row> = rows(source, &emitted)
        .into_iter()
        .filter(|(op, ..)| matches!(op, CompareEqual | JumpIfFalse(_)))
        .collect();
    assert_eq!(
        checks,
        [
            (CompareEqual, "(k)", Some(3)),
            (JumpIfFalse(4), "(k)", Some(2)),
            (JumpIfFalse(4), "n > 0", Some(2)),
        ],
        "a pinned value is checked as its pattern, parentheses included, and a guard as the guard"
    );
}

#[test]
fn a_map_patterns_lookups_are_each_its_entrys_and_a_whole_binding_its_binding() {
    let source = r"
defn f(m, k) -> match m {
    {[k]: v, a} as whole => [v, a, whole],
    _ => 0,
}
f({a: 1}, 'a')
";
    let pattern = "{[k]: v, a} as whole";
    let emitted = Script::new(source).code(UNOPTIMIZED).nested(0);
    let rows = rows(source, &emitted);
    let start = rows
        .iter()
        .position(|(op, ..)| *op == TypeTest(map_type()))
        .expect("the pattern tests for a Map")
        - 1;
    assert_eq!(
        rows[start..start + 13],
        [
            (DUP, pattern, Some(2)),
            (TypeTest(map_type()), pattern, Some(3)),
            (JumpIfFalse(17), pattern, Some(3)),
            (LoadLocal(0), "k", Some(2)),
            (TestKey, "[k]: v", Some(3)),
            (JumpIfFalse(14), "[k]: v", Some(4)),
            (ExtractKey, "[k]: v", Some(3)),
            (DefLocal(3), "v", Some(3)),
            (TestConstKey(0), "a", Some(2)),
            (JumpIfFalse(10), "a", Some(3)),
            (ExtractConstKey(1), "a", Some(2)),
            (DefLocal(4), "a", Some(3)),
            (DefLocal(5), "whole", Some(2)),
        ],
        "a computed key is its own expression, each lookup its entry, the whole value its binding"
    );
}

#[test]
fn an_array_patterns_rest_is_its_binding() {
    let source = r"
defn f(xs) -> match xs {
    [first, ...rest] => rest,
    _ => 0,
}
f([1, 2])
";
    let pattern = "[first, ...rest]";
    let emitted = Script::new(source).code(UNOPTIMIZED).nested(0);
    let rows = rows(source, &emitted);
    let start = rows
        .iter()
        .position(|(op, ..)| *op == TestArrayLenAtLeast(1))
        .expect("the pattern tests the length");
    assert_eq!(
        rows[start..start + 6],
        [
            (TestArrayLenAtLeast(1), pattern, Some(2)),
            (JumpIfFalse(8), pattern, Some(3)),
            (SplitArray(1), pattern, Some(2)),
            (ExplodeArray(1), pattern, Some(3)),
            (DefLocal(2), "first", Some(3)),
            (DefLocal(3), "rest", Some(2)),
        ],
        "the layout is the Array pattern's, each store its binding's"
    );
}

/// A rewind can be reached from checks that leave the stack at different heights.
#[test]
fn a_rewind_reached_at_different_heights_has_no_height() {
    let source = r"
defn f(x) -> match x {
    [[1], b] => b,
    _ => 0,
}
f([[1], 2])
";
    let emitted = Script::new(source).code(UNOPTIMIZED).nested(0);
    let rewinds: Vec<Row> = rows(source, &emitted)
        .into_iter()
        .filter(|(op, ..)| *op == RewindToMark)
        .collect();
    assert_eq!(
        rewinds.first(),
        Some(&(RewindToMark, "[[1], b] => b", None)),
        "the outer length check fails with only the target's copy taken, the inner one with \
         the copy's elements laid out: {rewinds:?}"
    );
}

/// After an arm that matches every value, the match's failure path is unreachable.
#[test]
fn code_nothing_reaches_has_no_height() {
    let source = r"
defn f(x) -> match x {
    _ => 0,
}
f(1)
";
    let emitted = Script::new(source).code(UNOPTIMIZED).nested(0);
    let rows = rows(source, &emitted);
    let rewind = rows
        .iter()
        .position(|(op, ..)| *op == RewindToMark)
        .expect("the arm rewinds on failure");
    assert!(
        rows[rewind..].iter().all(|(.., height)| height.is_none()),
        "nothing jumps to the failed arm's rewind, so it and the no-match error after it \
         have no height: {rows:?}"
    );
    assert!(
        rows[..rewind].iter().all(|(.., height)| height.is_some()),
        "everything before is reached: {rows:?}"
    );
}

#[test]
fn an_alternatives_branch_is_tried_as_the_branch() {
    let source = r"
defn f(x) -> match x {
    1 | 2 => true,
    _ => false,
}
f(2)
";
    let emitted = Script::new(source).code(UNOPTIMIZED).nested(0);
    let tries: Vec<(Bytecode, &str)> = rows(source, &emitted)
        .into_iter()
        .filter(|(op, ..)| matches!(op, MarkStack | DropMark | RewindToMark))
        .map(|(op, text, _)| (op, text))
        .collect();
    assert_eq!(
        tries,
        [
            // The arm, then its first branch: the last needs no mark of its own.
            (MarkStack, "1 | 2 => true"),
            (MarkStack, "1"),
            (DropMark, "1"),
            (RewindToMark, "1"),
            (DropMark, "1 | 2 => true"),
            (RewindToMark, "1 | 2 => true"),
            (MarkStack, "_ => false"),
            (DropMark, "_ => false"),
            (RewindToMark, "_ => false"),
        ],
        "each alternative branch is tried as the branch, within its arm"
    );
}

#[test]
fn a_raising_alternatives_failure_is_the_alternatives() {
    let source = r"
defn f(x) -> {
    def [a] | [a, _] = x
    a
}
f([1])
";
    let alternative = "[a] | [a, _]";
    let emitted = Script::new(source).code(UNOPTIMIZED).nested(0);
    let rows = rows(source, &emitted);
    let tail = &rows[rows.len() - 3..];
    assert_eq!(
        tail,
        [
            (LoadConst(0), alternative, Some(1)),
            (ProduceError, alternative, Some(2)),
            (LoadLocal(2), "a", Some(0)),
        ],
        "no branch matching raises as the whole alternative"
    );
    let marks: Vec<(Bytecode, &str)> = rows
        .iter()
        .filter(|(op, ..)| matches!(op, MarkStack | RewindToMark))
        .map(|&(op, text, _)| (op, text))
        .collect();
    assert_eq!(
        marks,
        [
            (MarkStack, "[a]"),
            (RewindToMark, "[a]"),
            (MarkStack, "[a, _]"),
            (RewindToMark, "[a, _]"),
        ],
        "a raising alternative marks every branch, the last included"
    );
}

#[test]
fn a_def_patterns_raised_mismatch_is_its_check() {
    let source = r"
defn f(x) -> {
    def [a] = x
    a
}
f([1])
";
    assert_nested(
        source,
        UNOPTIMIZED,
        &[
            (DefLocal(0), "x", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (LoadLocal(0), "x", Some(0)),
            (TestArrayLenExact(1), "[a]", Some(1)),
            (JumpIfTrue(2), "[a]", Some(2)),
            (LoadConst(0), "[a]", Some(1)),
            (ProduceError, "[a]", Some(2)),
            (ExplodeArray(1), "[a]", Some(1)),
            (DefLocal(2), "a", Some(1)),
            (LoadLocal(2), "a", Some(0)),
        ],
    );
}

/// A tail call in an arm may replace the frame or go on to the arm's jump to the end.
#[test]
fn an_arm_in_tail_position_goes_on_past_its_tail_call() {
    let source = r"
defn f(x) -> match x {
    1 => print(x),
    _ => 0,
}
f(1)
";
    let emitted = Script::new(source).code(UNOPTIMIZED).nested(0);
    let rows = rows(source, &emitted);
    let call = rows
        .iter()
        .position(|(op, ..)| *op == TailCall(1))
        .expect("the arm's result is a tail call");
    assert_eq!(
        rows[call..call + 2],
        [
            (TailCall(1), "print(x)", Some(2)),
            (Jump(13), "1 => print(x)", Some(1)),
        ],
        "after the tail call, the arm jumps on with only the result"
    );
}

// ============================================================
// Optimization
// ============================================================

#[test]
fn a_folded_constant_is_everything_it_replaced() {
    let source = r"
print(1 + 2 * 3)
";
    assert_top_level(
        source,
        UNOPTIMIZED.with(Optimization::ConstantFold, true),
        &[
            (Pop, source, Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(7), "1 + 2 * 3", Some(1)),
            (TailCall(1), "print(1 + 2 * 3)", Some(2)),
        ],
    );
}

#[test]
fn a_propagated_constant_is_loaded_as_its_use() {
    let source = r"
def k = 10
print(k)
";
    assert_top_level(
        source,
        UNOPTIMIZED.with(Optimization::ConstantPropagate, true),
        &[
            (Pop, source, Some(1)),
            (PushInt(10), "10", Some(0)),
            (DefLocal(0), "k", Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(10), "k", Some(1)),
            (TailCall(1), "print(k)", Some(2)),
        ],
    );
}

#[test]
fn a_folded_def_patterns_values_are_the_whole_match_and_its_stores_their_bindings() {
    let source = r"
def [a, b] = [1, 2]
print(a)
";
    assert_top_level(
        source,
        UNOPTIMIZED.with(Optimization::ConstantFold, true),
        &[
            (Pop, source, Some(1)),
            (PushInt(1), "[a, b] = [1, 2]", Some(0)),
            (DefLocal(0), "a", Some(1)),
            (PushInt(2), "[a, b] = [1, 2]", Some(0)),
            (DefLocal(1), "b", Some(1)),
            (load_global("print"), "print", Some(0)),
            (LoadLocal(0), "a", Some(1)),
            (TailCall(1), "print(a)", Some(2)),
        ],
    );
}

#[test]
fn branch_elimination_keeps_the_surviving_branchs_own_spans() {
    let source = r"
print(if true: 1 else: 2)
";
    assert_top_level(
        source,
        UNOPTIMIZED.with(Optimization::BranchEliminate, true),
        &[
            (Pop, source, Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(1), "1", Some(1)),
            (TailCall(1), "print(if true: 1 else: 2)", Some(2)),
        ],
    );
}

#[test]
fn a_dead_stores_drop_is_the_stores_binding() {
    let source = r"
def x = print(1)
2
";
    assert_top_level(
        source,
        UNOPTIMIZED.with(Optimization::DeadStoreEliminate, true),
        &[
            (Pop, source, Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(1), "1", Some(1)),
            (Call(1), "print(1)", Some(2)),
            (Pop, "x", Some(1)),
            (PushInt(2), "2", Some(0)),
        ],
    );
}

#[test]
fn a_kept_drop_after_an_eliminated_discard_keeps_its_own_span() {
    // The load of `1` and the statement's drop of it go; the drop of
    // `print(0)`'s result stays, its own statement's.
    let source = r"
print(0)
1
print(2)
";
    assert_top_level(
        source,
        UNOPTIMIZED.with(Optimization::DiscardEliminate, true),
        &[
            (Pop, source, Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(0), "0", Some(1)),
            (Call(1), "print(0)", Some(2)),
            (Pop, "print(0)", Some(1)),
            (load_global("print"), "print", Some(0)),
            (PushInt(2), "2", Some(1)),
            (TailCall(1), "print(2)", Some(2)),
        ],
    );
}

#[test]
fn a_consumed_local_keeps_its_loads_span() {
    let source = r"
defn f(x) -> [x]
f(1)
";
    assert_nested(
        source,
        UNOPTIMIZED.with(Optimization::ConsumeLocals, true),
        &[
            (DefLocal(0), "x", Some(2)),
            (DefLocal(1), "f", Some(1)),
            (ConsumeLocal(0), "x", Some(0)),
            (MakeArray(1), "[x]", Some(1)),
        ],
    );
}

// ============================================================
// Every option
// ============================================================

/// Scripts exercising every construct, for the checks that hold under every option.
const SCRIPTS: &[&str] = &[
    r"
defn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)
print(fact(5))
",
    r"
def xs = [1, 2, 3]
def total = reduce(xs, fn acc, x -> acc + x)
def doubled = map(xs, $($ * 2))
print($'${total} ${doubled}')
",
    r"
defn classify(v) -> match v {
    0 | 1 => 'small',
    [a, b, ...rest] if: a > b => $'falling ${rest}',
    {name, age: a} as person => person.name,
    s is String => s,
    _ => 'other',
}
print(map([0, [2, 1, 9], {name: 'n', age: 3}, 'x', null], classify))
",
    r"
def {a, b: [c, d]} = {a: 1, b: [2, 3]}
def k = 'key'
def m = {[k]: a and c, other: d or null}
print(do {
    def inner = m.other
    inner + m[k]
})
",
    r"
def n = print(0)
defn make(x) -> fn y -> x + y + n
print(make(1)(2))
print(filter([1, 2, 3], $($ > 1)))
",
];

/// Every function in the tree of `function`, root first.
fn functions(function: &CompiledFunction) -> Vec<&CompiledFunction> {
    std::iter::once(function)
        .chain(function.child_fns.iter().flat_map(|child| functions(child)))
        .collect()
}

fn compile_tree(source: &str, options: CompilerOptions) -> CompiledFunction {
    let output = compile_program("script.frst", source, options)
        .unwrap_or_else(|errors| panic!("{source:?} compiles:\n{}", errors.render_plain()));
    let closure = output
        .code
        .into_closure()
        .expect("the script captures nothing");
    CompiledFunction::clone(closure_function(&closure))
}

/// Each script's tree under each optimization permutation, with debug info.
fn every_tree() -> impl Iterator<Item = (&'static str, OptimizationOptions, CompiledFunction)> {
    SCRIPTS.iter().flat_map(|&source| {
        optimization_permutations().map(move |optimization| {
            let options = CompilerOptions::new()
                .with_optimization(optimization)
                .with_debug_info(true);
            (source, optimization, compile_tree(source, options))
        })
    })
}

/// Whether `inner` lies within `outer`.
fn within(inner: Span, outer: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[test]
fn every_instruction_has_a_nonempty_span_within_its_lambda_under_every_option() {
    for (source, optimization, tree) in every_tree() {
        let whole = Span {
            start: 0,
            end: u32::try_from(source.len()).unwrap(),
        };
        // Each function with the span its code must lie within: the whole
        // script for the top level, its lambda for a nested function.
        let mut pending = vec![(&tree, whole)];
        while let Some((function, bound)) = pending.pop() {
            let ops = &function
                .debug_info
                .as_ref()
                .unwrap_or_else(|| {
                    panic!(
                        "{} should have debug info under {optimization:?}",
                        function.name
                    )
                })
                .ops;
            assert_eq!(
                ops.len(),
                function.code.len(),
                "{}: one entry per instruction under {optimization:?}",
                function.name
            );
            for (op, info) in function.code.iter().zip(ops) {
                assert!(
                    info.span.start < info.span.end && within(info.span, bound),
                    "{}: {op:?} at {:?} should be a nonempty span within {bound:?} \
                     under {optimization:?}",
                    function.name,
                    info.span
                );
                if let CreateClosure { child, .. } = *op {
                    pending.push((&function.child_fns[child as usize], info.span));
                }
            }
        }
    }
}

#[test]
fn every_height_follows_from_the_last_under_every_option() {
    for (_, optimization, tree) in every_tree() {
        for function in functions(&tree) {
            let heights: Vec<Option<u32>> = function
                .debug_info
                .as_ref()
                .expect("debug info is on")
                .ops
                .iter()
                .map(|info| info.stack_height)
                .collect();
            assert_eq!(
                heights.first().copied().flatten(),
                Some(entry_height(function.arity)),
                "{}: the first instruction starts at the callee and its arguments, \
                 under {optimization:?}",
                function.name
            );
            for (pc, pair) in function.code.windows(2).enumerate() {
                let [op, next] = [pair[0], pair[1]];
                let goes_on = !matches!(op, Jump(_) | ProduceError);
                match (heights[pc], heights[pc + 1]) {
                    (Some(height), Some(next_height)) if goes_on && op != RewindToMark => {
                        let effect = op.stack_effect().expect("only a rewind has none");
                        assert_eq!(
                            effect.apply(u64::from(height)),
                            Some(u64::from(next_height)),
                            "{}: {next:?} at {} follows from {op:?} at height {height} \
                             under {optimization:?}",
                            function.name,
                            pc + 1
                        );
                    }
                    (before, None) => assert!(
                        next == RewindToMark || !goes_on || before.is_none(),
                        "{}: only a rewind, or code nothing falls into, may have no height, \
                         but {next:?} at {} has none under {optimization:?}",
                        function.name,
                        pc + 1
                    ),
                    _ => {}
                }
            }
        }
    }
}

/// The height on entry to a function of `arity`: the callee and its arguments.
fn entry_height(arity: Arity) -> u32 {
    match arity {
        Arity::Exact(params) => u32::try_from(params).unwrap() + 1,
        Arity::AtLeast(params) => u32::try_from(params).unwrap() + 2,
        Arity::Between(..) => unreachable!("the compiler makes no Between function"),
    }
}

#[test]
fn without_debug_info_no_function_has_any_and_the_code_is_the_same_under_every_option() {
    for source in SCRIPTS {
        for optimization in optimization_permutations() {
            let options = CompilerOptions::new().with_optimization(optimization);
            let with = compile_tree(source, options.with_debug_info(true));
            let without = compile_tree(source, options.with_debug_info(false));
            assert!(
                functions(&without)
                    .iter()
                    .all(|function| function.debug_info.is_none()),
                "{source:?} under {optimization:?}: no function has debug info"
            );
            assert_eq!(
                serde_json::to_value(&without).unwrap(),
                serde_json::to_value(with.strip()).unwrap(),
                "{source:?} under {optimization:?}: the code is the same, debug info aside"
            );
        }
    }
}
